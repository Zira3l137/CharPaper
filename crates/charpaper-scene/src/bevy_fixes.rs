// Fixes for bugs in the Bevy this app is on, each to be deleted once Bevy ships its own.

use bevy::core_pipeline::prepass::BackgroundMotionVectorsBindGroup;
use bevy::core_pipeline::prepass::BackgroundMotionVectorsPipelineId;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::core_pipeline::prepass::NoBackgroundMotionVectors;
use bevy::prelude::*;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderSystems;

pub(crate) struct BevyFixesPlugin;

impl Plugin for BevyFixesPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(Render, forget_background_motion_vectors.in_set(RenderSystems::Prepare));
    }
}

// Bevy 0.19 gives a camera with motion vectors a pipeline that writes them behind the scene,
// but never takes it back once the camera stops having them, as when TAA is turned off. The
// prepass then runs that pipeline in a pass with no motion vector texture, which wgpu rejects
// and the app quits. Bevy 0.20 does this itself (cleanup_background_motion_vectors).
fn forget_background_motion_vectors(
    mut commands: Commands,
    views: Query<
        Entity,
        (
            With<BackgroundMotionVectorsPipelineId>,
            Or<(Without<MotionVectorPrepass>, With<NoBackgroundMotionVectors>)>,
        ),
    >,
) {
    for view in &views {
        commands
            .entity(view)
            .remove::<(BackgroundMotionVectorsPipelineId, BackgroundMotionVectorsBindGroup)>();
    }
}
