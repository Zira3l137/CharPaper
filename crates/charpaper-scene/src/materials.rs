use std::collections::HashMap;

use bevy::asset::AssetEventSystems;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::tasks::ComputeTaskPool;
use charpaper_bake::mipmaps;
use charpaper_bake::mipmaps::Layout;

// What the app does to a suite's materials and textures as they arrive, before the GPU sees
// them.
pub(crate) struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        // Bevy extracts an asset at the end of the frame it arrives in, and keeps a GPU-only
        // image's pixels only until then. So this runs that same frame, after the asset events
        // that tell what arrived.
        app.add_systems(PostUpdate, add_mipmaps.after(AssetEventSystems));
    }
}

fn add_mipmaps(
    mut events: MessageReader<AssetEvent<Image>>,
    assets: Res<AssetServer>,
    materials: Res<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let added: Vec<AssetId<Image>> = events
        .read()
        .filter_map(|event| match event {
            AssetEvent::Added { id } => Some(*id),
            _ => None,
        })
        .collect();
    if added.is_empty() {
        return;
    }

    let mut cutoffs: HashMap<AssetId<Image>, f32> = HashMap::new();
    for (_, material) in materials.iter() {
        if let (AlphaMode::Mask(cutoff), Some(texture)) =
            (material.alpha_mode, &material.base_color_texture)
        {
            // The cutoff applies to texture alpha times the material's own.
            let factor = material.base_color.alpha().max(f32::EPSILON);
            let cutoff = (cutoff / factor).min(1.0);
            cutoffs.entry(texture.id()).and_modify(|c| *c = c.min(cutoff)).or_insert(cutoff);
        }
    }

    let mut jobs = Vec::new();
    for id in added {
        // A suite's textures, and only those: they're loaded from a file for the GPU alone.
        // Images the app makes itself, like a GIF's frames, are rewritten later at one size.
        if assets.get_path(id).is_none() {
            continue;
        }
        let Some(image) = images.get_mut_untracked(id) else {
            continue;
        };
        let Some(layout) = layout(image) else {
            continue;
        };
        if let Some(data) = image.data.take() {
            jobs.push((id, data, layout, cutoffs.get(&id).copied()));
        }
    }

    let done = ComputeTaskPool::get().scope(|scope| {
        for (id, data, layout, cutoff) in jobs {
            scope.spawn(async move { (id, layout, mipmaps::with_mipmaps(data, layout, cutoff)) });
        }
    });
    for (id, layout, data) in done {
        if let Some(image) = images.get_mut_untracked(id) {
            image.data = Some(data);
            image.texture_descriptor.mip_level_count =
                mipmaps::level_count(layout.width, layout.height);
        }
    }
}

fn layout(image: &Image) -> Option<Layout> {
    let descriptor = &image.texture_descriptor;
    if image.asset_usage != RenderAssetUsages::RENDER_WORLD
        || descriptor.mip_level_count != 1
        || descriptor.dimension != TextureDimension::D2
        || descriptor.size.depth_or_array_layers != 1
    {
        return None;
    }
    let (channels, srgb) = match descriptor.format {
        TextureFormat::Rgba8UnormSrgb => (4, true),
        TextureFormat::Rgba8Unorm => (4, false),
        TextureFormat::Rg8Unorm => (2, false),
        TextureFormat::R8Unorm => (1, false),
        _ => return None,
    };
    Some(Layout { width: descriptor.size.width, height: descriptor.size.height, channels, srgb })
}
