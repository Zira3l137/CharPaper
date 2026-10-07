use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;
use charpaper_scene::LookBackup;
use charpaper_scene::Resolved;
use charpaper_scene::RestoreLook;
use charpaper_suite::NO_LUT;
use charpaper_suite::Tonemapping;
use i18n_embed_fl::fl;

use crate::Tab;
use crate::elements::Cycler;
use crate::elements::Section;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::held::Held;
use crate::locale::Locale;
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
const ABERRATION_STEP: f32 = 0.0025;
const ABERRATION_LIMIT: f32 = 0.05;
const VIGNETTE_STEP: f32 = 0.05;
const VIGNETTE_SIZE_STEP: f32 = 0.1;
const VIGNETTE_SIZES: (f32, f32) = (0.3, 2.0);
const GRAIN_STEP: f32 = 0.01;
const GRAIN_LIMIT: f32 = 0.2;
const GRAIN_SIZE_STEP: f32 = 0.25;
const GRAIN_SIZES: (f32, f32) = (1.0, 4.0);
const GRADING_STEP: f32 = 0.05;
const SATURATIONS: (f32, f32) = (0.0, 2.0);
const CONTRASTS: (f32, f32) = (0.5, 1.5);

// Which camera and environment, and how the picture is finished. Look edits go to
// ActiveLook, which the app writes back into the suite's suite.toml.
pub(crate) struct SceneTabPlugin;

impl Plugin for SceneTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_observer(on_step).add_systems(
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
                        .or_eager(resource_exists_and_changed::<ActiveLook>)
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(resource_changed::<Held>)
                        .or_eager(resource_exists_and_changed::<ActiveSuite>),
                ),
                show_restore.run_if(resource_changed::<LookBackup>),
            )
                .chain(),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![
        section(
            |l| fl!(l, "section-camera"),
            Section::Camera,
            cycler(|l| fl!(l, "label-camera"), Cycler::Camera),
        ),
        section(
            |l| fl!(l, "section-environment"),
            Section::Environment,
            rows(children![
                cycler(|l| fl!(l, "label-environment"), Cycler::Environment),
                cycler(|l| fl!(l, "label-brightness"), Cycler::Brightness),
                cycler(|l| fl!(l, "label-shadows"), Cycler::Shadows),
            ]),
        ),
        section(
            |l| fl!(l, "section-image"),
            Section::Image,
            rows(children![
                cycler(|l| fl!(l, "label-tonemapping"), Cycler::Tonemapping),
                cycler(|l| fl!(l, "label-exposure"), Cycler::Exposure),
                cycler(|l| fl!(l, "label-bloom"), Cycler::Bloom),
                cycler(|l| fl!(l, "label-chromatic-aberration"), Cycler::ChromaticAberration),
            ]),
        ),
        section(
            |l| fl!(l, "section-color"),
            Section::Color,
            rows(children![
                cycler(|l| fl!(l, "label-warmth"), Cycler::Warmth),
                cycler(|l| fl!(l, "label-tint"), Cycler::Tint),
                cycler(|l| fl!(l, "label-saturation"), Cycler::Saturation),
                cycler(|l| fl!(l, "label-contrast"), Cycler::Contrast),
                cycler(|l| fl!(l, "label-lut"), Cycler::Lut),
                cycler(|l| fl!(l, "label-lut-strength"), Cycler::LutStrength),
            ]),
        ),
        section(
            |l| fl!(l, "section-film"),
            Section::Film,
            rows(children![
                cycler(|l| fl!(l, "label-vignette"), Cycler::Vignette),
                cycler(|l| fl!(l, "label-vignette-size"), Cycler::VignetteSize),
                cycler(|l| fl!(l, "label-grain"), Cycler::Grain),
                cycler(|l| fl!(l, "label-grain-size"), Cycler::GrainSize),
            ]),
        ),
        translated_button(|l| fl!(l, "restore-look"))
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
            (Some(UiContainer::Section(Section::Image | Section::Color | Section::Film)), _)
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
    let luts = !suite.luts.is_empty();
    show_container(&mut nodes, UiContainer::Row(Cycler::Lut), luts);
    show_container(&mut nodes, UiContainer::Row(Cycler::LutStrength), luts);
}

