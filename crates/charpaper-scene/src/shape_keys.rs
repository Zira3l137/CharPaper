use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::VisibilitySystems;
use bevy::light::SimulationLightSystems;
use bevy::mesh::morph::MeshMorphWeights;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;

// Works around a bug in Bevy 0.19.1, still there in 0.20.0, that leaves a mesh's shape keys
// frozen from the moment it comes into view until something about the mesh changes.
//
// The render world finds a mesh's shape key weights on the GPU through a slot it gives the
// mesh in prepare_morph_descriptors (bevy_pbr/src/render/morph.rs), and only gives one to a
// mesh it already knows. It learns of a mesh coming into view later that same frame, in
// collect_meshes_for_gpu_building, so it writes that mesh down with no slot, and writes it
// again only when the mesh moves, changes or leaves view. A skinned mesh gets away with it:
// its bounds follow its bones, so it changes every frame. Other meshes with shape keys stay
// frozen, depending on when each came into view, which looks random from outside.
//
// Marking the bounds of every such mesh changed each frame it is in view has the render world
// write it down again, by then with its slot. It costs about what a skinned mesh costs anyway.
// Check whether this is still needed on every Bevy upgrade.
pub(crate) struct ShapeKeysPlugin;

impl Plugin for ShapeKeysPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            rewrite_shape_keyed
                .after(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible)
                .after(SimulationLightSystems::CheckLightVisibility),
        );
    }
}

fn rewrite_shape_keyed(
    mut meshes: Query<(&ViewVisibility, &mut Aabb), (With<MeshMorphWeights>, Without<SkinnedMesh>)>,
) {
    for (visibility, mut bounds) in &mut meshes {
        if visibility.get() {
            bounds.set_changed();
        }
    }
}
