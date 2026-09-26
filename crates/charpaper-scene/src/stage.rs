//! The one real camera, starting where the suite's orbit settings say. What
//! it sees through (tonemapping, exposure, bloom, sky) is set by `look.rs`,
//! and what it draws into by `render.rs`.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::OrbitCamera;
use crate::PITCH_LIMIT;
use crate::cameras::Following;
use crate::render::RenderSettings;
use crate::render::scene_target;
use crate::suite::ActiveSuite;
use crate::update_camera_transform;

/// Roughly the chest of a human-sized character standing at the origin.
const ORBIT_FOCUS: [f32; 3] = [0.0, 1.0, 0.0];
const ORBIT_RADIUS: f32 = 2.0;
const ORBIT_YAW_DEG: f32 = 0.0;
const ORBIT_PITCH_DEG: f32 = 0.0;

pub(crate) fn spawn_stage(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    settings: Res<RenderSettings>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
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

    let target = scene_target(&mut commands, &mut images, windows.single().ok(), &settings);
    commands.spawn((Camera3d::default(), orbit, Following::default(), transform, target));
}
