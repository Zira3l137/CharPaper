use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::effect_stack;
use bevy::post_process::effect_stack::ChromaticAberration;
use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use bevy::render::view::ColorGradingGlobal;
use bevy::render::view::ColorGradingSection;
use charpaper_suite::GradingSection;
use charpaper_suite::Look;
use charpaper_suite::Tonemapping as SuiteTonemapping;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::environment::EnvironmentReady;
use crate::environment::ShownEnvironment;
use crate::environment::spawn_environment_scenes;
use crate::render::SceneImage;
use crate::render::SceneTarget;

// Applies the look, the settings the viewer can change while the app runs, to the camera
// and the environment's lights. Recomputed whole on every change, so no value is kept twice.
pub(crate) struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LookBackup>().add_message::<RestoreLook>().add_systems(
            Update,
            (apply_look, apply_grain)
                .run_if(look_needs_applying)
                .in_set(SceneSet::Run)
                .after(spawn_environment_scenes),
        );
    }
}

pub const DEFAULT_TONEMAPPING: SuiteTonemapping = SuiteTonemapping::TonyMcMapface;
pub const DEFAULT_EXPOSURE: f32 = 0.0;
pub const DEFAULT_BLOOM: f32 = 0.0;
pub const DEFAULT_CHROMATIC_ABERRATION: f32 = 0.0;
pub const DEFAULT_BRIGHTNESS: f32 = 1000.0;
pub const DEFAULT_SHADOWS: bool = true;

#[derive(Resource, Deref, DerefMut, Clone, Debug, Default)]
pub struct ActiveLook(pub Look);

#[derive(Resource, Default, Debug)]
pub struct LookBackup {
    pub exists: bool,
}

#[derive(Message, Debug)]
pub struct RestoreLook;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    pub tonemapping: SuiteTonemapping,
    pub bloom: f32,
    pub chromatic_aberration: f32,
    pub vignette: f32,
    pub vignette_size: f32,
    pub grain: f32,
    pub grain_size: f32,
    pub warmth: f32,
    pub tint: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub brightness: f32,
    pub shadows: bool,
    pub exposure: f32,
}

impl Resolved {
    pub fn new(look: &Look, environment: Option<&str>) -> Self {
        let entry = environment.map(|name| look.environment(name)).unwrap_or_default();
        Self {
            tonemapping: look.post.tonemapping.unwrap_or(DEFAULT_TONEMAPPING),
            bloom: look.post.bloom.unwrap_or(DEFAULT_BLOOM),
            chromatic_aberration: look
                .post
                .chromatic_aberration
                .unwrap_or(DEFAULT_CHROMATIC_ABERRATION),
            vignette: look.post.vignette.intensity(),
            vignette_size: look.post.vignette.size(),
            grain: look.post.grain.intensity(),
            grain_size: look.post.grain.size(),
            warmth: look.post.grading.warmth(),
            tint: look.post.grading.tint(),
            saturation: look.post.grading.saturation(),
            contrast: look.post.grading.contrast(),
            brightness: entry.brightness.unwrap_or(DEFAULT_BRIGHTNESS),
            shadows: entry.shadows.unwrap_or(DEFAULT_SHADOWS),
            exposure: entry.exposure.or(look.post.exposure).unwrap_or(DEFAULT_EXPOSURE),
        }
    }
}

