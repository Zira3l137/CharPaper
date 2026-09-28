use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;
use charpaper_scene::LookBackup;
use charpaper_scene::Resolved;
use charpaper_scene::RestoreLook;
use charpaper_suite::Tonemapping;

use crate::Tab;
use crate::UiConfig;
use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::locale::UiLocale;
use crate::widgets::*;

// cd/m², spaced so each step looks about as big as the last.
const BRIGHTNESS_STOPS: [f32; 15] = [
    50.0, 100.0, 200.0, 300.0, 500.0, 700.0, 1000.0, 1500.0, 2000.0, 3000.0, 5000.0, 8000.0,
    12000.0, 20000.0, 30000.0,
];
const EXPOSURE_STEP: f32 = 0.5;
const EXPOSURE_LIMIT: f32 = 10.0;
const BLOOM_STEP: f32 = 0.05;
const BLOOM_LIMIT: f32 = 1.0;

// Which camera and environment, and how the picture is finished. Look edits go to
// ActiveLook, which the app writes back into the suite's suite.toml.
pub(crate) struct SceneTabPlugin;

impl Plugin for SceneTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            (
                show_tab.run_if(resource_added::<ActiveSuite>),
                // Baking new maps changes the suite, which can make the Brightness row relevant.
                show_rows.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_exists_and_changed::<ActiveSuite>),
                ),
                show_values.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_exists_and_changed::<ActiveLook>),
                ),
                show_restore.run_if(resource_changed::<LookBackup>),
            )
                .chain(),
        );
    }
}

pub(crate) fn page(locale: &UiLocale) -> impl Bundle {
    children![
        section(
            locale.get_or("section.camera", "CAMERA"),
            Section::Camera,
            cycler(locale.get_or("label.camera", "Camera"), Cycler::Camera),
        ),
        section(
            locale.get_or("section.environment", "ENVIRONMENT"),
            Section::Environment,
            rows(children![
                cycler(locale.get_or("label.environment", "Environment"), Cycler::Environment),
                cycler(locale.get_or("label.brightness", "Brightness"), Cycler::Brightness),
                cycler(locale.get_or("label.shadows", "Shadows"), Cycler::Shadows),
            ]),
        ),
        section(
            locale.get_or("section.image", "IMAGE"),
            Section::Image,
            rows(children![
                cycler(locale.get_or("label.tonemapping", "Tonemapping"), Cycler::Tonemapping),
                cycler(locale.get_or("label.exposure", "Exposure"), Cycler::Exposure),
                cycler(locale.get_or("label.bloom", "Bloom"), Cycler::Bloom),
            ]),
        ),
        button(locale.get_or("restore_look", "Restore defaults"))
            .node(|n| n.display = Display::None)
            .build(UiButton::RestoreLook),
    ]
}

// With a suite there is always something here: at least the image section.
fn show_tab(suite: Res<ActiveSuite>, nodes: Query<(&mut Node, AnyOf<(&UiContainer, &UiButton)>)>) {
    for (mut node, element) in nodes {
        let shown = match element {
            (Some(UiContainer::Section(Section::Camera)), _) => !suite.cameras.is_empty(),
            (Some(UiContainer::Section(Section::Environment)), _) => !suite.environments.is_empty(),
            (Some(UiContainer::Section(Section::Image)), _)
            | (_, Some(UiButton::Tab(Tab::Scene))) => true,
            _ => continue,
        };
        node.display = display(shown);
    }
}

// A row only shows when it would do something for the environment on screen.
fn show_rows(
    suite: Option<Res<ActiveSuite>>,
    state: Res<CharacterState>,
    mut nodes: Query<(&UiContainer, &mut Node)>,
) {
    let Some(suite) = suite else {
        return;
    };
    let environment = state
        .environment
        .as_ref()
        .and_then(|name| suite.environments.iter().find(|e| &e.name == name));
    let lit = environment.is_some_and(|e| e.sky().is_some() || e.reflections().is_some());
    let scene = environment.is_some_and(|e| e.scene.is_some());

    let several = suite.environments.len() > 1;
    show_container(&mut nodes, UiContainer::Row(Cycler::Environment), several);
    show_container(&mut nodes, UiContainer::Row(Cycler::Brightness), lit);
    show_container(&mut nodes, UiContainer::Row(Cycler::Shadows), scene);
}

fn show_restore(backup: Res<LookBackup>, buttons: Query<(&UiButton, &mut Node)>) {
    for (button, mut node) in buttons {
        if *button == UiButton::RestoreLook {
            node.display = display(backup.exists);
        }
    }
}

