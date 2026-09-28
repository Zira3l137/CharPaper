// Correctives are shape keys a body animation keys along with the bones, like a skirt
// following the legs. Every shape-keyed mesh of the worn skin, except the ones its
// expressions own, becomes a target of the armature's player.

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

// Runs before binding moves branches: target ids hash the node paths as they are in the file.
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
