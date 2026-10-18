mod focus;
mod orbit;
mod rig;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use charpaper_settings::Graphics;

pub(crate) use rig::ShownRig;
pub(crate) use rig::follow_selected;

use crate::SceneSet;
use crate::camera::orbit::OrbitCamera;
use crate::camera::orbit::PITCH_LIMIT;
use crate::camera::orbit::update_camera_transform;
use crate::camera::rig::Following;
use crate::environment::ShownEnvironment;
use crate::render::SceneImage;
use crate::render::ScreenParts;
use crate::render::screen_parts;
use crate::screens::Screen;
use crate::screens::ScreenSync;
use crate::screens::ScreenUi;
use crate::screens::ShowsStage;
use crate::shadows::ShadowReach;
use crate::stage::StageKey;
use crate::stage::StageOf;
use crate::stage::sync_stages;
use crate::suite::ActiveSuite;

const ORBIT_FOCUS: [f32; 3] = [0.0, 1.0, 0.0];
const ORBIT_RADIUS: f32 = 2.0;
const ORBIT_YAW_DEG: f32 = 0.0;
const ORBIT_PITCH_DEG: f32 = 0.0;

// Where a stage is seen from: the orbit, moved by the mouse, or the exported camera in use.
// It renders nothing itself. The cameras that do are its children, so they all look from
// wherever it is. A child of its stage.
#[derive(Component)]
pub(crate) struct SceneView;

// A camera rendering the scene, always a child of the SceneView.
#[derive(Component)]
pub(crate) struct SceneCamera;

pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(rig::on_rig_ready)
            .add_systems(
                PreUpdate,
                (regroup_screens, spawn_screen_cameras)
                    .chain()
                    .after(sync_stages)
                    .in_set(ScreenSync),
            )
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

pub(crate) fn view_bundle() -> impl Bundle {
    let orbit = orbit_for(None);
    let mut transform = Transform::default();
    update_camera_transform(&mut transform, &orbit);
    let reach = ShadowReach::default();
    (Name::new("Scene view"), SceneView, orbit, Following::default(), transform, reach)
}

// A screen moved to another stage, or whose stage went away, loses its camera, display and
// panel, and gets new ones under the stage it shows now.
fn regroup_screens(
    mut commands: Commands,
    screens: Query<(Entity, &ShowsStage, &ScreenParts)>,
    stage_of: StageOf,
    keys: Query<&StageKey>,
) {
    for (screen, shows, parts) in &screens {
        let current = stage_of.get(parts.camera).and_then(|stage| keys.get(stage).ok());
        if current.is_none_or(|key| key.0 != shows.0) {
            commands.entity(screen).remove::<(ScreenParts, ScreenUi)>();
        }
    }
}

// A screen whose window or stage doesn't exist yet is tried again next frame.
fn spawn_screen_cameras(
    mut commands: Commands,
    graphics: Query<&Graphics>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SceneImage>>,
    views: Query<(Entity, &ChildOf), With<SceneView>>,
    keys: Query<&StageKey>,
    screens: Query<(Entity, &Screen, &ShowsStage), Without<ScreenParts>>,
    windows: Query<&Window>,
) {
    for (entity, screen, shows) in &screens {
        let view = views
            .iter()
            .find(|(_, stage)| keys.get(stage.parent()).is_ok_and(|key| key.0 == shows.0));
        let Some((view, stage)) = view else {
            continue;
        };
        let (Ok(window), Ok(graphics)) = (windows.get(screen.window), graphics.get(stage.parent()))
        else {
            continue;
        };
        let (target, mut parts) =
            screen_parts(&mut commands, &mut images, &mut materials, screen, window, graphics);
        // Only the pointer over the scene steers the orbit, so anything drawn on top of the
        // scene, such as the settings panel, keeps its scrolls and drags to itself.
        commands
            .entity(parts.image_node)
            .insert(orbit::Steers(view))
            .observe(orbit::on_pan)
            .observe(orbit::on_orbit)
            .observe(orbit::on_zoom);
        parts.camera = commands
            .spawn((
                Name::new("Scene camera"),
                SceneCamera,
                Camera3d::default(),
                target,
                ChildOf(view),
            ))
            .id();
        commands.entity(entity).insert((ScreenUi(parts.display), parts));
    }
}

// Back to the orbit camera, placed where the new suite says, whenever a suite is shown.
fn reset_view(
    stages: Query<(Entity, &ActiveSuite), Added<ActiveSuite>>,
    mut views: Query<
        (Entity, &mut OrbitCamera, &mut Transform, &mut Following, &ChildOf),
        With<SceneView>,
    >,
    mut cameras: Query<(&mut Projection, &ChildOf), With<SceneCamera>>,
) {
    for (stage, suite) in &stages {
        for (view, mut orbit, mut transform, mut following, parent) in &mut views {
            if parent.parent() != stage {
                continue;
            }
            *orbit = orbit_for(Some(&suite.camera));
            update_camera_transform(&mut transform, &orbit);
            *following = Following::default();
            for (mut projection, _) in cameras.iter_mut().filter(|(_, c)| c.parent() == view) {
                *projection = Projection::default();
            }
        }
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
    views: Query<(Entity, Ref<Following>, &ChildOf), With<SceneView>>,
    stages: Query<Ref<ShownEnvironment>>,
    mut cameras: Query<(&mut TemporalAntiAliasing, &ChildOf)>,
) {
    for (view, following, stage) in &views {
        let shown = stages.get(stage.parent()).is_ok_and(|shown| shown.is_changed());
        if !following.is_changed() && !shown {
            continue;
        }
        for (mut taa, _) in cameras.iter_mut().filter(|(_, c)| c.parent() == view) {
            taa.reset = true;
        }
    }
}
