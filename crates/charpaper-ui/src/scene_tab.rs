//! The Scene tab: which camera looks at the character, which environment
//! surrounds it, and how the picture is finished.
//!
//! The look controls edit the scene's `ActiveLook`, which the app writes into
//! the suite's `suite.toml`. Environment settings go to the environment on
//! screen, so each one keeps its own brightness, shadows and exposure.

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
use crate::config::UiLocale;
use crate::helpers::Cycler;
use crate::helpers::CyclerValue;
use crate::helpers::Section;
use crate::helpers::UiButton;
use crate::helpers::UiContainer;
use crate::helpers::UiElement;
use crate::helpers::UiNode;
use crate::pages::step;
use crate::widgets::*;

/// Brightness steps in cd/m², from a dim room to a bright sky, spaced so each
/// step looks about as big as the last.
const BRIGHTNESS_STOPS: [f32; 15] = [
    50.0, 100.0, 200.0, 300.0, 500.0, 700.0, 1000.0, 1500.0, 2000.0, 3000.0, 5000.0, 8000.0,
    12000.0, 20000.0, 30000.0,
];
const EXPOSURE_STEP: f32 = 0.5;
const EXPOSURE_LIMIT: f32 = 10.0;
const BLOOM_STEP: f32 = 0.05;
const BLOOM_LIMIT: f32 = 1.0;

pub(crate) fn scene_page(locale: &UiLocale) -> impl Bundle {
    let text = |key: &str, english: &'static str| locale.get_or(key, english).to_string();
    children![
        section(
            locale.get_or("section.camera", "CAMERA"),
            UiContainer::Section(Section::Camera),
            cycler(text("label.camera", "Camera"), Cycler::Camera),
        ),
        section(
            locale.get_or("section.environment", "ENVIRONMENT"),
            UiContainer::Section(Section::Environment),
            rows(children![
                cycler(text("label.environment", "Environment"), Cycler::Environment),
                cycler(text("label.brightness", "Brightness"), Cycler::Brightness),
                cycler(text("label.shadows", "Shadows"), Cycler::Shadows),
            ]),
        ),
        section(
            locale.get_or("section.image", "IMAGE"),
            UiContainer::Section(Section::Image),
            rows(children![
                cycler(text("label.tonemapping", "Tonemapping"), Cycler::Tonemapping),
                cycler(text("label.exposure", "Exposure"), Cycler::Exposure),
                cycler(text("label.bloom", "Bloom"), Cycler::Bloom),
            ]),
        ),
        button(locale.get_or("restore_look", "Restore defaults"))
            .edit_node(|n| n.display = Display::None)
            .build_with(UiElement::Button(UiButton::RestoreLook)),
    ]
}

fn rows(content: impl Bundle) -> impl Bundle {
    (
        Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
        Pickable::IGNORE,
        content,
    )
}

/// With a suite there is always something here: at least the image section.
pub(crate) fn show_scene_tab(suite: Res<ActiveSuite>, nodes: Query<(&UiElement, &mut Node)>) {
    for (element, mut node) in nodes {
        let shown = match element {
            UiElement::Container(UiContainer::Section(Section::Camera)) => {
                !suite.cameras.is_empty()
            }
            UiElement::Container(UiContainer::Section(Section::Environment)) => {
                !suite.environments.is_empty()
            }
            UiElement::Container(UiContainer::Section(Section::Image))
            | UiElement::Container(UiContainer::TabBar)
            | UiElement::Button(UiButton::Tab(Tab::Scene)) => true,
            _ => continue,
        };
        node.display = if shown { Display::Flex } else { Display::None };
    }
}

/// Each environment row only shows when it would do something for the
/// environment on screen: brightness needs a sky or reflections to scale,
/// shadows need a scene with lights. A cycler over one environment has
/// nothing to pick.
pub(crate) fn show_rows(
    suite: Option<Res<ActiveSuite>>,
    state: Res<CharacterState>,
    nodes: Query<(&UiElement, &mut Node)>,
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

    for (element, mut node) in nodes {
        let shown = match element {
            UiElement::Container(UiContainer::Row(Cycler::Environment)) => {
                suite.environments.len() > 1
            }
            UiElement::Container(UiContainer::Row(Cycler::Brightness)) => lit,
            UiElement::Container(UiContainer::Row(Cycler::Shadows)) => scene,
            _ => continue,
        };
        node.display = if shown { Display::Flex } else { Display::None };
    }
}

pub(crate) fn show_restore(backup: Res<LookBackup>, nodes: Query<(&UiElement, &mut Node)>) {
    for (element, mut node) in nodes {
        if *element == UiElement::Button(UiButton::RestoreLook) {
            node.display = if backup.exists { Display::Flex } else { Display::None };
        }
    }
}

pub(crate) fn show_scene_values(
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
            Cycler::Animation => continue,
        };
    }
}

pub(crate) fn on_scene_tab_click(
    event: On<Pointer<Click>>,
    elements: Query<&UiElement>,
    suite: Option<Res<ActiveSuite>>,
    look: Option<ResMut<ActiveLook>>,
    mut state: ResMut<CharacterState>,
    mut restore: MessageWriter<RestoreLook>,
) {
    let (Ok(UiElement::Button(button)), Some(suite)) = (elements.get(event.entity), suite) else {
        return;
    };
    let (cycler, forward) = match button {
        UiButton::Previous(cycler) => (*cycler, false),
        UiButton::Next(cycler) => (*cycler, true),
        UiButton::RestoreLook => {
            restore.write(RestoreLook);
            return;
        }
        _ => return,
    };

    match cycler {
        Cycler::Camera => {
            // `None` is the orbit camera, which every suite has.
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
        Cycler::Animation => {}
        look_cycler => {
            if let Some(mut look) = look {
                edit_look(&mut look, state.environment.as_deref(), look_cycler, forward);
            }
        }
    }
}

/// Environment settings are kept per environment, so they are only edited
/// while one is shown. Exposure is the exception: without an environment it
/// falls back to the suite-wide value, so that is the one it edits.
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
        Cycler::Animation | Cycler::Camera | Cycler::Environment => {}
    }
}

/// The next stop past `current` in `forward` direction, stopping at the ends.
/// A value between stops, such as one typed into `suite.toml`, moves to the
/// neighbouring stop rather than by a fixed amount.
fn step_stops(stops: &[f32], current: f32, forward: bool) -> f32 {
    let next = if forward {
        stops.iter().copied().find(|&s| s > current * 1.001)
    } else {
        stops.iter().rev().copied().find(|&s| s < current * 0.999)
    };
    next.unwrap_or(current)
}

/// Keeps repeated steps from drifting into values like `0.15000001`.
fn round(value: f32, per_unit: f32) -> f32 {
    (value * per_unit).round() / per_unit
}

fn on_off(locale: &UiLocale, on: bool) -> &str {
    if on { locale.get_or("value.on", "On") } else { locale.get_or("value.off", "Off") }
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
