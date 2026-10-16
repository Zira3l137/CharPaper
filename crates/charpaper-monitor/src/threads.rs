use std::time::Instant;

use bevy::prelude::*;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderSystems;

use crate::Gathered;
use crate::Monitoring;
use crate::RenderStages;
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

// The render thread's busy time, by the stages of Bevy's Render schedule, which prepares
// each frame for the GPU, writes out its commands and hands them over. A mark between each
// two stages takes the time since the one before.
pub(crate) struct RenderThreadPlugin;

impl Plugin for RenderThreadPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<StageClock>().add_systems(
            Render,
            (
                start_render.in_set(RenderSystems::ExtractCommands),
                mark::<ASSETS>.after(RenderSystems::CreateViews).before(RenderSystems::Specialize),
                mark::<SPECIALIZE>
                    .after(RenderSystems::Specialize)
                    .before(RenderSystems::PrepareViews),
                mark::<VIEWS>.after(RenderSystems::PrepareViews).before(RenderSystems::Queue),
                mark::<QUEUE>.after(RenderSystems::PhaseSort).before(RenderSystems::Prepare),
                mark::<PREPARE>.after(RenderSystems::Prepare).before(RenderSystems::Render),
                mark::<RENDER>.after(RenderSystems::Render).before(RenderSystems::Cleanup),
                end_render.in_set(RenderSystems::PostCleanup),
            ),
        );
    }
}

const ASSETS: usize = 0;
const SPECIALIZE: usize = 1;
const VIEWS: usize = 2;
const QUEUE: usize = 3;
const PREPARE: usize = 4;
const RENDER: usize = 5;
const CLEANUP: usize = 6;

#[derive(Resource, Default)]
struct StageClock {
    started: Option<Instant>,
    last: Option<Instant>,
    stages: [f64; RenderStages::COUNT],
}

fn start_render(on: Res<Monitoring>, mut clock: ResMut<StageClock>) {
    let now = on.0.then(Instant::now);
    *clock = StageClock { started: now, last: now, ..default() };
}

fn mark<const STAGE: usize>(mut clock: ResMut<StageClock>) {
    let Some(last) = clock.last else {
        return;
    };
    let now = Instant::now();
    clock.stages[STAGE] = (now - last).as_secs_f64() * 1000.0;
    clock.last = Some(now);
}

fn end_render(mut clock: ResMut<StageClock>, samples: Res<Samples>) {
    let (Some(started), Some(last)) = (clock.started.take(), clock.last.take()) else {
        return;
    };
    clock.stages[CLEANUP] = last.elapsed().as_secs_f64() * 1000.0;
    let mut samples = samples.lock();
    samples.render_thread.add(started.elapsed().as_secs_f64() * 1000.0);
    samples.render_stages.add(&clock.stages);
}
