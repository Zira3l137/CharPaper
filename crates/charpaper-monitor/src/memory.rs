// Where the app's memory goes, and which part keeps growing. Each kind of memory is told
// apart from the others:
// - the working set: every page of RAM the process touches, driver and DLLs included, which
//   is what Task Manager shows;
// - committed memory: what the process has asked Windows for, touched or not;
// - the heap: what Rust code holds, counted by CountingAllocator;
// - GPU memory: what wgpu has allocated on the graphics card;
// - counts of things that each hold memory, which point at what leaks when one keeps growing.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

use bevy::ecs::entity::Entities;
use bevy::ecs::system::SystemParam;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::render::renderer::RenderDevice;

use crate::heap;

// How far back growth is measured. Shorter would show the ups and downs of single frames.
const GROWTH_WINDOW: Duration = Duration::from_secs(60);
const MIN_WINDOW: Duration = Duration::from_secs(10);
const LARGEST_SHOWN: usize = 3;
const MB: f64 = 1024.0 * 1024.0;

#[derive(Clone, Debug, Default)]
pub struct Memory {
    pub working_set: Option<Amount>,
    // Windows only.
    pub committed: Option<Amount>,
    pub heap: Option<Amount>,
    pub heap_blocks: usize,
    pub gpu: Option<GpuMemory>,
    pub gpu_objects: Option<GpuObjects>,
    pub entities: usize,
    pub render_entities: Option<usize>,
    pub assets: AssetCounts,
}

// Megabytes, and how many more each minute over the last minute.
#[derive(Clone, Copy, Debug, Default)]
pub struct Amount {
    pub mb: f32,
    pub per_minute: Option<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct GpuMemory {
    pub allocated: Amount,
    // Taken from the driver in blocks; what isn't allocated from them yet is spare.
    pub reserved_mb: f32,
    // The largest kinds, by the name wgpu gives each allocation, with megabytes.
    pub largest: Vec<(String, f32)>,
}

// Live wgpu objects. A leak in rendering shows as one of these growing frame after frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuObjects {
    pub buffers: isize,
    pub textures: isize,
    pub texture_views: isize,
    pub bind_groups: isize,
    pub samplers: isize,
    // Render and compute. Bevy keeps every pipeline it compiles for as long as the app runs.
    pub pipelines: isize,
    pub shader_modules: isize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AssetCounts {
    pub images: usize,
    pub meshes: usize,
    pub materials: usize,
    pub clips: usize,
    pub gltfs: usize,
}

#[derive(SystemParam)]
pub(crate) struct MemorySources<'w, 's> {
    entities: &'w Entities,
    device: Option<Res<'w, RenderDevice>>,
    images: Res<'w, Assets<Image>>,
    meshes: Res<'w, Assets<Mesh>>,
    materials: Res<'w, Assets<StandardMaterial>>,
    clips: Res<'w, Assets<AnimationClip>>,
    gltfs: Res<'w, Assets<Gltf>>,
    history: Local<'s, Growth>,
}

// Measured by the caller, from the process and the render world.
pub(crate) struct Measured {
    pub working_set_mb: Option<f32>,
    pub committed_mb: Option<f32>,
    pub render_entities: Option<usize>,
}

impl MemorySources<'_, '_> {
    pub(crate) fn read(&mut self, measured: Measured) -> Memory {
        let heap = heap::live();
        let gpu = self.device.as_deref().and_then(gpu_memory);
        let sample = [
            measured.working_set_mb,
            measured.committed_mb,
            heap.map(|(bytes, _)| (bytes as f64 / MB) as f32),
            gpu.as_ref().map(|(allocated, ..)| *allocated),
        ];
        let rates = self.history.push(sample);
        let amount = |index: usize| sample[index].map(|mb| Amount { mb, per_minute: rates[index] });

        Memory {
            working_set: amount(0),
            committed: amount(1),
            heap: amount(2),
            heap_blocks: heap.map_or(0, |(_, blocks)| blocks),
            gpu: gpu.map(|(_, reserved_mb, largest)| GpuMemory {
                allocated: amount(3).unwrap_or_default(),
                reserved_mb,
                largest,
            }),
            gpu_objects: self.device.as_deref().and_then(gpu_objects),
            entities: self.entities.count_spawned() as usize,
            render_entities: measured.render_entities,
            assets: AssetCounts {
                images: self.images.len(),
                meshes: self.meshes.len(),
                materials: self.materials.len(),
                clips: self.clips.len(),
                gltfs: self.gltfs.len(),
            },
        }
    }

    pub(crate) fn reset(&mut self) {
        self.history.0.clear();
    }
}

// Megabytes allocated, megabytes reserved, and the largest kinds.
type GpuReport = (f32, f32, Vec<(String, f32)>);

fn gpu_memory(device: &RenderDevice) -> Option<GpuReport> {
    let report = device.wgpu_device().generate_allocator_report()?;
    let mut by_name: HashMap<&str, u64> = HashMap::new();
    for allocation in &report.allocations {
        let name = if allocation.name.is_empty() { "unnamed" } else { &allocation.name };
        *by_name.entry(name).or_default() += allocation.size;
    }
    let mut largest: Vec<_> = by_name.into_iter().collect();
    largest.sort_unstable_by_key(|&(_, bytes)| Reverse(bytes));
    let largest = largest
        .into_iter()
        .take(LARGEST_SHOWN)
        .map(|(name, bytes)| (name.to_string(), (bytes as f64 / MB) as f32))
        .collect();
    let mb = |bytes: u64| (bytes as f64 / MB) as f32;
    Some((mb(report.total_allocated_bytes), mb(report.total_reserved_bytes), largest))
}

// wgpu only counts with its `counters` feature, and reads all zeros without it.
fn gpu_objects(device: &RenderDevice) -> Option<GpuObjects> {
    let hal = device.wgpu_device().get_internal_counters().hal;
    let objects = GpuObjects {
        buffers: hal.buffers.read(),
        textures: hal.textures.read(),
        texture_views: hal.texture_views.read(),
        bind_groups: hal.bind_groups.read(),
        samplers: hal.samplers.read(),
        pipelines: hal.render_pipelines.read() + hal.compute_pipelines.read(),
        shader_modules: hal.shader_modules.read(),
    };
    (objects.buffers > 0 || objects.textures > 0).then_some(objects)
}

// Readings of the last minute, for how fast each grows.
#[derive(Default)]
struct Growth(VecDeque<(Instant, [Option<f32>; 4])>);

impl Growth {
    fn push(&mut self, sample: [Option<f32>; 4]) -> [Option<f32>; 4] {
        let now = Instant::now();
        self.0.push_back((now, sample));
        while self.0.front().is_some_and(|(at, _)| now - *at > GROWTH_WINDOW) {
            self.0.pop_front();
        }
        let Some(&(since, first)) = self.0.front() else {
            return [None; 4];
        };
        let minutes = (now - since).as_secs_f32() / 60.0;
        if now - since < MIN_WINDOW {
            return [None; 4];
        }
        std::array::from_fn(|i| Some((sample[i]? - first[i]?) / minutes))
    }
}
