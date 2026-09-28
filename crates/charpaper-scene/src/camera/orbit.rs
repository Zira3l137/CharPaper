use bevy::prelude::*;

use crate::state::CharacterState;

const BASE_ZOOM_SPEED: f32 = 0.1;
const BASE_PAN_SPEED: f32 = 0.001;
const BASE_SENSITIVITY: f32 = 0.005;
// Just short of straight up or down, where the orbit would flip over.
pub(crate) const PITCH_LIMIT: f32 = 1.54;

#[derive(Component)]
pub(crate) struct OrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

pub(crate) fn on_zoom(
    event: On<Pointer<Scroll>>,
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
    state: Res<CharacterState>,
) {
    if state.camera.is_some() {
        return;
    }
    let (camera, orbit) = &mut *query;

    let scroll_y = event.y;

    let zoom_factor = 1.0 - scroll_y * BASE_ZOOM_SPEED;
    orbit.radius = (orbit.radius * zoom_factor).clamp(1.0, 100.0);
    update_camera_transform(camera, orbit);
}

pub(crate) fn on_pan(
    event: On<Pointer<Drag>>,
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
    state: Res<CharacterState>,
) {
    if state.camera.is_some() {
        return;
    }
    if !matches!(event.button, PointerButton::Middle) {
        return;
    }

    let (camera, orbit) = &mut *query;

    let delta = event.delta;
    let right = camera.right().as_vec3();
    let up = camera.up().as_vec3();
    let pan_speed = BASE_PAN_SPEED * orbit.radius;

    let world_delta = (-right * delta.x + up * delta.y) * pan_speed;

    orbit.focus += world_delta;
    update_camera_transform(camera, orbit);
}

pub(crate) fn on_orbit(
    event: On<Pointer<Drag>>,
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
    state: Res<CharacterState>,
) {
    if state.camera.is_some() {
        return;
    }
    if !matches!(event.button, PointerButton::Secondary) {
        return;
    }

    let (transform, orbit) = &mut *query;
    let delta = event.delta;

    orbit.yaw -= delta.x * BASE_SENSITIVITY;
    orbit.pitch = (orbit.pitch - delta.y * BASE_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);

    update_camera_transform(transform, orbit);
}

pub(crate) fn update_camera_transform(transform: &mut Transform, orbit: &OrbitCamera) {
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    let offset = rotation * Vec3::new(0.0, 0.0, orbit.radius);
    transform.translation = orbit.focus + offset;
    transform.look_at(orbit.focus, Vec3::Y);
}
