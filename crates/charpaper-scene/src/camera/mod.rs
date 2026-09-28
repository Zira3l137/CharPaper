mod orbit;
mod rig;

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::window::PrimaryWindow;

pub(crate) use rig::ShownRig;

use crate::SceneSet;
use crate::camera::orbit::OrbitCamera;
use crate::camera::orbit::PITCH_LIMIT;
use crate::camera::orbit::update_camera_transform;
use crate::camera::rig::Following;
use crate::render::RenderSettings;
use crate::render::scene_target;
use crate::suite::ActiveSuite;

const ORBIT_FOCUS: [f32; 3] = [0.0, 1.0, 0.0];
const ORBIT_RADIUS: f32 = 2.0;
const ORBIT_YAW_DEG: f32 = 0.0;
const ORBIT_PITCH_DEG: f32 = 0.0;

// The one camera that renders. The mouse moves it while it orbits; otherwise it copies the
// exported camera in use.
#[derive(Component)]
pub(crate) struct SceneCamera;

pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShownRig>()
            .add_observer(orbit::on_pan)
            .add_observer(orbit::on_orbit)
            .add_observer(orbit::on_zoom)
            .add_observer(rig::on_rig_ready)
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (rig::choose_camera, reset_view).in_set(SceneSet::Fill))
            .add_systems(Update, (rig::switch_rig, rig::spawn_rig).chain().in_set(SceneSet::Run))
            // After propagation, so the rig's animated transform is final, and before frusta
            // are built from the camera's.
            .add_systems(
                PostUpdate,
                rig::follow_selected
                    .after(TransformSystems::Propagate)
                    .before(VisibilitySystems::UpdateFrusta),
            );
    }
}

fn spawn_camera(
    mut commands: Commands,
    settings: Res<RenderSettings>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let orbit = orbit_for(None);
    let mut transform = Transform::default();
    update_camera_transform(&mut transform, &orbit);

    let target = scene_target(&mut commands, &mut images, windows.single().ok(), &settings);
    commands.spawn((
        Name::new("Scene camera"),
        SceneCamera,
        Camera3d::default(),
        orbit,
        Following::default(),
        transform,
        target,
    ));
}

// Back to the orbit camera, placed where the new suite says, whenever a suite is shown.
fn reset_view(
    suite: Res<ActiveSuite>,
    camera: Single<(&mut OrbitCamera, &mut Transform, &mut Projection, &mut Following)>,
) {
    let (mut orbit, mut transform, mut projection, mut following) = camera.into_inner();
    *orbit = orbit_for(Some(&suite.camera));
    update_camera_transform(&mut transform, &orbit);
    *projection = Projection::default();
    *following = Following::default();
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
