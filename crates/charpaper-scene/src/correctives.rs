//! Correctives: shape keys a body animation keys along with the bones, such
//! as a skirt that follows the legs through a jump.
//!
//! They arrive in the body clip itself, targeting the mesh by its name path
//! like any bone. So every mesh of the worn skin with shape keys, except the
//! ones its expressions own, is made a target of the armature's player: the
//! jump then drives the skirt on the same clock, crossfades included. A skin
//! without that mesh simply leaves those keys with nothing to move.

use bevy::animation::AnimatedBy;
use bevy::animation::AnimationTargetId;
use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;

use crate::animation::kids;
use crate::binding::InstanceReady;
use crate::character::Armature;
use crate::character::SkinRoot;
use crate::expression::SkinClips;

#[derive(Component)]
pub(crate) struct CorrectivesBound;

/// Runs before binding, which moves rigid parts elsewhere in the hierarchy:
/// the paths must be the ones the skin's file had, as in the animation file.
pub(crate) fn bind_correctives(
    mut commands: Commands,
    skins: Query<(Entity, &SkinRoot, &SkinClips), (With<InstanceReady>, Without<CorrectivesBound>)>,
    armature: Query<Entity, With<Armature>>,
    children: Query<&Children>,
    names: Query<&Name>,
    morphs: Query<(), With<MorphWeights>>,
) {
    let Ok(armature) = armature.single() else {
        return;
    };
    for (root, skin, clips) in &skins {
        // skin root -> glTF scene root(s) -> top-level nodes, as for the armature
        let mut pending: Vec<(Entity, Vec<&str>)> = Vec::new();
        for &scene_root in kids(&children, root) {
            pending.extend(kids(&children, scene_root).iter().map(|&node| (node, Vec::new())));
        }

        let mut bound = 0;
        while let Some((node, mut path)) = pending.pop() {
            let Ok(name) = names.get(node) else {
                continue;
            };
            path.push(name.as_str());
            if morphs.contains(node) && !clips.expression_meshes.contains(name.as_str()) {
                commands
                    .entity(node)
                    .insert((AnimationTargetId::from_iter(path.iter()), AnimatedBy(armature)));
                bound += 1;
            }
            pending.extend(kids(&children, node).iter().map(|&child| (child, path.clone())));
        }

        commands.entity(root).insert(CorrectivesBound);
        if bound > 0 {
            info!("skin {:?}: {bound} mesh(es) take correctives from body animations", skin.name);
        }
    }
}
