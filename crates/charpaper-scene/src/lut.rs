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
use crate::render::SceneTarget;
use crate::suite::ActiveSuite;

pub(crate) struct LutPlugin;

impl Plugin for LutPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(CubeLoader)
            .init_resource::<ShownLut>()
            .add_systems(Update, apply_lut.in_set(SceneSet::Run));
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
#[derive(Resource, Default)]
pub(crate) struct ShownLut {
    path: Option<AssetPath<'static>>,
    loading: Option<Handle<Image>>,
}

// Every frame, since it has to notice a load finishing. Cheap when nothing changes.
fn apply_lut(
    look: Option<Res<ActiveLook>>,
    suite: Option<Res<ActiveSuite>>,
    target: Option<Res<SceneTarget>>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownLut>,
    mut materials: ResMut<Assets<SceneImage>>,
) {
    let Some(target) = target else {
        return;
    };
    let lut = look.map(|l| l.post.lut.clone()).unwrap_or_default();
    let wanted = suite.as_ref().and_then(|suite| {
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
            _ => return,
        },
    };
    let strength = if ready.is_some() { lut.strength() } else { 0.0 };
    if materials
        .get(&target.material)
        .is_some_and(|m| m.lut != ready || m.settings.lut_strength != strength)
        && let Some(mut material) = materials.get_mut(&target.material)
    {
        material.lut = ready;
        material.settings.lut_strength = strength;
    }
}