fn show_restore(backup: Res<LookBackup>, buttons: Query<(&UiButton, &mut Node)>) {
    for (button, mut node) in buttons {
        if *button == UiButton::RestoreLook {
            node.display = display(backup.exists);
        }
    }
}

fn show_values(
    suite: Option<Res<ActiveSuite>>,
    state: Res<CharacterState>,
    held: Res<Held>,
    look: Option<Res<ActiveLook>>,
    locale: Res<Locale>,
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    let Some(suite) = suite else {
        return;
    };
    let look = look.map(|l| l.0.clone()).unwrap_or_default();
    let resolved = Resolved::new(&look, state.environment.as_deref());
    let orbit = fl!(locale, "camera-orbit");
    let off = on_off(&locale, false);
    // Intensities read as a share of the strongest the row allows, Off at none.
    let strength = |value: f32, max: f32| {
        let text = if value <= 0.0 { off.clone() } else { percent(value / max) };
        number(text, value, 0.0, max)
    };
    for (container, mut shown) in &mut rows {
        let UiContainer::Row(cycler) = *container else {
            continue;
        };
        let next = match cycler {
            Cycler::Camera => {
                let camera = held.current(Cycler::Camera, &state.camera);
                choice(&camera_options(&suite), &camera, |camera| {
                    camera.clone().unwrap_or_else(|| orbit.clone())
                })
            }
            Cycler::Environment => {
                let environment = held.current(Cycler::Environment, &state.environment);
                choice(&environment_options(&suite), &environment, name)
            }
            // The stops look evenly spaced, so the bar follows them on a log scale too.
            Cycler::Brightness => {
                let (min, max) =
                    (BRIGHTNESS_STOPS[0], BRIGHTNESS_STOPS[BRIGHTNESS_STOPS.len() - 1]);
                let text = format!("{:.0}", resolved.brightness);
                number(text, resolved.brightness.ln(), min.ln(), max.ln())
            }
            Cycler::Shadows => switch(&locale, resolved.shadows),
            Cycler::Tonemapping => {
                choice(&Tonemapping::ALL, &resolved.tonemapping, |t| tonemapping_name(*t).into())
            }
            Cycler::Exposure => {
                let text = format!("{:+.1} EV", resolved.exposure);
                balance(text, resolved.exposure, -EXPOSURE_LIMIT, EXPOSURE_LIMIT)
            }
            Cycler::Bloom => strength(resolved.bloom, BLOOM_LIMIT),
            Cycler::ChromaticAberration => {
                strength(resolved.chromatic_aberration, ABERRATION_LIMIT)
            }
            Cycler::Vignette => strength(resolved.vignette, 1.0),
            Cycler::VignetteSize => {
                let (min, max) = VIGNETTE_SIZES;
                number(format!("{:.1}", resolved.vignette_size), resolved.vignette_size, min, max)
            }
            Cycler::Grain => strength(resolved.grain, GRAIN_LIMIT),
            Cycler::GrainSize => {
                let (min, max) = GRAIN_SIZES;
                number(format!("{:.2} px", resolved.grain_size), resolved.grain_size, min, max)
            }
            Cycler::Warmth => balance(signed_percent(resolved.warmth), resolved.warmth, -1.0, 1.0),
            Cycler::Tint => balance(signed_percent(resolved.tint), resolved.tint, -1.0, 1.0),
            // 100% leaves the picture as it is, which sits in the middle of both ranges.
            Cycler::Saturation => {
                let (min, max) = SATURATIONS;
                balance(percent(resolved.saturation), resolved.saturation, min, max)
            }
            Cycler::Contrast => {
                let (min, max) = CONTRASTS;
                balance(percent(resolved.contrast), resolved.contrast, min, max)
            }
            Cycler::Lut => {
                let lut = look.post.lut.name().map(str::to_string);
                choice(&lut_options(&suite), &lut, |lut| lut.clone().unwrap_or_else(|| off.clone()))
            }
            Cycler::LutStrength => {
                number(percent(resolved.lut_strength), resolved.lut_strength, 0.0, 1.0)
            }
            _ => continue,
        };
        shown.set_if_neq(next);
    }
}

