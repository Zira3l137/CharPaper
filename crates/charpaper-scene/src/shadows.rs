// Bevy gives a sun four shadow cascades reaching 150 m, sized for open worlds. Shadows only
// matter as far as there is something to fall on, so the cascades end just past the farthest
// thing in view: in a room usually one cascade, drawn once a frame instead of four times, and
// sharper, since the same shadow map covers less.

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
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
        app.add_systems(
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
// camera mustn't cause one every frame. One per stage's view.
#[derive(Component, Default)]
pub(crate) struct ShadowReach(Option<f32>);

// Each stage on its own: its suns, the meshes they fall on and the view are all on the
// stage's render layer.
fn fit_cascades(
    mut views: Query<(&GlobalTransform, &RenderLayers, &mut ShadowReach), With<SceneView>>,
    meshes: Meshes,
    mut suns: Query<(&mut CascadeShadowConfig, &RenderLayers), With<DirectionalLight>>,
) {
    for (eye, layers, mut reach) in &mut views {
        if !suns.iter().any(|(_, sun)| sun.intersects(layers)) {
            continue;
        }
        let Some(distance) = reach_for(eye, layers, &meshes, &mut reach) else {
            continue;
        };
        let config: CascadeShadowConfig = CascadeShadowConfigBuilder {
            num_cascades: sun_cascades(distance),
            maximum_distance: distance,
            first_cascade_far_bound: FIRST_CASCADE.min(distance),
            ..default()
        }
        .into();
        for (mut sun, sun_layers) in &mut suns {
            if sun_layers.intersects(layers) && sun.bounds != config.bounds {
                *sun = config.clone();
            }
        }
    }
}

type Meshes<'w, 's> = Query<
    'w,
    's,
    (&'static Aabb, &'static GlobalTransform, &'static InheritedVisibility, &'static RenderLayers),
    With<Mesh3d>,
>;

fn reach_for(
    eye: &GlobalTransform,
    layers: &RenderLayers,
    meshes: &Meshes,
    reach: &mut ShadowReach,
) -> Option<f32> {
    let forward = Vec3A::from(*eye.forward());
    let eye = Vec3A::from(eye.translation());
    let mut deepest = 0.0_f32;
    for (aabb, transform, visible, mesh_layers) in meshes {
        if !visible.get() || !mesh_layers.intersects(layers) {
            continue;
        }
        let affine = transform.affine();
        let center = affine.transform_point3a(aabb.center) - eye;
        let reach_along = (affine.matrix3.transpose() * forward).abs().dot(aabb.half_extents);
        deepest = deepest.max(center.dot(forward) + reach_along);
    }

    let needed = (deepest + 1.0).ceil().clamp(1.0, MAX_SHADOW_DISTANCE);
    match reach.0 {
        Some(kept) if needed <= kept && needed > kept - 2.0 => Some(kept),
        _ => {
            debug!("sun shadows reach {needed} m in {} cascade(s)", sun_cascades(needed));
            reach.0 = Some(needed);
            Some(needed)
        }
    }
}
