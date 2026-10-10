mod focus;
mod orbit;
mod rig;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::window::PrimaryWindow;

pub(crate) use rig::ShownRig;
pub(crate) use rig::follow_selected;

use crate::SceneSet;
use crate::camera::orbit::OrbitCamera;
use crate::camera::orbit::PITCH_LIMIT;
use crate::camera::orbit::update_camera_transform;
use crate::camera::rig::Following;
use crate::environment::ShownEnvironment;
use crate::render::RenderSettings;
use crate::render::SceneImage;
use crate::render::scene_target;
use crate::suite::ActiveSuite;

const ORBIT_FOCUS: [f32; 3] = [0.0, 1.0, 0.0];
const ORBIT_RADIUS: f32 = 2.0;
const ORBIT_YAW_DEG: f32 = 0.0;
const ORBIT_PITCH_DEG: f32 = 0.0;

// Where the scene is seen from: the orbit, moved by the mouse, or the exported camera in use.
// It renders nothing itself. The cameras that do are its children, so they all look from
// wherever it is.
#[derive(Component)]
pub(crate) struct SceneView;

// A camera rendering the scene, always a child of the SceneView.
#[derive(Component)]
pub(crate) struct SceneCamera;

pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShownRig>()
            .add_observer(rig::on_rig_ready)
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (rig::choose_camera, reset_view).in_set(SceneSet::Fill))
            .add_systems(Update, (rig::switch_rig, rig::spawn_rig).chain().in_set(SceneSet::Run))
            // After propagation, so the rig's animated transform is final, and before frusta
            // are built from the camera's. Focus is measured once the camera has moved.
            .add_systems(
                PostUpdate,
                (
                    rig::follow_selected.before(VisibilitySystems::UpdateFrusta),
                    focus::focus_lens,
                    reset_history,
                )
                    .chain()
                    .after(TransformSystems::Propagate),
            );
    }
}

fn spawn_camera(
    mut commands: Commands,
    settings: Res<RenderSettings>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SceneImage>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let orbit = orbit_for(None);
    let mut transform = Transform::default();
    update_camera_transform(&mut transform, &orbit);

    let window = windows.single().ok();
    let (target, image_node) =
        scene_target(&mut commands, &mut images, &mut materials, window, &settings);
    // Only the pointer over the scene steers the orbit, so anything drawn on top of the scene,
    // such as the settings panel, keeps its scrolls and drags to itself.
    commands
        .entity(image_node)
        .observe(orbit::on_pan)
        .observe(orbit::on_orbit)
        .observe(orbit::on_zoom);
    let view = commands
        .spawn((Name::new("Scene view"), SceneView, orbit, Following::default(), transform))
        .id();
    commands.spawn((
        Name::new("Scene camera"),
        SceneCamera,
        Camera3d::default(),
        target,
        ChildOf(view),
    ));
}

// Back to the orbit camera, placed where the new suite says, whenever a suite is shown.
fn reset_view(
    suite: Res<ActiveSuite>,
    view: Single<(&mut OrbitCamera, &mut Transform, &mut Following), With<SceneView>>,
    mut cameras: Query<&mut Projection, With<SceneCamera>>,
) {
    let (mut orbit, mut transform, mut following) = view.into_inner();
    *orbit = orbit_for(Some(&suite.camera));
    update_camera_transform(&mut transform, &orbit);
    *following = Following::default();
    for mut projection in &mut cameras {
        *projection = Projection::default();
    }
}

fn orbit_for(view: Option<&charpaper_suite::Camera>) -> OrbitCamera {
    let view = view.cloned().unwrap_or_default();
    OrbitCamera {
        focus: Vec3::from_array(view.focus.unwrap_or(ORBIT_FOCUS)),
        radius: view.radius.unwrap_or(ORBIT_RADIUS),
        yaw: view.yaw_deg.unwrap_or(ORBIT_YAW_DEG).to_radians(),
        // The orbit's pitch grows downward, the manifest's upward.
        pitch: (-view.pitch_deg.unwrap_or(ORBIT_PITCH_DEG).to_radians())
            .clamp(-PITCH_LIMIT, PITCH_LIMIT),
    }
}

// After a cut to another camera or environment, TAA's past frames show something else, and
// blending them in would leave a ghost of it for a moment.
fn reset_history(
    shown: Res<ShownEnvironment>,
    following: Single<Ref<Following>, With<SceneView>>,
    mut cameras: Query<&mut TemporalAntiAliasing>,
) {
    if following.is_changed() || shown.is_changed() {
        for mut taa in &mut cameras {
            taa.reset = true;
        }
    }
}