// None is the orbit camera, which every suite has.
fn camera_options(suite: &ActiveSuite) -> Vec<Option<String>> {
    std::iter::once(None).chain(suite.cameras.iter().map(|c| Some(c.name.clone()))).collect()
}

fn environment_options(suite: &ActiveSuite) -> Vec<Option<String>> {
    suite.environments.iter().map(|e| Some(e.name.clone())).collect()
}

// None is no LUT.
fn lut_options(suite: &ActiveSuite) -> Vec<Option<String>> {
    std::iter::once(None).chain(suite.luts.iter().map(|l| Some(l.name.clone()))).collect()
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    mut restore: MessageWriter<RestoreLook>,
) {
    if let Ok(UiButton::RestoreLook) = buttons.get(event.entity) {
        restore.write(RestoreLook);
    }
}

fn on_step(
    event: On<Step>,
    time: Res<Time<Real>>,
    mut held: ResMut<Held>,
    suite: Option<Res<ActiveSuite>>,
    look: Option<ResMut<ActiveLook>>,
    mut state: ResMut<CharacterState>,
) {
    let Some(suite) = suite else {
        return;
    };
    match event.cycler {
        Cycler::Camera => {
            let options = camera_options(&suite);
            let current = held.current(Cycler::Camera, &state.camera);
            if let Some(next) = step(&options, &current, &event) {
                if let Some(choice) = held.hold(&event, next, time.elapsed()) {
                    state.camera = choice;
                }
            }
        }
        Cycler::Environment => {
            let options = environment_options(&suite);
            let current = held.current(Cycler::Environment, &state.environment);
            if let Some(next) = step(&options, &current, &event) {
                if let Some(choice) = held.hold(&event, next, time.elapsed()) {
                    state.environment = choice;
                }
            }
        }
        Cycler::Lut => {
            if let Some(mut look) = look {
                let options = lut_options(&suite);
                let current = look.post.lut.name().map(str::to_string);
                if let Some(next) = step(&options, &current, &event) {
                    look.post.lut.name = Some(next.unwrap_or_else(|| NO_LUT.to_string()));
                }
            }
        }
        Cycler::Brightness
        | Cycler::Shadows
        | Cycler::Tonemapping
        | Cycler::Exposure
        | Cycler::Bloom
        | Cycler::ChromaticAberration
        | Cycler::Vignette
        | Cycler::VignetteSize
        | Cycler::Grain
        | Cycler::GrainSize
        | Cycler::Warmth
        | Cycler::Tint
        | Cycler::Saturation
        | Cycler::Contrast
        | Cycler::LutStrength => {
            if let Some(mut look) = look {
                edit_look(&mut look, state.environment.as_deref(), &event);
            }
        }
        _ => {}
    }
}

