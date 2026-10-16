// Measures what the app spends its time on: frame rate, CPU time on the main and render
// threads, the process's CPU and memory use, and GPU time for each stage of drawing a frame.
// Nothing is measured while it is switched off, so it costs nothing then.
//
// The numbers are averaged and published twice a second in `Readings`, which anything may
// show; the panel shows them as an overlay.

mod gpu;
mod process;
mod threads;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::light::FogVolume;
use bevy::light::VolumetricLight;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::extract_resource::ExtractResourcePlugin;
use bevy::render::renderer::RenderAdapterInfo;
use wgpu::DeviceType;

use crate::process::Process;

const EVERY: Duration = Duration::from_millis(500);

pub struct MonitorPlugin;

impl Plugin for MonitorPlugin {
    fn build(&self, app: &mut App) {
        let samples = Samples::default();
        app.init_resource::<Monitoring>()
            .init_resource::<Readings>()
            .init_resource::<Gathered>()
            .insert_resource(samples.clone())
            .add_plugins(ExtractResourcePlugin::<Monitoring>::default())
            .add_plugins(threads::MainThreadPlugin)
            .add_systems(Last, publish);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(samples).init_resource::<Monitoring>();
        }
        app.add_plugins((threads::RenderThreadPlugin, gpu::GpuClockPlugin));
    }
}

// Whether to measure. Whoever shows the readings switches it.
#[derive(Resource, ExtractResource, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Monitoring(pub bool);

pub(crate) fn monitoring(on: Res<Monitoring>) -> bool {
    on.0
}

// Times in milliseconds, per frame unless said otherwise. None until there is something to
// show, or where this machine can't tell.
#[derive(Resource, Default, Clone, Debug)]
pub struct Readings {
    pub fps: f32,
    pub frame_ms: f32,
    // The app's own work: animation, the scene, the panel.
    pub main_thread_ms: Option<f32>,
    // Getting each frame ready for the GPU, on a thread of its own, while the main thread
    // works on the next one.
    pub render_thread_ms: Option<f32>,
    // The part of the render thread's time spent writing out the GPU's commands.
    pub recording_ms: Option<f32>,
    // Of the whole machine, as Task Manager counts it.
    pub cpu_percent: Option<f32>,
    pub cores: usize,
    pub memory_mb: Option<f32>,
    pub adapter: Option<Adapter>,
    pub gpu: Gpu,
    // Point light faces and spot lights drawn into shadow maps each frame.
    pub shadow_maps: Option<f32>,
    pub lights: LightCounts,
    pub fog_boxes: usize,
}

#[derive(Clone, Debug)]
pub struct Adapter {
    pub name: String,
    pub kind: AdapterKind,
    pub backend: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterKind {
    Discrete,
    Integrated,
    // Drawn by the CPU, with no GPU at all: everything is slow then.
    Software,
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Gpu {
    #[default]
    Waiting,
    // The driver can't time work on the GPU.
    Unsupported,
    Measured(GpuTimes),
}

// One frame's GPU work by stage. The stages add up to `total`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuTimes {
    pub total: f32,
    // Shadow maps of point and spot lights.
    pub light_shadows: f32,
    // Sun shadows, the depth prepass and sorting out what to draw.
    pub prepass: f32,
    // Lighting and drawing every surface.
    pub main: f32,
    pub fog: f32,
    // TAA, depth of field, bloom, tonemapping and the rest after the fog.
    pub effects: f32,
    // Putting the picture on screen, with the panel.
    pub display: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LightCounts {
    pub total: usize,
    pub shadowed: usize,
    pub in_fog: usize,
}

#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct Mean {
    sum: f64,
    count: u32,
}

impl Mean {
    pub(crate) fn add(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
    }