fn show_values(
    state: Res<CharacterState>,
    look: Option<Res<ActiveLook>>,
    config: Res<UiConfig>,
    values: Query<(&CyclerValue, &mut Text)>,
) {
    let locale = &config.locale;
    let look = look.map(|l| l.0.clone()).unwrap_or_default();
    let resolved = Resolved::new(&look, state.environment.as_deref());
    for (value, mut text) in values {
        text.0 = match value.0 {
            Cycler::Camera => state
                .camera
                .clone()
                .unwrap_or_else(|| locale.get_or("camera.orbit", "Orbit").into()),
            Cycler::Environment => state.environment.clone().unwrap_or_else(|| "-".into()),
            Cycler::Brightness => format!("{:.0}", resolved.brightness),
            Cycler::Shadows => on_off(locale, resolved.shadows).into(),
            Cycler::Tonemapping => tonemapping_name(resolved.tonemapping).into(),
            Cycler::Exposure => format!("{:+.1} EV", resolved.exposure),
            Cycler::Bloom if resolved.bloom <= 0.0 => on_off(locale, false).into(),
            Cycler::Bloom => format!("{:.2}", resolved.bloom),
            _ => continue,
        };
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    suite: Option<Res<ActiveSuite>>,
    look: Option<ResMut<ActiveLook>>,
    mut state: ResMut<CharacterState>,
    mut restore: MessageWriter<RestoreLook>,
) {
    let (Ok(button), Some(suite)) = (buttons.get(event.entity), suite) else {
        return;
    };
    if *button == UiButton::RestoreLook {
        restore.write(RestoreLook);
        return;
    }
    let Some((cycler, forward)) = button.step() else {
        return;
    };

    match cycler {
        Cycler::Camera => {
            // None is the orbit camera, which every suite has.
            let options: Vec<Option<String>> = std::iter::once(None)
                .chain(suite.cameras.iter().map(|c| Some(c.name.clone())))
                .collect();
            if let Some(next) = step(&options, &state.camera, forward) {
                state.camera = next;
            }
        }
        Cycler::Environment => {
            let options: Vec<Option<String>> =
                suite.environments.iter().map(|e| Some(e.name.clone())).collect();
            if let Some(next) = step(&options, &state.environment, forward) {
                state.environment = next;
            }
        }
        Cycler::Brightness
        | Cycler::Shadows
        | Cycler::Tonemapping
        | Cycler::Exposure
        | Cycler::Bloom => {
            if let Some(mut look) = look {
                edit_look(&mut look, state.environment.as_deref(), cycler, forward);
            }
        }
        _ => {}
    }
}

// Environment settings are kept per environment, so they only change while one shows.
// Exposure falls back to the suite-wide value without one.
fn edit_look(look: &mut ActiveLook, environment: Option<&str>, cycler: Cycler, forward: bool) {
    let resolved = Resolved::new(look, environment);
    let sign = if forward { 1.0 } else { -1.0 };
    match cycler {
        Cycler::Tonemapping => {
            look.post.tonemapping = step(&Tonemapping::ALL, &resolved.tonemapping, forward);
        }
        Cycler::Bloom => {
            let bloom = (resolved.bloom + sign * BLOOM_STEP).clamp(0.0, BLOOM_LIMIT);
            look.post.bloom = Some(round(bloom, 100.0));
        }
        Cycler::Exposure => {
            let exposure =
                (resolved.exposure + sign * EXPOSURE_STEP).clamp(-EXPOSURE_LIMIT, EXPOSURE_LIMIT);
            let exposure = Some(round(exposure, 10.0));
            match environment {
                Some(name) => look.environments.entry(name.into()).or_default().exposure = exposure,
                None => look.post.exposure = exposure,
            }
        }
        Cycler::Brightness => {
            if let Some(name) = environment {
                let brightness = step_stops(&BRIGHTNESS_STOPS, resolved.brightness, forward);
                look.environments.entry(name.into()).or_default().brightness = Some(brightness);
            }
        }
        Cycler::Shadows => {
            if let Some(name) = environment {
                look.environments.entry(name.into()).or_default().shadows = Some(!resolved.shadows);
            }
        }
        _ => {}
    }
}

// A value between stops, like one typed into suite.toml, moves to the next stop.
fn step_stops(stops: &[f32], current: f32, forward: bool) -> f32 {
    let next = if forward {
        stops.iter().copied().find(|&s| s > current * 1.001)
    } else {
        stops.iter().rev().copied().find(|&s| s < current * 0.999)
    };
    next.unwrap_or(current)
}

// Keeps repeated steps from drifting into values like 0.15000001.
fn round(value: f32, per_unit: f32) -> f32 {
    (value * per_unit).round() / per_unit
}

fn tonemapping_name(tonemapping: Tonemapping) -> &'static str {
    match tonemapping {
        Tonemapping::None => "None",
        Tonemapping::Reinhard => "Reinhard",
        Tonemapping::ReinhardLuminance => "Reinhard lum.",
        Tonemapping::AcesFitted => "ACES",
        Tonemapping::Agx => "AgX",
        Tonemapping::SomewhatBoringDisplayTransform => "Boring",
        Tonemapping::TonyMcMapface => "TonyMcMapface",
        Tonemapping::BlenderFilmic => "Filmic",
    }
}
