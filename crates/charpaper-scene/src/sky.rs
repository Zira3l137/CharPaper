// The sky behind the scene, blurred apart from the light it casts. Bevy's skybox always draws
// the sharpest level of its cubemap, so a blurred background is one level of the reflection
// map, which the baker already blurred for rougher surfaces, copied into a cubemap of its own.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::TextureFormatPixelInfo;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDataOrder;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureViewDescriptor;
use bevy::render::render_resource::TextureViewDimension;
use charpaper_suite::BackgroundBlur;
use charpaper_suite::Look;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::environment::ShownEnvironment;
use crate::look::ActiveLook;
use crate::look::Resolved;
use crate::stage::StageOf;

pub(crate) struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, blur_background.in_set(SceneSet::Run));
    }
}

// The environment's own sky, kept for when the background isn't blurred.
#[derive(Component)]
pub(crate) struct SharpSky(pub Handle<Image>);

// By the reflection map and its level. Each lives as long as its reflection map.
#[derive(Default)]
struct BlurredSkies(HashMap<(AssetId<Image>, u32), Handle<Image>>);

// Every frame, since the reflection map may finish loading or baking at any time. It only
// writes the skybox when its image should change.
fn blur_background(
    stages: Query<(Option<&ActiveLook>, &ShownEnvironment)>,
    mut cameras: Query<
        (Entity, &mut Skybox, &SharpSky, Option<&EnvironmentMapLight>),
        With<SceneCamera>,
    >,
    stage_of: StageOf,
    mut images: ResMut<Assets<Image>>,
    mut blurred: Local<BlurredSkies>,
) {
    blurred.0.retain(|(source, _), _| images.contains(*source));
    for (camera, mut skybox, sharp, light) in &mut cameras {
        let Some((look, shown)) = stage_of.get(camera).and_then(|s| stages.get(s).ok()) else {
            continue;
        };
        let look = look.map_or_else(Look::default, |l| l.0.clone());
        let blur = Resolved::new(&look, shown.name.as_deref()).background_blur;
        let wanted = light
            .and_then(|light| blurred_sky(&mut images, &mut blurred, &light.specular_map, blur))
            .unwrap_or_else(|| sharp.0.clone());
        if skybox.image.as_ref() != Some(&wanted) {
            skybox.image = Some(wanted);
        }
    }
}

// The reflection map's level i of n is what perceptual roughness i / (n - 1) sees.
fn blurred_sky(
    images: &mut Assets<Image>,
    blurred: &mut BlurredSkies,
    reflections: &Handle<Image>,
    blur: BackgroundBlur,
) -> Option<Handle<Image>> {
    let roughness = match blur {
        BackgroundBlur::Off => return None,
        BackgroundBlur::Low => 0.25,
        BackgroundBlur::Medium => 0.375,
        BackgroundBlur::High => 0.5,
    };
    let source = images.get(reflections)?;
    let last = source.texture_descriptor.mip_level_count.saturating_sub(1);
    let level = ((roughness * last as f32).round() as u32).clamp(1.min(last), last);
    let key = (reflections.id(), level);
    if let Some(handle) = blurred.0.get(&key) {
        return Some(handle.clone());
    }
    let image = cube_level(source, level)?;
    let handle = images.add(image);
    blurred.0.insert(key, handle.clone());
    Some(handle)
}

// One mip level of a cubemap as a cubemap of its own. None for a compressed format or one
// whose pixels aren't kept on the CPU.
fn cube_level(cube: &Image, level: u32) -> Option<Image> {
    let descriptor = &cube.texture_descriptor;
    let format = descriptor.format;
    let texel = format.pixel_size().ok()?;
    let data = cube.data.as_ref()?;
    let layers = descriptor.size.depth_or_array_layers as usize;
    let mips = descriptor.mip_level_count;
    let side = |l: u32| (descriptor.size.width >> l).max(1);
    let face = |l: u32| side(l) as usize * side(l) as usize * texel;
    let below = |l: u32| (0..l).map(face).sum::<usize>();

    let mut bytes = Vec::with_capacity(face(level) * layers);
    match cube.data_order {
        TextureDataOrder::MipMajor => {
            let start = below(level) * layers;
            bytes.extend_from_slice(data.get(start..start + face(level) * layers)?);
        }
        TextureDataOrder::LayerMajor => {
            let per_layer = below(mips);
            for layer in 0..layers {
                let start = layer * per_layer + below(level);
                bytes.extend_from_slice(data.get(start..start + face(level))?);
            }
        }
    }

    let size =
        Extent3d { width: side(level), height: side(level), depth_or_array_layers: layers as u32 };
    let mut image =
        Image::new(size, TextureDimension::D2, bytes, format, RenderAssetUsages::RENDER_WORLD);
    image.sampler = cube.sampler.clone();
    image.texture_view_descriptor =
        Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::Cube), ..default() });
    Some(image)
}
