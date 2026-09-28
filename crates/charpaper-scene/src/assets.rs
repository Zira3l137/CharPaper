use std::path::Path;

use bevy::asset::AssetPath;
use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfLoaderSettings;
use bevy::prelude::*;
use charpaper_suite::Suite;

// Suite files load as `characters://<suite folder>/<file>`. The app registers this
// source over its characters folder.
pub const CHARACTERS_SOURCE: &str = "characters";

// A second source over the same folder, only for the armature. Bevy keeps one asset per
// path and ignores the settings of later loads, so an armature borrowed from a skin file is
// read under another path to leave that skin's meshes out.
pub const ARMATURE_SOURCE: &str = "armature";

pub(crate) fn asset_path(suite: &Suite, file: &Path) -> AssetPath<'static> {
    let folder = suite.root.file_name().unwrap_or_default();
    AssetPath::from_path_buf(Path::new(folder).join(file)).with_source(CHARACTERS_SOURCE)
}

// Once uploaded, Bevy drops the RAM copy of meshes and textures. Bounding boxes survive
// but shape key names don't; expressions read those from the file instead.
pub(crate) const GPU_ONLY: RenderAssetUsages = RenderAssetUsages::RENDER_WORLD;

pub(crate) fn load_gltf(assets: &AssetServer, suite: &Suite, file: &Path) -> Handle<Gltf> {
    assets
        .load_builder()
        .with_settings(|s: &mut GltfLoaderSettings| {
            s.load_cameras = false;
            s.load_lights = false;
            s.load_meshes = GPU_ONLY;
            s.load_materials = GPU_ONLY;
        })
        .load(asset_path(suite, file))
}

// Scene 0, not the default scene: Bevy has no label for the default one, and Blender
// exports the active scene as scene 0. An armature borrowed from a skin skips that skin's
// meshes, materials and clips, which the skin shows itself when worn.
pub(crate) fn load_armature(assets: &AssetServer, suite: &Suite) -> Handle<WorldAsset> {
    let borrowed = suite.model_is_skin;
    let path = asset_path(suite, &suite.model).with_source(ARMATURE_SOURCE);
    assets
        .load_builder()
        .with_settings(move |s: &mut GltfLoaderSettings| {
            s.load_cameras = false;
            s.load_lights = false;
            if borrowed {
                s.load_meshes = RenderAssetUsages::empty();
                s.load_materials = RenderAssetUsages::empty();
                s.load_animations = false;
            } else {
                s.load_meshes = GPU_ONLY;
                s.load_materials = GPU_ONLY;
            }
        })
        .load(GltfAssetLabel::Scene(0).from_asset(path))
}
