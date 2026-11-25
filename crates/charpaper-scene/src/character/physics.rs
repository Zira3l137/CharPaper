// The suite's spring chains, run on the armature after the animation and the gaze have posed
// it and before the world transforms are worked out from it.
//
// With high quality on, chains keep off the body's own triangles around each collider's bone
// instead of its capsule. The app keeps no copy of the meshes once they're on the GPU, so they
// are read from the worn skin's file in the background; until then, and around bones no mesh
// is weighted to, the capsules do.
//
// A chain can hang from a skin's own bones, a skirt only one dress has, so chains are bound
// to whatever bones the armature has once its skins are bound, and bound again when a skin
// comes or goes. A chain none of whose bones are there belongs to another skin.

use std::collections::BTreeMap;
use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::Task;
use bevy::tasks::block_on;
use bevy::tasks::poll_once;
use charpaper_anim::springs::Capsule;
use charpaper_anim::springs::Chain;
use charpaper_anim::springs::Collider as Pushes;
use charpaper_anim::springs::Forces;
use charpaper_anim::springs::Joint;
use charpaper_anim::springs::Settings;
use charpaper_anim::surface;
use charpaper_anim::surface::Part;
use charpaper_settings::PhysicsSettings;
use charpaper_suite::PhysicsChain;

use crate::character::Armature;
use crate::character::CharacterActor;
use crate::character::ShownSkin;
use crate::character::binding::Bound;
use crate::character::binding::SkinPart;
use crate::stage::StageOf;
use crate::state::CharacterState;
use crate::suite::ActiveSuite;

const GRAVITY: f32 = 9.81;
const STIFFNESS: f32 = 0.2;
const DRAG: f32 = 0.1;

// Its Body is kept when the chains are bound again, which a skin's coming binds them.
#[derive(Component, Default)]
#[require(Body)]
pub(crate) struct Springs {
    chains: Vec<Swinging>,
    colliders: Vec<Collider>,
}

// The worn skin's body around the colliders' bones, for high quality.
#[derive(Component, Default)]
pub(crate) struct Body {
    // The skin read, or being read.
    skin: Option<String>,
    reading: Option<Task<Result<Vec<Part>, String>>>,
    parts: Vec<Part>,
    // Which of the parts each collider's bone keeps chains off.
    around: BTreeMap<String, usize>,
}

struct Swinging {
    name: String,
    parent: Entity,
    joints: Vec<(Entity, Source)>,
    locals: Vec<charpaper_anim::Transform>,
    chain: Chain,
}

struct Collider {
    name: String,
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
                let name = collider.bone.clone();
                Some(Collider { name, bone, length: collider.length, radius: collider.radius })
            })
            .collect();

        if !chains.is_empty() {
            info!(
                "physics: {} chain(s) and {} collider(s) on this skin",
                chains.len(),
                colliders.len()
            );
        }
        commands.entity(armature).insert(Springs { chains, colliders });
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

pub(crate) fn read_body(
    mut armatures: Query<(Entity, &mut Body)>,
    stage_of: StageOf,
    stages: Query<(&ActiveSuite, &PhysicsSettings, &ShownSkin, Ref<CharacterState>)>,
) {
    for (armature, mut body) in &mut armatures {
        let Some((suite, settings, shown, state)) =
            stage_of.get(armature).and_then(|s| stages.get(s).ok())
        else {
            continue;
        };
        let wanted =
            settings.enabled && settings.high_quality && !suite.physics.colliders.is_empty();
        let skin = shown.name.clone().filter(|_| wanted);
        if body.skin != skin {
            *body = Body { skin: skin.clone(), ..default() };
            if let Some(file) = skin.and_then(|name| suite.skins.iter().find(|s| s.name == name)) {
                let path = suite.absolute(&file.file);
                let bones: Vec<(String, f32)> =
                    suite.physics.colliders.values().map(|c| (c.bone.clone(), c.length)).collect();
                info!("physics: reading the body from {} for high quality", path.display());
                body.reading = Some(AsyncComputeTaskPool::get().spawn(async move {
                    surface::read_parts(&path, &bones).map_err(|e| e.to_string())
                }));
            }
        }
        let read = body.reading.as_mut().and_then(|task| block_on(poll_once(task)));
        let arrived = read.is_some();
        if let Some(result) = read {
            body.reading = None;
            match result {
                Ok(parts) => body.parts = parts,
                Err(err) => warn!("physics: cannot read the body, so capsules stay: {err}"),
            }
        }
        // A hidden coat is no body to keep off.
        if arrived || state.is_changed() {
            let body = &mut *body;
            let hidden = body.skin.as_ref().and_then(|skin| state.hidden.get(skin));
            let is_hidden = |object: &str| hidden.is_some_and(|h| h.contains(object));
            body.around = surface::body(&body.parts, is_hidden);
        }
        if arrived {
            info!(
                "physics: {} collider bone(s) keep chains off the body's mesh",
                body.around.len()
            );
        }
    }
}

pub(crate) fn swing(
    time: Res<Time>,
    mut armatures: Query<(Entity, &CharacterActor, &mut Springs, &Body)>,
    stage_of: StageOf,
    settings: Query<&PhysicsSettings>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
) {
    let forces = Forces { gravity: Vec3::NEG_Y * GRAVITY };
    let dt = time.delta_secs();
    for (armature, actor, mut springs, body) in &mut armatures {
        let on = stage_of.get(armature).and_then(|stage| settings.get(stage).ok());
        let Springs { chains, colliders } = &mut *springs;
        let pose = &actor.mixer().pose().nodes;

        for swinging in chains.iter_mut() {
            for ((_, source), local) in swinging.joints.iter().zip(&mut swinging.locals) {
                *local = match *source {
                    Source::Rig(at) => pose.get(at).copied().unwrap_or(*local),
                    Source::Rest(rest) => rest,
                };
            }
        }
        // Off, the bones go back to the animation: the mixer only writes the ones it animates.
        if !on.is_some_and(|settings| settings.enabled) {
            for swinging in chains.iter_mut() {
                swinging.chain.reset();
                write(swinging, &mut transforms);
            }
            continue;
        }

        let readonly = transforms.as_readonly();
        let pushes: Vec<Pushes> = colliders
            .iter()
            .map(|collider| {
                let bone = world(collider.bone, &parents, &readonly);
                match body.around.get(&collider.name) {
                    Some(&at) => Pushes::surface(&body.parts[at].surface, bone),
                    None => Capsule::around(bone, collider.length, collider.radius).into(),
                }
            })
            .collect();

        for swinging in chains.iter_mut() {
            let parent = world(swinging.parent, &parents, &transforms.as_readonly());
            swinging.chain.update(parent, &mut swinging.locals, &pushes, forces, dt);
            write(swinging, &mut transforms);
        }
    }
}

fn write(swinging: &Swinging, transforms: &mut Query<&mut Transform>) {
    for ((entity, _), local) in swinging.joints.iter().zip(&swinging.locals) {
        if let Ok(mut transform) = transforms.get_mut(*entity)
            && transform.rotation != local.rotation
        {
            transform.rotation = local.rotation;
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