// Environment settings are kept per environment, so they only change while one shows.
// Exposure falls back to the suite-wide value without one.
fn edit_look(look: &mut ActiveLook, environment: Option<&str>, event: &Step) {
    let resolved = Resolved::new(look, environment);
    let steps = event.by() as f32;
    match event.cycler {
        Cycler::Tonemapping => {
            look.post.tonemapping = step(&Tonemapping::ALL, &resolved.tonemapping, event);
        }
        Cycler::Bloom => {
            let bloom = (resolved.bloom + steps * BLOOM_STEP).clamp(0.0, BLOOM_LIMIT);
            look.post.bloom = Some(round(bloom, 100.0));
        }
        Cycler::ChromaticAberration => {
            let aberration = (resolved.chromatic_aberration + steps * ABERRATION_STEP)
                .clamp(0.0, ABERRATION_LIMIT);
            look.post.chromatic_aberration = Some(round(aberration, 10_000.0));
        }
        Cycler::Vignette => {
            let vignette = (resolved.vignette + steps * VIGNETTE_STEP).clamp(0.0, 1.0);
            look.post.vignette.intensity = Some(round(vignette, 100.0));
        }
        Cycler::VignetteSize => {
            let (min, max) = VIGNETTE_SIZES;
            let size = (resolved.vignette_size + steps * VIGNETTE_SIZE_STEP).clamp(min, max);
            look.post.vignette.size = Some(round(size, 10.0));
        }
        Cycler::Grain => {
            let grain = (resolved.grain + steps * GRAIN_STEP).clamp(0.0, GRAIN_LIMIT);
            look.post.grain.intensity = Some(round(grain, 100.0));
        }
        Cycler::GrainSize => {
            let (min, max) = GRAIN_SIZES;
            let size = (resolved.grain_size + steps * GRAIN_SIZE_STEP).clamp(min, max);
            look.post.grain.size = Some(round(size, 100.0));
        }
        Cycler::Warmth => {
            let warmth = (resolved.warmth + steps * GRADING_STEP).clamp(-1.0, 1.0);
            look.post.grading.warmth = Some(round(warmth, 100.0));
        }
        Cycler::Tint => {
            let tint = (resolved.tint + steps * GRADING_STEP).clamp(-1.0, 1.0);
            look.post.grading.tint = Some(round(tint, 100.0));
        }
        Cycler::Saturation => {
            let (min, max) = SATURATIONS;
            let saturation = (resolved.saturation + steps * GRADING_STEP).clamp(min, max);
            look.post.grading.saturation = Some(round(saturation, 100.0));
        }
        Cycler::Contrast => {
            let (min, max) = CONTRASTS;
            let contrast = (resolved.contrast + steps * GRADING_STEP).clamp(min, max);
            look.post.grading.contrast = Some(round(contrast, 100.0));
        }
        Cycler::LutStrength => {
            let strength = (resolved.lut_strength + steps * GRADING_STEP).clamp(0.0, 1.0);
            look.post.lut.strength = Some(round(strength, 100.0));
        }
        Cycler::Exposure => {
            let exposure =
                (resolved.exposure + steps * EXPOSURE_STEP).clamp(-EXPOSURE_LIMIT, EXPOSURE_LIMIT);
            let exposure = Some(round(exposure, 10.0));
            match environment {
                Some(name) => look.environments.entry(name.into()).or_default().exposure = exposure,
                None => look.post.exposure = exposure,
            }
        }
        Cycler::Brightness => {
            if let Some(name) = environment {
                let brightness = step_stops(&BRIGHTNESS_STOPS, resolved.brightness, event.by());
                look.environments.entry(name.into()).or_default().brightness = Some(brightness);
            }
        }
        Cycler::Shadows => {
            if let Some(name) = environment {
                let shadows = flip(resolved.shadows, event);
                look.environments.entry(name.into()).or_default().shadows = Some(shadows);
            }
        }
        _ => {}
    }
}

// A value between stops, like one typed into suite.toml, moves to the next stop. Stops at the
// ends rather than going round.
fn step_stops(stops: &[f32], current: f32, by: i32) -> f32 {
    (0..by.unsigned_abs()).fold(current, |value, _| {
        let next = if by > 0 {
            stops.iter().copied().find(|&s| s > value * 1.001)
        } else {
            stops.iter().rev().copied().find(|&s| s < value * 0.999)
        };
        next.unwrap_or(value)
    })
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
