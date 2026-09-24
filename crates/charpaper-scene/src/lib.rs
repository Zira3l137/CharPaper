//! The thing we actually draw.

mod animation;
mod binding;
mod character;
mod config;
mod importer;
mod suite;

use bevy::prelude::*;
pub use character::Character;
pub use character::CharacterState;
pub use config::SceneConfig;
pub use suite::ActiveSuite;

/// Asset source id for [`SceneConfig::characters_dir`], so suite files load as
/// `characters://<suite>/<file>`.
pub const CHARACTERS_SOURCE: &str = "characters";

pub const BASE_ZOOM_SPEED: f32 = 0.1;
pub const BASE_PAN_SPEED: f32 = 0.001;
pub const BASE_SENSITIVITY: f32 = 0.005;

#[derive(Component)]
struct OrbitCamera {
    focus: Vec3, // the point being orbited
    radius: f32,
    yaw: f32,   // horizontal angle
    pitch: f32, // vertical angle
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 2.0,
            yaw: 0.0,
            pitch: 0.0, // slight downward tilt to start
        }
    }
}

pub struct ScenePlugin {
    pub config: SceneConfig,
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        let [r, g, b] = self.config.clear_color;

        app.insert_resource(ClearColor(Color::srgb(r, g, b)))
            .insert_resource(self.config.clone())
            .add_observer(on_pan)
            .add_observer(on_orbit)
            .add_observer(on_zoom)
            .add_observer(binding::mark_ready)
            .init_resource::<CharacterState>()
            .add_systems(
                Startup,
                ((suite::select_suite, character::spawn_character).chain(), spawn_scene),
            )
            .add_systems(
                Update,
                (
                    animation::make_armature_animatable,
                    binding::bind_skins,
                    character::show_selected_skin.run_if(resource_changed::<CharacterState>),
                ),
            );
    }
}

fn spawn_scene(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        OrbitCamera::default(),
        Transform::default(),
        AmbientLight { color: Color::srgb(0.6, 0.7, 1.0), brightness: 200.0, ..default() },
    ));
}

fn on_zoom(
    event: On<Pointer<Scroll>>,
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
) {
    let (camera, orbit) = &mut *query;

    let scroll_y = event.y;

    let zoom_factor = 1.0 - scroll_y * BASE_ZOOM_SPEED;
    orbit.radius = (orbit.radius * zoom_factor).clamp(1.0, 100.0);
    update_camera_transform(camera, orbit);
}

fn on_pan(
    event: On<Pointer<Drag>>,
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
) {
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

fn on_orbit(
    event: On<Pointer<Drag>>,
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
) {
    if !matches!(event.button, PointerButton::Secondary) {
        return;
    }

    let (transform, orbit) = &mut *query;
    let delta = event.delta;

    orbit.yaw -= delta.x * BASE_SENSITIVITY;
    orbit.pitch = (orbit.pitch - delta.y * BASE_SENSITIVITY).clamp(-1.54, 1.54); // avoid flipping past straight up/down (~88°)

    update_camera_transform(transform, orbit);
}

fn update_camera_transform(transform: &mut Transform, orbit: &OrbitCamera) {
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    let offset = rotation * Vec3::new(0.0, 0.0, orbit.radius);
    transform.translation = orbit.focus + offset;
    transform.look_at(orbit.focus, Vec3::Y);
}
