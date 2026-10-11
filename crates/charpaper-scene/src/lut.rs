// Color lookup tables from the suite's luts/ folder. A .cube file loads in the background as
// a 3D texture, and the one the look picks reaches the scene image only once loaded, so a
// switch keeps the previous grade on screen instead of flashing an ungraded frame.

use std::io;

use bevy::asset::AssetLoader;
use bevy::asset::AssetPath;
use bevy::asset::LoadContext;
use bevy::asset::LoadState;
use bevy::asset::RenderAssetUsages;
use bevy::asset::io::Reader;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use charpaper_suite::CubeLut;
use charpaper_suite::parse_cube;

use crate::SceneSet;
use crate::assets::asset_path;
use crate::look::ActiveLook;
use crate::render::SceneImage;
use crate::render::ScreenParts;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

pub(crate) struct LutPlugin;

impl Plugin for LutPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(CubeLoader).add_systems(Update, apply_lut.in_set(SceneSet::Run));
    }
}

#[derive(TypePath)]
struct CubeLoader;

impl AssetLoader for CubeLoader {
    type Asset = Image;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Image, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
        let text = String::from_utf8(bytes).map_err(|e| invalid(e.to_string()))?;
        let lut = parse_cube(&text).map_err(invalid)?;
        Ok(lut_image(&lut))
    }

    fn extensions(&self) -> &[&str] {
        &["cube"]
    }
}

// The cube's entries in their file order, red fastest, are a 3D texture's x, y, z order.
fn lut_image(lut: &CubeLut) -> Image {
    let size = lut.size as u32;
    let data =
        lut.data.iter().flat_map(|&[r, g, b]| [r, g, b, 1.0]).flat_map(f32::to_le_bytes).collect();
    Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: size },
        TextureDimension::D3,
        data,
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD,
    )
}

// By path rather than name: two suites can each have a LUT of the same name.
#[derive(Component, Default)]
pub(crate) struct ShownLut {
    path: Option<AssetPath<'static>>,
    loading: Option<Handle<Image>>,
}

// Every frame, since it has to notice a load finishing. Cheap when nothing changes.
fn apply_lut(
    mut stages: Query<(Entity, Option<&ActiveLook>, Option<&ActiveSuite>, &mut ShownLut)>,
    screens: Query<&ScreenParts>,
    stage_of: StageOf,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<SceneImage>>,
) {
    for (stage, look, suite, mut shown) in &mut stages {
        let Some((lut, strength)) = loaded_lut(look, suite, &assets, &mut shown) else {
            continue;
        };
        for parts in &screens {
            if stage_of.get(parts.camera) != Some(stage) {
                continue;
            }
            if materials
                .get(&parts.material)
                .is_some_and(|m| m.lut != lut || m.settings.lut_strength != strength)
                && let Some(mut material) = materials.get_mut(&parts.material)
            {
                material.lut = lut.clone();
                material.settings.lut_strength = strength;
            }
        }
    }
}

// The LUT the stage's look picks and how strongly, or None while it is still loading.
fn loaded_lut(
    look: Option<&ActiveLook>,
    suite: Option<&ActiveSuite>,
    assets: &AssetServer,
    shown: &mut ShownLut,
) -> Option<(Option<Handle<Image>>, f32)> {
    let lut = look.map(|l| l.post.lut.clone()).unwrap_or_default();
    let wanted = suite.and_then(|suite| {
        let name = lut.name()?;
        let file = suite.luts.iter().find(|l| l.name == name)?;
        Some(asset_path(suite, &file.file))
    });
    if shown.path != wanted {
        shown.loading = wanted.clone().map(|path| assets.load(path));
        shown.path = wanted;
    }

    let ready = match &shown.loading {
        None => None,
        Some(handle) => match assets.load_state(handle.id()) {
            LoadState::Loaded => Some(handle.clone()),
            LoadState::Failed(err) => {
                warn!("cannot load the LUT {:?}: {err}", shown.path);
                shown.loading = None;
                None
            }
            _ => return None,
        },
    };
    let strength = if ready.is_some() { lut.strength() } else { 0.0 };
    Some((ready, strength))
}
