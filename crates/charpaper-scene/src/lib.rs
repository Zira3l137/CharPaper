//! The thing we actually draw.

mod animation;
mod binding;
mod cameras;
mod character;
mod config;
mod environment;
mod importer;
mod look;
mod render;
mod stage;
mod suite;

pub use animation::CharacterClips;
use bevy::camera::visibility::VisibilitySystems;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
pub use character::Character;
pub use character::CharacterState;
pub use character::Picks;
pub use character::SkinObjects;
pub use config::SceneConfig;
pub use look::ActiveLook;
pub use look::LookBackup;
pub use look::Resolved;
pub use look::RestoreLook;
pub use render::AntiAliasing;
pub use render::FpsLimit;
pub use render::RENDER_SCALES;
pub use render::RenderSettings;
pub use suite::ActiveSuite;
pub use suite::AvailableSuites;

/// Asset source id for [`SceneConfig::characters_dir`], so suite files load as
/// `characters://<suite>/<file>`.
pub const CHARACTERS_SOURCE: &str = "characters";

pub const BASE_ZOOM_SPEED: f32 = 0.1;
pub const BASE_PAN_SPEED: f32 = 0.001;
pub const BASE_SENSITIVITY: f32 = 0.005;
/// Just short of straight up or down (~88°), where the orbit would flip over.
const PITCH_LIMIT: f32 = 1.54;

#[derive(Component)]
pub(crate) struct OrbitCamera {
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
            .add_observer(cameras::on_rig_ready)
            .init_resource::<CharacterState>()
            .init_resource::<AvailableSuites>()
            .init_resource::<suite::LoadedSuite>()
            .insert_resource(suite::Remembered(self.config.remembered.clone()))
            .init_resource::<character::ShownSkin>()
            .init_resource::<SkinObjects>()
            .init_resource::<cameras::ShownRig>()
            // No built-in lighting: the environment's own lights and maps are
            // all that light the character.
            .insert_resource(GlobalAmbientLight::NONE)
            .init_resource::<environment::ShownEnvironment>()
            .init_resource::<environment::Baking>()
            .init_resource::<LookBackup>()
            .insert_resource(self.config.render.clone())
            .add_message::<RestoreLook>()
            .add_observer(environment::on_environment_ready)
            .add_systems(Startup, (suite::discover_suites, stage::spawn_stage))
            .add_systems(
                Update,
                (
                    (
                        suite::switch_suite,
                        (
                            character::spawn_character,
                            animation::load_clips,
                            cameras::choose_camera,
                            environment::choose_environment,
                            stage::reset_view,
                        )
                            .run_if(resource_added::<ActiveSuite>),
                    )
                        .chain(),
                    (
                        (
                            animation::make_armature_animatable,
                            animation::build_graph,
                            animation::finish_once,
                            animation::play_selected,
                        )
                            .chain(),
                        (
                            binding::bind_skins,
                            character::list_skin_objects,
                            character::apply_hidden_objects.run_if(
                                resource_changed::<CharacterState>
                                    .or_eager(resource_changed::<SkinObjects>),
                            ),
                        )
                            .chain(),
                        render::fit_scene_target,
                        render::apply_anti_aliasing.run_if(resource_changed::<RenderSettings>),
                        (cameras::switch_rig, cameras::spawn_rig).chain(),
                        (
                            environment::switch_environment,
                            environment::finish_bakes,
                            environment::spawn_environment_scenes,
                            look::apply_look.run_if(look::look_needs_applying),
                        )
                            .chain(),
                        character::switch_skin,
                    ),
                )
                    .chain(),
            )
            // After propagation so the rig's animated transform is final for
            // this frame, and before frusta are built from the camera's.
            .add_systems(
                PostUpdate,
                cameras::follow_selected
                    .after(TransformSystems::Propagate)
                    .before(VisibilitySystems::UpdateFrusta),
            );
    }
}

fn on_zoom(
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

fn on_pan(
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

fn on_orbit(
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

fn update_camera_transform(transform: &mut Transform, orbit: &OrbitCamera) {
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    let offset = rotation * Vec3::new(0.0, 0.0, orbit.radius);
    transform.translation = orbit.focus + offset;
    transform.look_at(orbit.focus, Vec3::Y);
}
