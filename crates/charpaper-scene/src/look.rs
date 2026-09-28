use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use charpaper_suite::Look;
use charpaper_suite::Tonemapping as SuiteTonemapping;

use crate::OrbitCamera;
use crate::environment::EnvironmentReady;
use crate::environment::ShownEnvironment;

pub const DEFAULT_TONEMAPPING: SuiteTonemapping = SuiteTonemapping::TonyMcMapface;
pub const DEFAULT_EXPOSURE: f32 = 0.0;
pub const DEFAULT_BLOOM: f32 = 0.0;
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
        With<OrbitCamera>,
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
