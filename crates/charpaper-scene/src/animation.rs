//! Plays the suite's clips on the armature.

use bevy::animation::AnimatedBy;
use bevy::animation::AnimationTargetId;
use bevy::prelude::*;

use crate::binding::InstanceReady;
use crate::character::Armature;

#[derive(Component)]
pub(crate) struct Animatable;

/// Bevy's glTF loader only marks a node as an animation target when a clip in
/// the same file moves it, and the model holds no clips. So the armature is
/// marked here by the loader's own rule: a node's id is the hash of the names
/// from its top-level node down to itself. Clips from other files were baked
/// with ids made the same way, which is why their names must match exactly.
///
/// The player lives on the armature's root entity rather than on a glTF node.
/// Ids only encode names, so the player can sit anywhere, and one player then
/// covers a model with several top-level nodes.
pub(crate) fn make_armature_animatable(
    mut commands: Commands,
    armature: Query<Entity, (With<Armature>, With<InstanceReady>, Without<Animatable>)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok(armature) = armature.single() else {
        return;
    };

    // armature root -> glTF scene root(s) -> top-level nodes
    let mut pending: Vec<(Entity, Vec<&str>)> = Vec::new();
    for &scene_root in kids(&children, armature) {
        pending.extend(kids(&children, scene_root).iter().map(|&node| (node, Vec::new())));
    }

    let mut targets = 0;
    while let Some((node, mut path)) = pending.pop() {
        let Ok(name) = names.get(node) else {
            continue;
        };
        path.push(name.as_str());
        commands
            .entity(node)
            .insert((AnimationTargetId::from_iter(path.iter()), AnimatedBy(armature)));
        targets += 1;
        pending.extend(kids(&children, node).iter().map(|&child| (child, path.clone())));
    }

    commands.entity(armature).insert((
        Animatable,
        AnimationPlayer::default(),
        AnimationTransitions::new(),
    ));
    info!("armature: {targets} node(s) ready to animate");
}

fn kids<'a>(children: &'a Query<&Children>, entity: Entity) -> &'a [Entity] {
    children.get(entity).map(|c| &**c).unwrap_or_default()
}
