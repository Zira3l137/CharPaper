// The suite's spring chains, run on the armature after the animation and the gaze have posed
// it and before the world transforms are worked out from it.
//
// A chain can hang from a skin's own bones, a skirt only one dress has, so chains are bound
// to whatever bones the armature has once its skins are bound, and bound again when a skin
// comes or goes. A chain none of whose bones are there belongs to another skin.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use charpaper_anim::springs::Capsule;
use charpaper_anim::springs::Chain;
use charpaper_anim::springs::Forces;
use charpaper_anim::springs::Joint;
use charpaper_anim::springs::Settings;
use charpaper_suite::PhysicsChain;

use crate::character::Armature;
use crate::character::CharacterActor;
use crate::character::binding::Bound;
use crate::character::binding::SkinPart;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

const GRAVITY: f32 = 9.81;
const STIFFNESS: f32 = 0.2;
const DRAG: f32 = 0.1;

#[derive(Component, Default)]
pub(crate) struct Springs {
    chains: Vec<Swinging>,
    colliders: Vec<Collider>,
    capsules: Vec<Capsule>,
}

struct Swinging {
    name: String,
    parent: Entity,
    joints: Vec<(Entity, Source)>,
    locals: Vec<charpaper_anim::Transform>,
    chain: Chain,
}

struct Collider {
    bone: Entity,
    length: f32,
    radius: f32,
}

