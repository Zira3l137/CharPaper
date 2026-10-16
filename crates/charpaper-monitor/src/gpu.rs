// Times the GPU's work stage by stage. The GPU runs a frame long after the CPU has written it
// out, so the CPU can't time it. Instead it asks the GPU to write down its clock at marks
// between the stages, then reads those times back a frame or two later.
//
// Bevy draws a frame in a fixed order, which the marks follow:
//
//   frame start
//     every point light face and spot light into its shadow map
//   for each scene camera: camera start
//     culling, the depth prepass and sun shadows       → main start
//     the main pass                                    → main end
//     fog                                              → fog end
//     TAA, depth of field, bloom, tonemapping          → camera end
//   the panel's cameras: the scene image and the panel
//   frame end

use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::time::Instant;

use bevy::anti_alias::taa::temporal_anti_alias;
use bevy::core_pipeline::core_3d::main_opaque_pass_3d;
use bevy::core_pipeline::core_3d::main_transparent_pass_3d;
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::schedule::Core3dSystems;
use bevy::core_pipeline::schedule::RootNonCameraView;
use bevy::core_pipeline::upscaling::upscaling;
use bevy::pbr::EARLY_SHADOW_PASS;
use bevy::pbr::clear_indirect_parameters_metadata;
use bevy::pbr::per_view_shadow_pass;
use bevy::pbr::shared_shadow_pass;
use bevy::prelude::*;
use bevy::render::GpuResourceAppExt;
use bevy::render::RenderApp;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_resource::Buffer;
use bevy::render::render_resource::BufferDescriptor;
use bevy::render::render_resource::BufferUsages;
use bevy::render::render_resource::CommandEncoder;
use bevy::render::render_resource::CommandEncoderDescriptor;
use bevy::render::render_resource::MapMode;
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::renderer::PendingCommandBuffers;
use bevy::render::renderer::RenderContext;
use bevy::render::renderer::RenderDevice;
use bevy::render::renderer::RenderGraph;
use bevy::render::renderer::RenderGraphSystems;
use bevy::render::renderer::RenderQueue;
use bevy::render::renderer::ViewQuery;
use wgpu::QuerySet;
use wgpu::QuerySetDescriptor;
use wgpu::QueryType;

use crate::GpuTimes;
use crate::Monitoring;
use crate::Samples;

// Two per frame and six per scene camera, so room for a dozen screens.
const MAX_MARKS: u32 = 80;
// Frames whose times are still on their way back. More are skipped rather than waited for.
const MAX_FRAMES: usize = 4;

pub(crate) struct GpuClockPlugin;

impl Plugin for GpuClockPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_gpu_resource::<GpuClock>()
            .add_systems(
                RenderGraph,
                (
                    begin_frame.in_set(RenderGraphSystems::Begin),
                    end_frame.after(RenderGraphSystems::Render).before(RenderGraphSystems::Submit),
                    send_frame.in_set(RenderGraphSystems::Finish),
                ),
            )
            .add_systems(
                Core3d,
                (
                    mark::<CAMERA_START>
                        .before(Core3dSystems::Prepass)
                        .before(clear_indirect_parameters_metadata)
                        .before(per_view_shadow_pass::<EARLY_SHADOW_PASS>)
                        .before(shared_shadow_pass::<EARLY_SHADOW_PASS>),
                    // Inside the sets, so that nothing Bevy runs between them, fog included,
                    // can land on the wrong side of a mark.
                    mark::<MAIN_START>.in_set(Core3dSystems::MainPass).before(main_opaque_pass_3d),
                    mark::<MAIN_END>
                        .in_set(Core3dSystems::MainPass)
                        .after(main_opaque_pass_3d)
                        .after(main_transparent_pass_3d),
                    mark::<FOG_END>
                        .in_set(Core3dSystems::EarlyPostProcess)
                        .before(temporal_anti_alias),
                    mark::<CAMERA_END>.after(Core3dSystems::PostProcess).after(upscaling),
                ),
            );
    }
}

const FRAME_START: u8 = 0;
const CAMERA_START: u8 = 1;
const MAIN_START: u8 = 2;
const MAIN_END: u8 = 3;
const FOG_END: u8 = 4;
const CAMERA_END: u8 = 5;
const FRAME_END: u8 = 6;

#[derive(Resource)]
pub(crate) struct GpuClock {
    supported: bool,
    // Nanoseconds per tick of the GPU's clock.
    period: f32,
    spare: Vec<ClockFrame>,
    current: Option<ClockFrame>,
    sent: Vec<ClockFrame>,
    made: usize,
}

impl FromWorld for GpuClock {
    fn from_world(world: &mut World) -> Self {
        let wanted = WgpuFeatures::TIMESTAMP_QUERY | WgpuFeatures::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        Self {
            supported: world.resource::<RenderDevice>().features().contains(wanted),
            period: world.resource::<RenderQueue>().get_timestamp_period(),
            spare: Vec::new(),
            current: None,
            sent: Vec::new(),
            made: 0,
        }
    }
}

struct ClockFrame {
    queries: QuerySet,
    resolved: Buffer,
    readable: Buffer,
    // What each written timestamp marks, and for the camera marks, which camera.
    marks: Vec<(u8, u32)>,
    cameras: u32,
    started: Instant,
    state: Arc<AtomicU8>,
}

const MAPPING: u8 = 0;
const MAPPED: u8 = 1;
const FAILED: u8 = 2;

