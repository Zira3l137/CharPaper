// Depth of field on the scene camera. The lens comes from the camera in use. With a focus
// object, the distance to it is measured again every frame, since either may move.

use bevy::post_process::dof::DepthOfField;
use bevy::post_process::dof::DepthOfFieldMode;
use bevy::prelude::*;

use crate::camera::SceneCamera;
use crate::camera::SceneView;
use crate::camera::orbit::OrbitCamera;
use crate::camera::rig::CameraRig;
use crate::camera::rig::Following;
use crate::character::Armature;
use crate::render::DepthOfFieldQuality;
use crate::render::RenderSettings;
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
    settings: Res<RenderSettings>,
    suite: Option<Res<ActiveSuite>>,
    view: Single<(&GlobalTransform, &OrbitCamera, &Following), With<SceneView>>,
    mut cameras: Query<(Entity, Option<&mut DepthOfField>), With<SceneCamera>>,
    mut rigs: Query<(Entity, &CameraRig, &mut FocusTarget)>,
    armature: Query<Entity, With<Armature>>,
    children: Query<&Children>,
    names: Query<&Name>,
    globals: Query<&GlobalTransform>,
) {
    let (eye, orbit, following) = view.into_inner();

    let lens = match following.name() {
        None => suite.as_ref().and_then(|s| s.camera.f_stop).map(|f_stop| Lens {
            f_stop,
            sensor_height: DepthOfField::default().sensor_height,
            focal_distance: orbit.radius,
        }),
        Some(name) => rigs.iter_mut().find(|(_, rig, _)| rig.name == name).and_then(
            |(root, rig, mut target)| {
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
                    target.0 = find(root).or_else(|| armature.iter().find_map(find));
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
            },
        ),
    };

    let mode = match settings.depth_of_field {
        DepthOfFieldQuality::Off => None,
        DepthOfFieldQuality::Blur => Some(DepthOfFieldMode::Gaussian),
        DepthOfFieldQuality::Bokeh => Some(DepthOfFieldMode::Bokeh),
    };
    let (Some(lens), Some(mode)) = (lens, mode) else {
        for (camera, depth_of_field) in &cameras {
            if depth_of_field.is_some() {
                commands.entity(camera).remove::<DepthOfField>();
            }
        }
        return;
    };

    let wanted = DepthOfField {
        mode,
        focal_distance: lens.focal_distance,
        sensor_height: lens.sensor_height,
        aperture_f_stops: lens.f_stop,
        // Counted in the scene image's pixels, so it follows the render scale to look the same.
        max_circle_of_confusion_diameter: MAX_BLUR_PX * settings.scale_percent() as f32 / 100.0,
        max_depth: SKY_DEPTH,
    };
    for (camera, depth_of_field) in &mut cameras {
        match depth_of_field {
            Some(mut depth_of_field) => *depth_of_field = wanted,
            None => {
                commands.entity(camera).insert(wanted);
            }
        }
    }
}
