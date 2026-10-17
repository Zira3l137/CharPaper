// Point and spot light shadow maps are drawn again only where something in them changed.
//
// Bevy draws every face of every point light, and every spot light, into its shadow map each
// frame, as a view of its own, and each costs the render thread a fixed amount whatever it
// shows. Most show the same thing every frame: a lamp on a wall and the walls around it. The
// shadow maps are layers of textures Bevy takes from its texture cache, the same textures
// every frame while their size and count stay the same, and a layer keeps what was drawn into
// it until it is drawn again. So a face that would come out the same is skipped, and its
// layer keeps last frame's map.
//
// A face is drawn when
// - a mesh in it moved, or is skinned or has shape keys, since those change every frame;
// - the meshes in it changed, or its light moved or changed how it casts shadows;
// - it is new, or its light's layer moved, or Bevy made new shadow textures;
// - it is among the few that went longest without being drawn, so a change none of this
//   catches is never stuck for more than about a second.
//
// It relies on how Bevy 0.19.1 makes these views and textures (prepare_lights in
// bevy_pbr/src/render/light.rs); check it again on every Bevy upgrade.

use std::collections::HashMap;

use bevy::camera::visibility::CubemapVisibleEntities;
use bevy::camera::visibility::VisibleMeshEntities;
use bevy::core_pipeline::schedule::RootNonCameraView;
use bevy::ecs::query::QueryItem;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::light::SimulationLightSystems;
use bevy::mesh::morph::MeshMorphWeights;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::pbr::LightEntity;
use bevy::pbr::ShadowView;
use bevy::pbr::ViewShadowBindings;
use bevy::pbr::prepare_lights;
use bevy::prelude::*;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderSystems;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::occlusion_culling::OcclusionCulling;
use bevy::render::render_resource::TextureId;
use bevy::render::sync_component::SyncComponent;

// Faces drawn each frame beyond those that changed. 63 faces are all drawn again in about
// 30 frames.
const REFRESHED_PER_FRAME: usize = 2;

pub(crate) struct ShadowCachePlugin;

impl Plugin for ShadowCachePlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<PointLight, ShadowWatch>()
            .register_required_components::<SpotLight, ShadowWatch>()
            .add_plugins(ExtractComponentPlugin::<StaleFaces>::default())
            .add_systems(
                PostUpdate,
                find_stale_faces.after(SimulationLightSystems::CheckLightVisibility),
            );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<DrawnFaces>()
            .add_schedule(Schedule::new(UnchangedShadowMap))
            .add_systems(
                Render,
                skip_unchanged_faces.after(prepare_lights).before(RenderSystems::Render),
            );
    }
}

// The faces of a light whose shadow maps must be drawn this frame, one bit each. A spot light
// has one.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct StaleFaces(u8);

impl SyncComponent for StaleFaces {
    type Target = Self;
}

impl ExtractComponent for StaleFaces {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(stale: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(*stale)
    }
}

// What a light's shadow maps were drawn from last frame.
#[derive(Component, Default)]
#[require(StaleFaces)]
struct ShadowWatch {
    transform: Option<GlobalTransform>,
    casting: Option<Casting>,
    meshes: [u64; 6],
}

// What decides how a light's shadow maps come out, besides where it is and what it sees.
#[derive(Clone, Copy, PartialEq)]
struct Casting {
    enabled: bool,
    depth_bias: f32,
    normal_bias: f32,
    near: f32,
    // A spot light's cone; none for a point light.
    angle: f32,
}

type Meshes<'w, 's> = Query<
    'w,
    's,
    (Ref<'static, GlobalTransform>, Ref<'static, Mesh3d>, Has<SkinnedMesh>, Has<MeshMorphWeights>),
>;

fn find_stale_faces(
    mut points: Query<(
        &PointLight,
        &GlobalTransform,
        &CubemapVisibleEntities,
        &mut ShadowWatch,
        &mut StaleFaces,
    )>,
    mut spots: Query<
        (&SpotLight, &GlobalTransform, &VisibleMeshEntities, &mut ShadowWatch, &mut StaleFaces),
        Without<PointLight>,
    >,
    meshes: Meshes,
) {
    for (light, transform, faces, mut watch, mut stale) in &mut points {
        let casting = Casting {
            enabled: light.shadow_maps_enabled,
            depth_bias: light.shadow_depth_bias,
            normal_bias: light.shadow_normal_bias,
            near: light.shadow_map_near_z,
            angle: 0.0,
        };
        let lists = faces.iter().map(|face| face.entities.as_slice());
        stale.0 = watch.update(transform, casting, lists, &meshes);
    }
    for (light, transform, visible, mut watch, mut stale) in &mut spots {
        let casting = Casting {
            enabled: light.shadow_maps_enabled,
            depth_bias: light.shadow_depth_bias,
            normal_bias: light.shadow_normal_bias,
            near: light.shadow_map_near_z,
            angle: light.outer_angle,
        };
        let lists = std::iter::once(visible.entities.as_slice());
        stale.0 = watch.update(transform, casting, lists, &meshes);
    }
}

