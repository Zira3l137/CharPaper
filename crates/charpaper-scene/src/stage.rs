//! Everything around the character: the environment, the camera and its
//! effects.
//!
//! There is no built-in lighting. The environment's own lights and reflection
//! maps are all that light the character, so the app's global ambient light
//! is switched off in `ScenePlugin`.

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
    let post = suite.as_ref().map(|s| s.post.clone()).unwrap_or_default();
    let view = suite.as_ref().map(|s| s.camera.clone()).unwrap_or_default();

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

    let mut camera = commands.spawn((Camera3d::default(), orbit, Following::default(), transform));
    apply_post(&mut camera, &post);

    let Some(suite) = suite else {
        return;
    };
    let Some(environment) =
        suite.environments.iter().find(|e| Some(&e.name) == suite.default_environment.as_ref())
    else {
        return;
    };
    if let Some(sky) = environment.sky() {
        camera.insert(Skybox {
            image: Some(assets.load(asset_path(&suite, sky))),
            brightness: environment.settings.brightness.unwrap_or(SKYBOX_BRIGHTNESS),
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