impl ClockFrame {
    fn new(device: &RenderDevice) -> Self {
        let size = u64::from(MAX_MARKS) * 8;
        let buffer = |label, usage| {
            device.create_buffer(&BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            queries: device.wgpu_device().create_query_set(&QuerySetDescriptor {
                label: Some("monitor_timestamps"),
                ty: QueryType::Timestamp,
                count: MAX_MARKS,
            }),
            resolved: buffer(
                "monitor_resolved",
                BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
            ),
            readable: buffer("monitor_readable", BufferUsages::COPY_DST | BufferUsages::MAP_READ),
            marks: Vec::new(),
            cameras: 0,
            started: Instant::now(),
            state: Arc::new(AtomicU8::new(MAPPING)),
        }
    }

    fn mark(&mut self, encoder: &mut CommandEncoder, mark: u8) {
        let index = self.marks.len() as u32;
        if index >= MAX_MARKS {
            return;
        }
        if mark == CAMERA_START {
            self.cameras += 1;
        }
        encoder.write_timestamp(&self.queries, index);
        self.marks.push((mark, self.cameras));
    }

    fn times(&self, period: f32) -> Option<GpuTimes> {
        let count = self.marks.len();
        let data = self.readable.slice(..).get_mapped_range();
        let stamps: Vec<u64> = data
            .chunks_exact(8)
            .take(count)
            .map(|bytes| u64::from_le_bytes(bytes.try_into().unwrap_or_default()))
            .collect();
        drop(data);

        let at = |mark: u8, camera: u32| {
            self.marks.iter().position(|&m| m == (mark, camera)).map(|i| stamps[i])
        };
        let ms = |from: Option<u64>, to: Option<u64>| match (from, to) {
            (Some(from), Some(to)) => to.saturating_sub(from) as f32 * period / 1e6,
            _ => 0.0,
        };

        let start = at(FRAME_START, 0)?;
        let end = at(FRAME_END, self.cameras)?;
        let mut times = GpuTimes { total: ms(Some(start), Some(end)), ..default() };
        if self.cameras == 0 {
            times.light_shadows = times.total;
            return Some(times);
        }
        times.light_shadows = ms(Some(start), at(CAMERA_START, 1));
        for camera in 1..=self.cameras {
            let mark = |mark| at(mark, camera);
            times.prepass += ms(mark(CAMERA_START), mark(MAIN_START));
            times.main += ms(mark(MAIN_START), mark(MAIN_END));
            times.fog += ms(mark(MAIN_END), mark(FOG_END));
            times.effects += ms(mark(FOG_END), mark(CAMERA_END));
        }
        times.display = ms(at(CAMERA_END, self.cameras), Some(end));
        Some(times)
    }
}

// Takes in the times that came back since last frame, then starts this frame's.
fn begin_frame(
    on: Res<Monitoring>,
    clock: Option<ResMut<GpuClock>>,
    device: Res<RenderDevice>,
    mut pending: ResMut<PendingCommandBuffers>,
    samples: Res<Samples>,
    shadow_views: Query<(), With<RootNonCameraView>>,
) {
    let Some(mut clock) = clock else {
        return;
    };
    let clock = &mut *clock;
    let mut i = 0;
    while i < clock.sent.len() {
        let state = clock.sent[i].state.load(Ordering::Acquire);
        if state == MAPPING {
            i += 1;
            continue;
        }
        let mut frame = clock.sent.swap_remove(i);
        if state == MAPPED {
            if let Some(times) = frame.times(clock.period)
                && on.0
            {
                samples.lock().gpu.add(&times);
            }
            frame.readable.unmap();
        }
        frame.marks.clear();
        frame.cameras = 0;
        frame.state.store(MAPPING, Ordering::Release);
        clock.spare.push(frame);
    }

    if !on.0 {
        return;
    }
    let mut samples = samples.lock();
    samples.shadow_maps.add(shadow_views.iter().count() as f64);
    if !clock.supported {
        samples.gpu_unsupported = true;
        return;
    }
    let frame = match clock.spare.pop() {
        Some(frame) => frame,
        None if clock.made < MAX_FRAMES => {
            clock.made += 1;
            ClockFrame::new(&device)
        }
        None => return,
    };
    let frame = clock.current.insert(frame);
    frame.started = Instant::now();
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    frame.mark(&mut encoder, FRAME_START);
    pending.push_encoder(encoder);
}

// Runs once for each view Bevy draws. Shadow maps are views too, but have no camera.
fn mark<const MARK: u8>(
    _camera: ViewQuery<(), With<ExtractedCamera>>,
    clock: Option<ResMut<GpuClock>>,
    mut ctx: RenderContext,
) {
    if let Some(frame) = clock.and_then(|clock| clock.into_inner().current.as_mut()) {
        frame.mark(ctx.command_encoder(), MARK);
    }
}

fn end_frame(
    clock: Option<ResMut<GpuClock>>,
    device: Res<RenderDevice>,
    mut pending: ResMut<PendingCommandBuffers>,
    samples: Res<Samples>,
) {
    let Some(frame) = clock.and_then(|clock| clock.into_inner().current.as_mut()) else {
        return;
    };
    samples.lock().recording.add(frame.started.elapsed().as_secs_f64() * 1000.0);
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    frame.mark(&mut encoder, FRAME_END);
    let count = frame.marks.len() as u32;
    encoder.resolve_query_set(&frame.queries, 0..count, &frame.resolved, 0);
    encoder.copy_buffer_to_buffer(&frame.resolved, 0, &frame.readable, 0, u64::from(count) * 8);
    pending.push_encoder(encoder);
}

// Only once the frame's commands are submitted may the buffer they fill be mapped.
fn send_frame(clock: Option<ResMut<GpuClock>>) {
    let Some(mut clock) = clock else {
        return;
    };
    let Some(frame) = clock.current.take() else {
        return;
    };
    let state = frame.state.clone();
    frame.readable.slice(..).map_async(MapMode::Read, move |result| {
        state.store(if result.is_ok() { MAPPED } else { FAILED }, Ordering::Release);
    });
    clock.sent.push(frame);
}