// Where a joint's animated pose comes from. A bone a skin brought isn't animated, so it
// follows the pose it came with, which has to be kept: its transform is the springs' after
// the first frame.
#[derive(Clone, Copy)]
enum Source {
    Rig(usize),
    Rest(charpaper_anim::Transform),
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn bind_springs(
    mut commands: Commands,
    mut armatures: Query<(Entity, &CharacterActor, Option<&mut Springs>), With<Armature>>,
    bound: Query<(), Added<Bound>>,
    mut gone: RemovedComponents<SkinPart>,
    fresh: Query<(), Added<CharacterActor>>,
    stage_of: StageOf,
    suites: Query<&ActiveSuite>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    meshes: Query<(), With<Mesh3d>>,
    transforms: Query<&Transform>,
) {
    let removed = gone.read().count() > 0;
    let skins_changed = removed || !bound.is_empty();
    for (armature, actor, springs) in &mut armatures {
        if !skins_changed && !fresh.contains(armature) && springs.is_some() {
            continue;
        }
        let Some(suite) = stage_of.get(armature).and_then(|stage| suites.get(stage).ok()) else {
            continue;
        };
        let physics = &suite.0.physics;
        let bones: HashMap<&str, Entity> = children
            .iter_descendants(armature)
            .filter(|&e| !meshes.contains(e))
            .filter_map(|e| names.get(e).ok().map(|name| (name.as_str(), e)))
            .collect();

        let mut old: HashMap<String, Swinging> = HashMap::new();
        if let Some(springs) = springs {
            old = springs.into_inner().chains.drain(..).map(|c| (c.name.clone(), c)).collect();
        }
        let mut chains = Vec::new();
        for (name, chain) in &physics.chains {
            let found: Vec<Entity> =
                chain.bones.iter().filter_map(|bone| bones.get(bone.as_str()).copied()).collect();
            if found.is_empty() {
                continue;
            }
            if found.len() < chain.bones.len() {
                warn!("physics {name:?}: some of its bones aren't on the armature");
                continue;
            }
            let linked = found
                .windows(2)
                .all(|pair| parents.get(pair[1]).is_ok_and(|parent| parent.parent() == pair[0]));
            let Some(parent) = parents.get(found[0]).ok().map(ChildOf::parent) else {
                continue;
            };
            if !linked {
                warn!("physics {name:?}: each bone has to be the child of the one before");
                continue;
            }
            if let Some(kept) = old.remove(name).filter(|kept| kept.parent == parent)
                && kept.joints.iter().map(|(e, _)| *e).eq(found.iter().copied())
            {
                chains.push(kept);
                continue;
            }
            let joints = found
                .iter()
                .map(|&entity| {
                    let source = match actor.node_index(entity) {
                        Some(at) => Source::Rig(at),
                        None => Source::Rest(to_anim(transforms.get(entity).ok())),
                    };
                    (entity, source)
                })
                .collect();
            chains.push(swinging(name, chain, parent, joints));
        }
        // A chain hanging from another chain's bone has to wait for that one to have swung.
        chains.sort_by_cached_key(|chain| parents.iter_ancestors(chain.parent).count());

        let colliders: Vec<Collider> = physics
            .colliders
            .iter()
            .filter_map(|(name, collider)| {
                let Some(&bone) = bones.get(collider.bone.as_str()) else {
                    debug!("physics {name:?}: no bone {:?} on this skin", collider.bone);
                    return None;
                };
                Some(Collider { bone, length: collider.length, radius: collider.radius })
            })
            .collect();

        if !chains.is_empty() {
            info!(
                "physics: {} chain(s) and {} collider(s) on this skin",
                chains.len(),
                colliders.len()
            );
        }
        commands.entity(armature).insert(Springs { chains, colliders, capsules: Vec::new() });
    }
}

fn swinging(
    name: &str,
    chain: &PhysicsChain,
    parent: Entity,
    joints: Vec<(Entity, Source)>,
) -> Swinging {
    let joint = Joint { length: chain.length, radius: chain.radius.unwrap_or(0.0) };
    let settings = Settings {
        stiffness: chain.stiffness.unwrap_or(STIFFNESS),
        drag: chain.drag.unwrap_or(DRAG),
        gravity: chain.gravity.unwrap_or(1.0),
    };
    Swinging {
        name: name.to_string(),
        parent,
        locals: vec![charpaper_anim::Transform::default(); joints.len()],
        chain: Chain::new(vec![joint; joints.len()], settings),
        joints,
    }
}

pub(crate) fn swing(
    time: Res<Time>,
    mut armatures: Query<(&CharacterActor, &mut Springs)>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
) {
    let forces = Forces { gravity: Vec3::NEG_Y * GRAVITY };
    let dt = time.delta_secs();
    for (actor, mut springs) in &mut armatures {
        let Springs { chains, colliders, capsules } = &mut *springs;
        let pose = &actor.mixer().pose().nodes;

        capsules.clear();
        capsules.extend(colliders.iter().map(|collider| {
            let bone = world(collider.bone, &parents, &transforms.as_readonly());
            Capsule::around(bone, collider.length, collider.radius)
        }));

        for swinging in chains {
            for ((_, source), local) in swinging.joints.iter().zip(&mut swinging.locals) {
                *local = match *source {
                    Source::Rig(at) => pose.get(at).copied().unwrap_or(*local),
                    Source::Rest(rest) => rest,
                };
            }
            let parent = world(swinging.parent, &parents, &transforms.as_readonly());
            swinging.chain.update(parent, &mut swinging.locals, capsules, forces, dt);
            for ((entity, _), local) in swinging.joints.iter().zip(&swinging.locals) {
                if let Ok(mut transform) = transforms.get_mut(*entity) {
                    transform.rotation = local.rotation;
                }
            }
        }
    }
}

// Worked out from the transforms as they are now, unlike GlobalTransform, which is last
// frame's until they are propagated.
fn world(entity: Entity, parents: &Query<&ChildOf>, transforms: &Query<&Transform>) -> Affine3A {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .filter_map(|e| transforms.get(e).ok())
        .fold(Affine3A::IDENTITY, |below, transform| transform.compute_affine() * below)
}

fn to_anim(transform: Option<&Transform>) -> charpaper_anim::Transform {
    let transform = transform.copied().unwrap_or_default();
    charpaper_anim::Transform {
        translation: transform.translation,
        rotation: transform.rotation,
        scale: transform.scale,
    }
}
