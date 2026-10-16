use std::time::Instant;

use bevy::prelude::*;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderSystems;

use crate::Gathered;
use crate::Monitoring;
use crate::Samples;
use crate::monitoring;

// The main thread's busy time: from the first system of a frame to the last. Waiting for the
// next frame falls outside it.
pub(crate) struct MainThreadPlugin;

impl Plugin for MainThreadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameStart>()
            .add_systems(First, start_frame.run_if(monitoring))
            .add_systems(Last, end_frame.run_if(monitoring));
    }
}

// One in each world, each read by the system that ends what its partner started.
#[derive(Resource, Default)]
struct FrameStart(Option<Instant>);

fn start_frame(mut start: ResMut<FrameStart>) {
    start.0 = Some(Instant::now());
}

fn end_frame(mut start: ResMut<FrameStart>, mut gathered: ResMut<Gathered>) {
    if let Some(start) = start.0.take() {
        gathered.main_thread.add(start.elapsed().as_secs_f64() * 1000.0);
    }
}

// The render thread's busy time: Bevy's whole Render schedule, which prepares each frame for
// the GPU, writes out its commands and hands them over.
pub(crate) struct RenderThreadPlugin;

impl Plugin for RenderThreadPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<FrameStart>().add_systems(
            Render,
            (
                start_render.in_set(RenderSystems::ExtractCommands),
                end_render.in_set(RenderSystems::PostCleanup),
            ),
        );
    }
}

fn start_render(on: Res<Monitoring>, mut start: ResMut<FrameStart>) {
    start.0 = on.0.then(Instant::now);
}

fn end_render(mut start: ResMut<FrameStart>, samples: Res<Samples>) {
    if let Some(start) = start.0.take() {
        samples.lock().render_thread.add(start.elapsed().as_secs_f64() * 1000.0);
    }
}