    pub(crate) fn get(self) -> Option<f32> {
        (self.count > 0).then(|| (self.sum / f64::from(self.count)) as f32)
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct GpuMeans {
    total: Mean,
    light_shadows: Mean,
    prepass: Mean,
    main: Mean,
    fog: Mean,
    effects: Mean,
    display: Mean,
}

impl GpuMeans {
    pub(crate) fn add(&mut self, times: &GpuTimes) {
        self.total.add(times.total.into());
        self.light_shadows.add(times.light_shadows.into());
        self.prepass.add(times.prepass.into());
        self.main.add(times.main.into());
        self.fog.add(times.fog.into());
        self.effects.add(times.effects.into());
        self.display.add(times.display.into());
    }

    fn get(self) -> Option<GpuTimes> {
        Some(GpuTimes {
            total: self.total.get()?,
            light_shadows: self.light_shadows.get()?,
            prepass: self.prepass.get()?,
            main: self.main.get()?,
            fog: self.fog.get()?,
            effects: self.effects.get()?,
            display: self.display.get()?,
        })
    }
}

// What the render thread measured since the last readings. Shared with the render world,
// which has no other way to send anything back.
#[derive(Resource, Clone, Default)]
pub(crate) struct Samples(Arc<Mutex<RenderSamples>>);

impl Samples {
    pub(crate) fn lock(&self) -> MutexGuard<'_, RenderSamples> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Default)]
pub(crate) struct RenderSamples {
    pub render_thread: Mean,
    pub recording: Mean,
    pub shadow_maps: Mean,
    pub gpu: GpuMeans,
    pub gpu_unsupported: bool,
}

// What the main thread gathered since the last readings.
#[derive(Resource, Default)]
pub(crate) struct Gathered {
    elapsed: Duration,
    frames: u32,
    pub main_thread: Mean,
}

#[derive(SystemParam)]
struct SceneCounts<'w, 's> {
    points: Query<'w, 's, &'static PointLight>,
    spots: Query<'w, 's, &'static SpotLight>,
    suns: Query<'w, 's, &'static DirectionalLight>,
    in_fog: Query<'w, 's, (), With<VolumetricLight>>,
    fog: Query<'w, 's, (), With<FogVolume>>,
}

impl SceneCounts<'_, '_> {
    fn lights(&self) -> LightCounts {
        let shadowed = self.points.iter().filter(|l| l.shadow_maps_enabled).count()
            + self.spots.iter().filter(|l| l.shadow_maps_enabled).count()
            + self.suns.iter().filter(|l| l.shadow_maps_enabled).count();
        LightCounts {
            total: self.points.iter().len() + self.spots.iter().len() + self.suns.iter().len(),
            shadowed,
            in_fog: self.in_fog.iter().len(),
        }
    }
}

fn publish(
    on: Res<Monitoring>,
    time: Res<Time<Real>>,
    mut gathered: ResMut<Gathered>,
    samples: Res<Samples>,
    mut process: Local<Option<Process>>,
    mut readings: ResMut<Readings>,
    scene: SceneCounts,
    adapter: Option<Res<RenderAdapterInfo>>,
) {
    if !on.0 {
        if gathered.frames > 0 {
            *gathered = Gathered::default();
            *samples.lock() = RenderSamples::default();
            *process = None;
        }
        return;
    }
    gathered.elapsed += time.delta();
    gathered.frames += 1;
    if gathered.elapsed < EVERY {
        return;
    }

    let gathered = std::mem::take(&mut *gathered);
    let render = std::mem::take(&mut *samples.lock());
    let process = process.get_or_insert_with(Process::new);
    let usage = process.sample();
    let seconds = gathered.elapsed.as_secs_f32();

    *readings = Readings {
        fps: gathered.frames as f32 / seconds,
        frame_ms: seconds * 1000.0 / gathered.frames as f32,
        main_thread_ms: gathered.main_thread.get(),
        render_thread_ms: render.render_thread.get(),
        recording_ms: render.recording.get(),
        cpu_percent: usage.cpu_percent,
        cores: process.cores(),
        memory_mb: usage.memory_mb,
        adapter: adapter.map(|info| Adapter {
            name: info.name.clone(),
            kind: match info.device_type {
                DeviceType::DiscreteGpu => AdapterKind::Discrete,
                DeviceType::IntegratedGpu => AdapterKind::Integrated,
                DeviceType::Cpu => AdapterKind::Software,
                _ => AdapterKind::Other,
            },
            backend: format!("{:?}", info.backend),
        }),
        gpu: match render.gpu.get() {
            Some(times) => Gpu::Measured(times),
            None if render.gpu_unsupported => Gpu::Unsupported,
            None => Gpu::Waiting,
        },
        shadow_maps: render.shadow_maps.get(),
        lights: scene.lights(),
        fog_boxes: scene.fog.iter().len(),
    };
}
