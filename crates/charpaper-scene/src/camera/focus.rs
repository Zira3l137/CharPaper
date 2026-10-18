// Depth of field on each stage's cameras. The lens comes from the camera in use. With a focus
// object, the distance to it is measured again every frame, since either may move.

use bevy::post_process::dof::DepthOfField;
use bevy::post_process::dof::DepthOfFieldMode;
use bevy::prelude::*;
use charpaper_settings::DepthOfFieldQuality;
use charpaper_settings::Graphics;

use crate::camera::SceneCamera;
use crate::camera::SceneView;
use crate::camera::orbit::OrbitCamera;
use crate::camera::rig::CameraRig;
use crate::camera::rig::Following;
use crate::character::Armature;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

// Bevy's default cap on how far one pixel may spread, which keeps the cost bounded.
const MAX_BLUR_PX: f32 = 64.0;
// The sky has no depth, and Bevy blurs it without limit unless told how far away it is. At
// this distance it blurs like Blender's background at infinity.
const SKY_DEPTH: f32 = 10_000.0;

// The entity found for the rig's focus object. Looked up again whenever it is gone: bones
// only exist once the armature has spawned, and a skin's objects leave with the skin.
#[derive(Component, Default)]
pub(crate) struct FocusTarget(Option<Entity>);

struct Lens {
    f_stop: f32,
    sensor_height: f32,
    focal_distance: f32,
}

pub(crate) fn focus_lens(
    mut commands: Commands,
    graphics: Query<&Graphics>,
    suites: Query<&ActiveSuite>,
    views: Query<(Entity, &GlobalTransform, &OrbitCamera, &Following, &ChildOf), With<SceneView>>,
    mut cameras: Query<(Entity, Option<&mut DepthOfField>, &ChildOf), With<SceneCamera>>,
    mut rigs: Query<(Entity, &CameraRig, &mut FocusTarget, &ChildOf)>,
    armatures: Query<Entity, With<Armature>>,
    stage_of: StageOf,
    children: Query<&Children>,
    names: Query<&Name>,
    globals: Query<&GlobalTransform>,
) {
    for (view, eye, orbit, following, stage) in &views {
        let stage = stage.parent();
        let Ok(graphics) = graphics.get(stage) else {
            continue;
        };
        let mode = match graphics.depth_of_field {
            DepthOfFieldQuality::Off => None,
            DepthOfFieldQuality::Blur => Some(DepthOfFieldMode::Gaussian),
            DepthOfFieldQuality::Bokeh => Some(DepthOfFieldMode::Bokeh),
        };
        let lens = match following.name() {
            None => suites.get(stage).ok().and_then(|s| s.camera.f_stop).map(|f_stop| Lens {
                f_stop,
                sensor_height: DepthOfField::default().sensor_height,
                focal_distance: orbit.radius,
            }),
            Some(name) => rigs
                .iter_mut()
                .find(|(_, rig, _, at)| rig.name == name && at.parent() == stage)
                .and_then(|(root, rig, mut target, _)| {
                    let lens = &rig.settings;
                    let f_stop = lens.f_stop?;
                    let object = lens.focus_object.as_deref().and_then(|object| {
                        if let Some(found) = target.0.and_then(|e| globals.get(e).ok()) {
                            return Some(found);
                        }
                        let find = |from: Entity| {
                            children
                                .iter_descendants(from)
                                .find(|&e| names.get(e).is_ok_and(|n| n.as_str() == object))
                        };
                        let armature = armatures.iter().find(|&a| stage_of.get(a) == Some(stage));
                        target.0 = find(root).or_else(|| armature.and_then(find));
                        target.0.and_then(|e| globals.get(e).ok())
                    });
                    // Along the view axis rather than straight-line, as Blender measures it.
                    let focal_distance = match object {
                        Some(object) => {
                            eye.forward().dot(object.translation() - eye.translation()).abs()
                        }
                        None => lens.focus_distance(),
                    };
                    Some(Lens {
                        f_stop,
                        sensor_height: lens.sensor_height_mm() / 1000.0,
                        focal_distance,
                    })
                }),
        };

        let wanted = lens.zip(mode).map(|(lens, mode)| DepthOfField {
            mode,
            focal_distance: lens.focal_distance,
            sensor_height: lens.sensor_height,
            aperture_f_stops: lens.f_stop,
            // Counted in the scene image's pixels, so it follows the render scale to look the
            // same.
            max_circle_of_confusion_diameter: MAX_BLUR_PX * graphics.scale_percent() as f32 / 100.0,
            max_depth: SKY_DEPTH,
        });
        for (camera, depth_of_field, parent) in &mut cameras {
            if parent.parent() != view {
                continue;
            }
            match (wanted, depth_of_field) {
                (Some(wanted), Some(mut depth_of_field)) => *depth_of_field = wanted,
                (Some(wanted), None) => {
                    commands.entity(camera).insert(wanted);
                }
                (None, Some(_)) => {
                    commands.entity(camera).remove::<DepthOfField>();
                }
                (None, None) => {}
            }
        }
    }
}
