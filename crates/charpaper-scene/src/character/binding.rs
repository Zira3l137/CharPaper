// Every glTF file brings its own copy of the bones, and only the armature's copy is
// animated. So each skin's meshes are re-pointed at the armature's bones by name, and
// anything else hanging from a bone (a hat on Head, an extra ponytail chain) is moved
// onto the armature's copy of that bone.

use std::collections::HashMap;

use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::character::Armature;
use crate::character::skin::SkinRoot;

#[derive(Component)]
pub(crate) struct InstanceReady;

#[derive(Component)]
pub(crate) struct Bound;

#[derive(Component)]
pub(crate) struct SkinPart(pub Entity);

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
                // Each primitive spawns under its glTF node, so search from above the node: a mesh
                // object named after a bone must not match that bone.
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

        for (&branch, &bone) in &branches {
            commands.entity(branch).insert((ChildOf(bone), SkinPart(skin_root)));
        }
        commands.entity(skin_root).insert(Bound);

        info!(
            "skin {:?}: {rebound} joint(s) bound to the armature, {} branch(es) moved onto it",
            skin.name,
            branches.len()
        );
    }
}

// The entity just below the nearest ancestor the armature also has, and the armature's
// copy of that ancestor. Moving the whole branch keeps every transform along the way.
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