pub(crate) fn apply_look(
    mut commands: Commands,
    look: Option<Res<ActiveLook>>,
    shown: Res<ShownEnvironment>,
    mut camera: Query<
        (Entity, Option<&mut Skybox>, Option<&mut EnvironmentMapLight>),
        With<SceneCamera>,
    >,
    children: Query<&Children>,
    mut lights: Query<AnyOf<(&mut DirectionalLight, &mut PointLight, &mut SpotLight)>>,
) {
    let look = look.map(|l| l.0.clone()).unwrap_or_default();
    let resolved = Resolved::new(&look, shown.name.as_deref());

    if let Ok((entity, skybox, environment_light)) = camera.single_mut() {
        // A higher EV100 is darker; a higher look exposure is brighter.
        let ev100 = Exposure::EV100_BLENDER - resolved.exposure;
        commands.entity(entity).insert((to_bevy(resolved.tonemapping), Exposure { ev100 }));
        // Only when asked for: bloom costs an HDR target and extra passes, all day long.
        if resolved.bloom > 0.0 {
            commands.entity(entity).insert(Bloom { intensity: resolved.bloom, ..Bloom::NATURAL });
        } else {
            commands.entity(entity).remove::<Bloom>();
        }
        if resolved.chromatic_aberration > 0.0 {
            let intensity = resolved.chromatic_aberration;
            commands.entity(entity).insert(ChromaticAberration { intensity, ..default() });
        } else {
            commands.entity(entity).remove::<ChromaticAberration>();
        }
        let vignette = &look.post.vignette;
        if vignette.intensity() > 0.0 {
            let [r, g, b] = vignette.color();
            commands.entity(entity).insert(effect_stack::Vignette {
                intensity: vignette.intensity(),
                radius: vignette.size(),
                smoothness: vignette.falloff(),
                roundness: vignette.roundness(),
                color: Color::srgb(r, g, b),
                ..default()
            });
        } else {
            commands.entity(entity).remove::<effect_stack::Vignette>();
        }
        commands.entity(entity).insert(color_grading(&look));
        if let Some(mut skybox) = skybox {
            skybox.brightness = resolved.brightness;
        }
        if let Some(mut environment_light) = environment_light {
            environment_light.intensity = resolved.brightness;
        }
    }

    // glTF can't say whether a light casts shadows, so the look decides for all of them.
    let Some(root) = shown.root else {
        return;
    };
    for entity in children.iter_descendants(root) {
        let Ok((directional, point, spot)) = lights.get_mut(entity) else {
            continue;
        };
        if let Some(mut light) = directional {
            light.shadow_maps_enabled = resolved.shadows;
            light.contact_shadows_enabled = resolved.shadows;
        }
        if let Some(mut light) = point {
            light.shadow_maps_enabled = resolved.shadows;
            light.contact_shadows_enabled = resolved.shadows;
        }
        if let Some(mut light) = spot {
            light.shadow_maps_enabled = resolved.shadows;
            light.contact_shadows_enabled = resolved.shadows;
        }
    }
}

// Always present, and neutral when the look has no grading.
fn color_grading(look: &Look) -> ColorGrading {
    let grading = &look.post.grading;
    let [temperature, tint] = grading.white_balance();
    let [start, end] = grading.midtones_range();
    let section = |section: &GradingSection| ColorGradingSection {
        saturation: section.saturation(),
        contrast: section.contrast() * grading.contrast(),
        gamma: section.gamma(),
        gain: section.gain(),
        lift: section.lift(),
    };
    ColorGrading {
        global: ColorGradingGlobal {
            temperature,
            tint,
            hue: grading.hue_deg().to_radians(),
            post_saturation: grading.saturation(),
            midtones_range: start..end,
            ..default()
        },
        shadows: section(&grading.shadows),
        midtones: section(&grading.midtones),
        highlights: section(&grading.highlights),
    }
}

pub(crate) fn apply_grain(
    look: Option<Res<ActiveLook>>,
    target: Option<Res<SceneTarget>>,
    mut materials: ResMut<Assets<SceneImage>>,
) {
    let Some(target) = target else {
        return;
    };
    let grain = look.map(|l| l.post.grain.clone()).unwrap_or_default();
    let colored = if grain.colored() { 1.0 } else { 0.0 };
    let wanted = Vec4::new(grain.intensity(), grain.size(), colored, 0.0);
    // Only on a real change: every change rebuilds the material on the GPU.
    if materials.get(&target.material).is_some_and(|m| m.grain != wanted)
        && let Some(mut material) = materials.get_mut(&target.material)
    {
        material.grain = wanted;
    }
}

pub(crate) fn look_needs_applying(
    look: Option<Res<ActiveLook>>,
    shown: Res<ShownEnvironment>,
    ready: Query<(), Added<EnvironmentReady>>,
) -> bool {
    look.is_some_and(|l| l.is_changed()) || shown.is_changed() || !ready.is_empty()
}

fn to_bevy(from: SuiteTonemapping) -> Tonemapping {
    match from {
        SuiteTonemapping::None => Tonemapping::None,
        SuiteTonemapping::Reinhard => Tonemapping::Reinhard,
        SuiteTonemapping::ReinhardLuminance => Tonemapping::ReinhardLuminance,
        SuiteTonemapping::AcesFitted => Tonemapping::AcesFitted,
        SuiteTonemapping::Agx => Tonemapping::AgX,
        SuiteTonemapping::SomewhatBoringDisplayTransform => {
            Tonemapping::SomewhatBoringDisplayTransform
        }
        SuiteTonemapping::TonyMcMapface => Tonemapping::TonyMcMapface,
        SuiteTonemapping::BlenderFilmic => Tonemapping::BlenderFilmic,
    }
}
