use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use charpaper_settings::ViewSettings;

use crate::camera::SceneView;
use crate::state::CharacterState;

// The orbit radius is multiplied by this once per wheel notch towards the screen, so a fast
// spin can't overshoot to zero.
const ZOOM_PER_NOTCH: f32 = 0.81;
const BASE_PAN_SPEED: f32 = 0.002;
const BASE_SENSITIVITY: f32 = 0.01;
// Just short of straight up or down, where the orbit would flip over.
pub(crate) const PITCH_LIMIT: f32 = 1.54;

#[derive(Component)]
pub(crate) struct OrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

// On a screen's scene image: the view a drag or scroll over it steers.
#[derive(Component)]
pub(crate) struct Steers(pub Entity);

// The view under the pointer, while its stage looks through the orbit rather than an exported
// camera.
#[derive(SystemParam)]
pub(crate) struct Steered<'w, 's> {
    images: Query<'w, 's, &'static Steers>,
    views: Query<
        'w,
        's,
        (&'static mut Transform, &'static mut OrbitCamera, &'static ChildOf),
        With<SceneView>,
    >,
    stages: Query<'w, 's, (&'static CharacterState, &'static ViewSettings)>,
}

impl Steered<'_, '_> {
    fn free(&mut self, image: Entity) -> Option<(Mut<'_, Transform>, Mut<'_, OrbitCamera>)> {
        let view = self.images.get(image).ok()?.0;
        let stage = self.views.get(view).ok()?.2.parent();
        let (state, view_settings) = self.stages.get(stage).ok()?;
        if state.camera.is_some() || view_settings.locked {
            return None;
        }
        let (transform, orbit, _) = self.views.get_mut(view).ok()?;
        Some((transform, orbit))
    }
}

pub(crate) fn on_zoom(event: On<Pointer<Scroll>>, mut steered: Steered) {
    let Some((mut camera, mut orbit)) = steered.free(event.entity) else {
        return;
    };
    orbit.radius = (orbit.radius * ZOOM_PER_NOTCH.powf(event.y)).clamp(1.0, 100.0);
    update_camera_transform(&mut camera, &orbit);
}

pub(crate) fn on_pan(event: On<Pointer<Drag>>, mut steered: Steered) {
    if !matches!(event.button, PointerButton::Middle) {
        return;
    }
    let Some((mut camera, mut orbit)) = steered.free(event.entity) else {
        return;
    };

    let delta = event.delta;
    let right = camera.right().as_vec3();
    let up = camera.up().as_vec3();
    let pan_speed = BASE_PAN_SPEED * orbit.radius;

    let world_delta = (-right * delta.x + up * delta.y) * pan_speed;

    orbit.focus += world_delta;
    update_camera_transform(&mut camera, &orbit);
}

pub(crate) fn on_orbit(event: On<Pointer<Drag>>, mut steered: Steered) {
    if !matches!(event.button, PointerButton::Secondary) {
        return;
    }
    let Some((mut transform, mut orbit)) = steered.free(event.entity) else {
        return;
    };
    let delta = event.delta;

    orbit.yaw -= delta.x * BASE_SENSITIVITY;
    orbit.pitch = (orbit.pitch - delta.y * BASE_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);

    update_camera_transform(&mut transform, &orbit);
}

pub(crate) fn update_camera_transform(transform: &mut Transform, orbit: &OrbitCamera) {
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    let offset = rotation * Vec3::new(0.0, 0.0, orbit.radius);
    transform.translation = orbit.focus + offset;
    transform.look_at(orbit.focus, Vec3::Y);
}
