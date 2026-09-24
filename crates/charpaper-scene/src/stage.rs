//! Everything around the character: the environment, lights, the camera and
//! its effects.
//!
//! Each setting starts from an engine default below and is replaced by
//! whatever `suite.toml` says. A later user-override layer slots in between
//! the suite and the value used.

use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::Skybox;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use charpaper_suite::Post;
use charpaper_suite::Tonemapping as SuiteTonemapping;

use crate::OrbitCamera;
use crate::PITCH_LIMIT;
use crate::cameras::Following;
use crate::suite::ActiveSuite;
use crate::suite::asset_path;
use crate::suite::load_scene;
use crate::update_camera_transform;

const SUN_ILLUMINANCE: f32 = light_consts::lux::OVERCAST_DAY;
const SUN_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
const SUN_DIRECTION: [f32; 3] = [-4.0, -8.0, -4.0];
const SUN_SHADOWS: bool = true;
const AMBIENT_COLOR: [f32; 3] = [0.6, 0.7, 1.0];
const AMBIENT_BRIGHTNESS: f32 = 200.0;
const TONEMAPPING: Tonemapping = Tonemapping::TonyMcMapface;
const EXPOSURE_COMPENSATION: f32 = 0.0;
const BLOOM: f32 = 0.0;
/// Roughly the chest of a human-sized character standing at the origin.
const ORBIT_FOCUS: [f32; 3] = [0.0, 1.0, 0.0];
const ORBIT_RADIUS: f32 = 2.0;
const ORBIT_YAW_DEG: f32 = 0.0;
const ORBIT_PITCH_DEG: f32 = 0.0;
const SKYBOX_BRIGHTNESS: f32 = 1000.0;

pub(crate) fn spawn_stage(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
) {
    let lighting = suite.as_ref().map(|s| s.lighting.clone()).unwrap_or_default();
    let post = suite.as_ref().map(|s| s.post.clone()).unwrap_or_default();
    let view = suite.as_ref().map(|s| s.camera.clone()).unwrap_or_default();
    let sun = lighting.sun.unwrap_or_default();
    let ambient = lighting.ambient.unwrap_or_default();

    let shadows = sun.shadows.unwrap_or(SUN_SHADOWS);
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: sun.illuminance.unwrap_or(SUN_ILLUMINANCE),
            color: srgb(sun.color.unwrap_or(SUN_COLOR)),
            shadow_maps_enabled: shadows,
            contact_shadows_enabled: shadows,
            ..default()
        },
        // A directional light shines along its forward axis; `looking_to`
        // copes with a direction parallel to Y or of zero length.
        Transform::default()
            .looking_to(Vec3::from_array(sun.direction.unwrap_or(SUN_DIRECTION)), Vec3::Y),
    ));

    let orbit = OrbitCamera {
        focus: Vec3::from_array(view.focus.unwrap_or(ORBIT_FOCUS)),
        radius: view.radius.unwrap_or(ORBIT_RADIUS),
        yaw: view.yaw_deg.unwrap_or(ORBIT_YAW_DEG).to_radians(),
        // The orbit's own pitch is positive below the focus, so it is flipped
        // to match the manifest's "degrees above the horizon".
        pitch: (-view.pitch_deg.unwrap_or(ORBIT_PITCH_DEG).to_radians())
            .clamp(-PITCH_LIMIT, PITCH_LIMIT),
    };
    let mut transform = Transform::default();
    update_camera_transform(&mut transform, &orbit);

    let mut camera = commands.spawn((
        Camera3d::default(),
        orbit,
        Following::default(),
        transform,
        AmbientLight {
            color: srgb(ambient.color.unwrap_or(AMBIENT_COLOR)),
            brightness: ambient.brightness.unwrap_or(AMBIENT_BRIGHTNESS),
            ..default()
        },
    ));
    apply_post(&mut camera, &post);

    let Some(suite) = suite else {
        return;
    };
    let environment = &suite.environment;
    if let Some(skybox) = &environment.skybox {
        camera.insert(Skybox {
            image: Some(assets.load(asset_path(&suite, skybox))),
            brightness: environment.skybox_brightness.unwrap_or(SKYBOX_BRIGHTNESS),
            ..default()
        });
    }
    if let Some(scene) = &environment.scene {
        commands.spawn((
            Name::new("Environment"),
            WorldAssetRoot(load_scene(&assets, &suite, scene, true)),
        ));
    }
}

/// Bloom is added only when asked for: it pulls in an HDR render target and a
/// few extra passes, which a wallpaper running all day should not pay for by
/// default.
fn apply_post(camera: &mut EntityCommands, post: &Post) {
    let tonemapping = post.tonemapping.map_or(TONEMAPPING, to_bevy);
    // Bevy's exposure is in EV100, where a higher value means a darker image;
    // the suite's is compensation, where higher means brighter.
    let compensation = post.exposure.unwrap_or(EXPOSURE_COMPENSATION);
    camera.insert((tonemapping, Exposure { ev100: Exposure::EV100_BLENDER - compensation }));

    let bloom = post.bloom.unwrap_or(BLOOM);
    if bloom > 0.0 {
        camera.insert(Bloom { intensity: bloom, ..Bloom::NATURAL });
    }
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

fn srgb([r, g, b]: [f32; 3]) -> Color {
    Color::srgb(r, g, b)
}