impl ShadowWatch {
    // Returns the faces to draw, and remembers what they are drawn from.
    fn update<'a>(
        &mut self,
        transform: &GlobalTransform,
        casting: Casting,
        faces: impl Iterator<Item = &'a [Entity]>,
        meshes: &Meshes,
    ) -> u8 {
        let light_changed = self.transform != Some(*transform) || self.casting != Some(casting);
        self.transform = Some(*transform);
        self.casting = Some(casting);

        let mut stale = 0;
        for (index, entities) in faces.enumerate() {
            let listed = list_hash(entities);
            let moved = entities.iter().any(|&entity| {
                meshes.get(entity).is_ok_and(|(transform, mesh, skinned, morphs)| {
                    transform.is_changed() || mesh.is_changed() || skinned || morphs
                })
            });
            if light_changed || moved || self.meshes[index] != listed {
                stale |= 1 << index;
            }
            self.meshes[index] = listed;
        }
        stale
    }
}

// Bevy keeps each list sorted, so the same meshes give the same hash.
fn list_hash(entities: &[Entity]) -> u64 {
    entities.iter().fold(entities.len() as u64, |hash, entity| {
        (hash ^ entity.to_bits()).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

// Runs nothing, for faces left as they are.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UnchangedShadowMap;

// When each face was last drawn, by its light's render entity and its face, and which layer
// each light's maps are in.
#[derive(Resource, Default)]
struct DrawnFaces {
    frame: u64,
    textures: Option<[TextureId; 2]>,
    layers: HashMap<Entity, u32>,
    drawn: HashMap<(Entity, u8), u64>,
}

// Bevy makes these views anew every frame, each set to draw. Views with occlusion culling are
// left alone; CharPaper uses none.
fn skip_unchanged_faces(
    mut faces: ResMut<DrawnFaces>,
    bindings: Query<&ViewShadowBindings>,
    stale: Query<&StaleFaces>,
    mut views: Query<
        (&ShadowView, &LightEntity, &mut RootNonCameraView),
        Without<OcclusionCulling>,
    >,
) {
    // No camera to draw shadows for, or shadow textures not made yet: Bevy draws as it would.
    let Some(textures) = bindings
        .iter()
        .next()
        .map(|b| [b.point_light_depth_texture.id(), b.directional_light_depth_texture.id()])
    else {
        return;
    };
    let faces = &mut *faces;
    faces.frame += 1;
    // New textures hold nothing yet.
    if faces.textures != Some(textures) {
        faces.textures = Some(textures);
        faces.drawn.clear();
    }

    let mut layers = HashMap::new();
    let mut candidates = Vec::new();
    for (index, (view, light, _)) in views.iter().enumerate() {
        let (light, face) = match *light {
            LightEntity::Point { light_entity, face_index } => (light_entity, face_index as u8),
            LightEntity::Spot { light_entity } => (light_entity, 0),
            LightEntity::Directional { .. } => continue,
        };
        let layer = layer_of(view);
        if let Some(layer) = layer {
            layers.insert(light, layer);
        }
        let moved = layer.is_none() || faces.layers.get(&light).is_some_and(|&l| Some(l) != layer);
        let changed = stale.get(light).map_or(true, |stale| stale.0 & (1 << face) != 0);
        let last = faces.drawn.get(&(light, face)).copied().filter(|_| !moved);
        candidates.push((index, (light, face), last, changed));
    }

    // The ones to draw: changed or never drawn, then the longest waiting of the rest.
    let mut waiting: Vec<_> = candidates
        .iter()
        .filter(|&&(_, _, _, changed)| !changed)
        .filter_map(|&(index, _, last, _)| Some((last?, index)))
        .collect();
    waiting.sort_unstable();
    let refreshed: Vec<usize> =
        waiting.iter().take(REFRESHED_PER_FRAME).map(|&(_, index)| index).collect();

    let mut drawn = HashMap::with_capacity(candidates.len());
    let mut skip = Vec::new();
    for (index, key, last, changed) in candidates {
        if changed || last.is_none() || refreshed.contains(&index) {
            drawn.insert(key, faces.frame);
        } else if let Some(last) = last {
            drawn.insert(key, last);
            skip.push(index);
        }
    }
    faces.drawn = drawn;
    faces.layers = layers;

    for (index, (_, _, mut root)) in views.iter_mut().enumerate() {
        if skip.contains(&index) {
            root.0 = UnchangedShadowMap.intern();
        }
    }
}

// Which layer of the shadow textures a light's maps are in. Bevy only says so in the pass's
// name, `shadow_point_light_<layer>_<face>` or `shadow_spot_light_<layer>`. A name that reads
// otherwise means the face is drawn every frame, as Bevy would.
fn layer_of(view: &ShadowView) -> Option<u32> {
    let rest = view
        .pass_name
        .strip_prefix("shadow_point_light_")
        .or_else(|| view.pass_name.strip_prefix("shadow_spot_light_"))?;
    rest.split('_').next()?.parse().ok()
}
