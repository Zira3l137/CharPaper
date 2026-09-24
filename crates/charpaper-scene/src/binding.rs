//! Moves each skin onto the armature once both have spawned.
//!
//! Every glTF file brings its own copy of the bones, and only the armature's
//! copy will ever be animated. So once a skin has spawned:
//! - its skinned meshes are re-pointed at the armature's bones of the same
//!   name, which `--check-suite` has already verified share a rest pose;
//! - whatever else must follow the body is re-parented from the skin's copy of
//!   a bone onto the armature's. That covers rigid meshes parented to a bone
//!   (a hat on `Head`) and extra bones the armature lacks (a ponytail chain),
//!   which then ride along unanimated.
//!
//! The skin's own bone copies stay where they are. Nothing points at them any
//! more and they hold nothing visible, while despawning them would take along
//! any branch not yet moved.

use std::collections::HashMap;

use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::character::Armature;
use crate::character::CharacterState;
use crate::character::SkinRoot;
use crate::character::visibility_for;

#[derive(Component)]
pub(crate) struct InstanceReady;

#[derive(Component)]
pub(crate) struct Bound;

/// A branch moved out of a skin's hierarchy. It no longer inherits the skin
/// root's visibility, so it points back at its skin to be hidden with it.
#[derive(Component)]
pub(crate) struct SkinPart(pub Entity);

/// The armature and skins spawn asynchronously and in any order, so readiness
/// is recorded here and [`bind_skins`] acts once both sides have it.
pub(crate) fn mark_ready(
    ready: On<WorldInstanceReady>,
    roots: Query<(), Or<(With<Armature>, With<SkinRoot>)>>,
    mut commands: Commands,
) {
    if roots.contains(ready.entity) {
        commands.entity(ready.entity).insert(InstanceReady);
    }
}

pub(crate) fn bind_skins(
    mut commands: Commands,
    armature: Query<Entity, (With<Armature>, With<InstanceReady>)>,
    skins: Query<(Entity, &SkinRoot), (With<InstanceReady>, Without<Bound>)>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    meshes: Query<(), With<Mesh3d>>,
    mut skinned: Query<&mut SkinnedMesh>,
    state: Res<CharacterState>,
) {
    let Ok(armature) = armature.single() else {
        return;
    };
    if skins.is_empty() {
        return;
    }

    let bones: HashMap<&str, Entity> = children
        .iter_descendants(armature)
        .filter(|&e| !meshes.contains(e))
        .filter_map(|e| names.get(e).ok().map(|name| (name.as_str(), e)))
        .collect();

    for (skin_root, skin) in &skins {
        let mut rebound = 0;
        let mut branches: HashMap<Entity, Entity> = HashMap::new();

        for entity in children.iter_descendants(skin_root) {
            if let Ok(mut mesh) = skinned.get_mut(entity) {
                for joint in mesh.joints.iter_mut() {
                    let name = names.get(*joint).map(Name::as_str).unwrap_or_default();
                    if let Some(&bone) = bones.get(name) {
                        *joint = bone;
                        rebound += 1;
                    } else if let Some((branch, bone)) =
                        branch_point(*joint, skin_root, &parents, &names, &bones)
                    {
                        branches.insert(branch, bone);
                    } else {
                        warn!("skin {:?}: bone {name:?} has nowhere to attach", skin.name);
                    }
                }
            } else if meshes.contains(entity) {
                // Bevy spawns each primitive as a child of its glTF node, so
                // the search starts above the node: a mesh object named after
                // a bone ("Head") must not match that bone itself.
                let Ok(node) = parents.get(entity).map(ChildOf::parent) else {
                    continue;
                };
                if let Some((branch, bone)) =
                    branch_point(node, skin_root, &parents, &names, &bones)
                {
                    branches.insert(branch, bone);
                }
            }
        }

        let visibility = visibility_for(state.skin.as_deref() == Some(skin.name.as_str()));
        for (&branch, &bone) in &branches {
            commands.entity(branch).insert((ChildOf(bone), SkinPart(skin_root), visibility));
        }
        commands.entity(skin_root).insert(Bound);

        info!(
            "skin {:?}: {rebound} joint(s) bound to the armature, {} branch(es) moved onto it",
            skin.name,
            branches.len()
        );
    }
}

/// Walks up from `start` to the nearest ancestor whose name the armature also
/// has. Returns the entity hanging directly below that ancestor, which is the
/// branch to move, and the armature's copy of the ancestor, its new parent.
/// Moving the whole branch keeps every transform along the way intact.
fn branch_point(
    start: Entity,
    skin_root: Entity,
    parents: &Query<&ChildOf>,
    names: &Query<&Name>,
    bones: &HashMap<&str, Entity>,
) -> Option<(Entity, Entity)> {
    let mut child = start;
    for ancestor in parents.iter_ancestors(start) {
        if ancestor == skin_root {
            return None;
        }
        let bone = names.get(ancestor).ok().and_then(|name| bones.get(name.as_str()));
        if let Some(&bone) = bone {
            return Some((child, bone));
        }
        child = ancestor;
    }
    None
}
