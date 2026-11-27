use std::collections::HashMap;

use bevy::asset::embedded_asset;
use bevy::gltf::GltfMaterialName;
use bevy::pbr::ExtendedMaterial;
use bevy::pbr::MaterialExtension;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use charpaper_suite::Suite;

use crate::suite::ActiveSuite;

const SHADER: &str = "embedded://charpaper_scene/dither/dither.wgsl";

// See-through materials drawn the way EEVEE's Dithered render method draws them (dither.wgsl).
// Blender's default, so a material shows in the app as it does in the viewport; one set to
// Blended keeps Bevy's blending.
pub(crate) type DitheredMaterial = ExtendedMaterial<StandardMaterial, Dithered>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub(crate) struct Dithered {}

impl MaterialExtension for Dithered {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

pub(crate) struct DitherPlugin;

impl Plugin for DitherPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "dither.wgsl");
        app.add_plugins(MaterialPlugin::<DitheredMaterial>::default())
            .init_resource::<Dithers>()
            .add_systems(PostUpdate, dither_see_through);
    }
}

// The dithered copy of each see-through material, made once and shared by every mesh wearing
// it, as the original was.
#[derive(Resource, Default)]
struct Dithers(HashMap<AssetId<StandardMaterial>, Handle<DitheredMaterial>>);

fn dither_see_through(
    mut commands: Commands,
    worn: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>, Option<&GltfMaterialName>),
        Added<MeshMaterial3d<StandardMaterial>>,
    >,
    parents: Query<&ChildOf>,
    suites: Query<&ActiveSuite>,
    materials: Res<Assets<StandardMaterial>>,
    mut dithered: ResMut<Assets<DitheredMaterial>>,
    mut made: ResMut<Dithers>,
) {
    made.0.retain(|id, _| materials.contains(*id));
    for (entity, worn, name) in &worn {
        let Some(material) = materials.get(&worn.0) else {
            continue;
        };
        if material.alpha_mode != AlphaMode::Blend {
            continue;
        }
        // A suite that isn't on a stage, as thumbnails load one, keeps blending: a still
        // picture has no TAA to smooth the dither.
        let Some(suite) = parents.iter_ancestors(entity).find_map(|e| suites.get(e).ok()) else {
            continue;
        };
        if name.is_some_and(|name| blends(&suite.0, &name.0)) {
            continue;
        }
        let handle = made
            .0
            .entry(worn.0.id())
            .or_insert_with(|| {
                // The cutoff is never reached; the shader discards by the dither instead.
                let base =
                    StandardMaterial { alpha_mode: AlphaMode::Mask(0.0), ..material.clone() };
                dithered.add(ExtendedMaterial { base, extension: Dithered {} })
            })
            .clone();
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}

// A GIF plays on the StandardMaterial it finds on the screen's meshes, so its material stays one.
fn blends(suite: &Suite, material: &str) -> bool {
    suite.blended.contains(material)
        || suite.environments.iter().any(|environment| environment.animated.contains_key(material))
}
