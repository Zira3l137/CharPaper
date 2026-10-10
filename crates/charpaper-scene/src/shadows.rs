// Bevy gives a sun four shadow cascades reaching 150 m, sized for open worlds. Shadows only
// matter as far as there is something to fall on, so the cascades end just past the farthest
// thing in view: in a room usually one cascade, drawn once a frame instead of four times, and
// sharper, since the same shadow map covers less.

use bevy::camera::primitives::Aabb;
use bevy::light::CascadeShadowConfig;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::light::SimulationLightSystems;
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use charpaper_suite::FIRST_CASCADE;
use charpaper_suite::MAX_SHADOW_DISTANCE;
use charpaper_suite::sun_cascades;

use crate::camera::SceneView;
use crate::camera::follow_selected;

pub(crate) struct ShadowsPlugin;

impl Plugin for ShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShadowReach>().add_systems(
            PostUpdate,
            fit_cascades
                .after(TransformSystems::Propagate)
                .after(follow_selected)
                .before(SimulationLightSystems::UpdateDirectionalLightCascades),
        );
    }
}

// In whole meters, and kept until the scene needs a meter more or two less: every refit
// changes the size of the shadow maps' texels, which makes shadow edges crawl, so a moving
// camera mustn't cause one every frame.
#[derive(Resource, Default)]
struct ShadowReach(Option<f32>);

fn fit_cascades(
    mut reach: ResMut<ShadowReach>,
    camera: Query<&GlobalTransform, With<SceneView>>,
    meshes: Query<(&Aabb, &GlobalTransform, &InheritedVisibility), With<Mesh3d>>,
    mut suns: Query<&mut CascadeShadowConfig, With<DirectionalLight>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    if suns.is_empty() {
        return;
    }

    let eye = Vec3A::from(camera.translation());
    let forward = Vec3A::from(*camera.forward());
    let mut deepest = 0.0_f32;
    for (aabb, transform, visible) in &meshes {
        if !visible.get() {
            continue;
        }
        let affine = transform.affine();
        let center = affine.transform_point3a(aabb.center) - eye;
        let reach_along = (affine.matrix3.transpose() * forward).abs().dot(aabb.half_extents);
        deepest = deepest.max(center.dot(forward) + reach_along);
    }

    let needed = (deepest + 1.0).ceil().clamp(1.0, MAX_SHADOW_DISTANCE);
    let distance = match reach.0 {
        Some(kept) if needed <= kept && needed > kept - 2.0 => kept,
        _ => {
            debug!("sun shadows reach {needed} m in {} cascade(s)", sun_cascades(needed));
            reach.0 = Some(needed);
            needed
        }
    };

    let config: CascadeShadowConfig = CascadeShadowConfigBuilder {
        num_cascades: sun_cascades(distance),
        maximum_distance: distance,
        first_cascade_far_bound: FIRST_CASCADE.min(distance),
        ..default()
    }
    .into();
    for mut sun in &mut suns {
        if sun.bounds != config.bounds {
            *sun = config.clone();
        }
    }
}
