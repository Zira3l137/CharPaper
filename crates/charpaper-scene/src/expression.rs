//! Expressions: the looping clips stored in the worn skin's own file, which
//! key its shape keys.
//!
//! Each skin has its own players, put in place by Bevy's glTF loader because
//! the clips move nodes of that same file, so expressions never touch the
//! armature's player and the body animation plays on regardless. Switching
//! crossfades between clips; going back to neutral stops the clip and eases
//! every animated shape key back to the weight it was exported with, since a
//! stopped clip otherwise leaves the face as it last posed it.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Duration;

use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;

use crate::SceneConfig;
use crate::binding::InstanceReady;
use crate::character::CharacterState;
use crate::character::SkinFile;
use crate::character::SkinRoot;
use crate::suite::ActiveSuite;

/// The worn skin's expressions, in name order: what the panel's Expression
/// cycler offers after Neutral. Filled once the skin's file has loaded.
#[derive(Resource, Default, Debug)]
pub struct Expressions(pub Vec<String>);

/// The worn skin's expressions by name, taken from its file when it loads,
/// and the mesh objects they key. Those meshes belong to the expressions;
/// every other mesh with shape keys takes correctives from body animations.
#[derive(Component)]
pub(crate) struct SkinClips {
    clips: BTreeMap<String, Handle<AnimationClip>>,
    pub expression_meshes: BTreeSet<String>,
}

/// Everything needed to play the worn skin's expressions, set up once its
/// scene has spawned.
#[derive(Component)]
pub(crate) struct ExpressionPlayers {
    players: Vec<Entity>,
    nodes: BTreeMap<String, AnimationNodeIndex>,
    /// Every entity with shape keys, and the weights it was exported with.
    rest: Vec<(Entity, Vec<f32>)>,
    playing: Option<String>,
    /// While easing back to neutral: the weights it started from, and how far
    /// it has got.
    easing: Option<(Vec<Vec<f32>>, Duration)>,
}

/// Spawns a skin's scene once its file has loaded, and publishes its clips as
/// the expressions on offer.
pub(crate) fn spawn_skin_scenes(
    mut commands: Commands,
    skins: Query<(Entity, &SkinRoot, &SkinFile), Without<WorldAssetRoot>>,
    suite: Option<Res<ActiveSuite>>,
    gltfs: Res<Assets<Gltf>>,
    assets: Res<AssetServer>,
    mut expressions: ResMut<Expressions>,
) {
    let Some(suite) = suite else {
        return;
    };
    for (entity, skin, file) in &skins {
        if assets.load_state(file.0.id()).is_failed() {
            warn!("skin {:?}: its file failed to load", skin.name);
            commands.entity(entity).remove::<SkinFile>();
            continue;
        }
        let Some(gltf) = gltfs.get(&file.0) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned())
        else {
            warn!("skin {:?}: its file holds no scene", skin.name);
            commands.entity(entity).remove::<SkinFile>();
            continue;
        };
        // Which clips key only shape keys is read from the file itself: Bevy's
        // clips no longer say what kind of property each curve moves.
        let file =
            suite.skins.iter().find(|s| s.name == skin.name).map(|s| suite.absolute(&s.file));
        let found = match file.map(|f| charpaper_suite::skin_clips(&f)) {
            Some(Ok(found)) => found,
            Some(Err(err)) => {
                warn!("skin {:?}: cannot read its clips: {err}", skin.name);
                charpaper_suite::SkinClips::default()
            }
            None => charpaper_suite::SkinClips::default(),
        };
        let clips: BTreeMap<String, Handle<AnimationClip>> = found
            .expressions
            .iter()
            .filter_map(|name| {
                Some((name.clone(), gltf.named_animations.get(name.as_str())?.clone()))
            })
            .collect();
        let skipped = gltf.named_animations.len() - clips.len();
        if skipped > 0 {
            debug!("skin {:?}: {skipped} clip(s) move bones and are not expressions", skin.name);
        }
        expressions.0 = clips.keys().cloned().collect();
        let clips = SkinClips { clips, expression_meshes: found.expression_meshes };
        commands.entity(entity).insert((WorldAssetRoot(scene), clips));
    }
}

/// Once the skin's scene has spawned: every player the loader put in it gets
/// one graph with all the skin's clips, and the exported weights of every
/// shape-keyed mesh are noted for going back to neutral.
pub(crate) fn setup_expressions(
    mut commands: Commands,
    skins: Query<
        (Entity, &SkinRoot, &SkinClips),
        (With<InstanceReady>, Without<ExpressionPlayers>),
    >,
    children: Query<&Children>,
    names: Query<&Name>,
    players: Query<(), With<AnimationPlayer>>,
    morphs: Query<&MorphWeights>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (root, skin, clips) in &skins {
        let descendants: Vec<Entity> = children.iter_descendants(root).collect();
        let found: Vec<Entity> =
            descendants.iter().copied().filter(|&e| players.contains(e)).collect();
        if !clips.clips.is_empty() && found.is_empty() {
            warn!("skin {:?}: its clips animate nothing in its file", skin.name);
        }

        let (graph, indices) = AnimationGraph::from_clips(clips.clips.values().cloned());
        let graph = graphs.add(graph);
        for &player in &found {
            commands
                .entity(player)
                .insert((AnimationGraphHandle(graph.clone()), AnimationTransitions::new()));
        }
        // Only the expressions' own meshes go back to rest on neutral;
        // correctives belong to the body animation.
        let owned =
            |e: Entity| names.get(e).is_ok_and(|n| clips.expression_meshes.contains(n.as_str()));
        let rest = descendants
            .iter()
            .filter(|&&e| owned(e))
            .filter_map(|&e| morphs.get(e).ok().map(|m| (e, m.weights().to_vec())))
            .collect();

        commands.entity(root).insert(ExpressionPlayers {
            players: found,
            nodes: clips.clips.keys().cloned().zip(indices).collect(),
            rest,
            playing: None,
            easing: None,
        });
        info!("skin {:?}: {} expression(s)", skin.name, clips.clips.len());
    }
}

pub(crate) fn play_expression(
    time: Res<Time>,
    state: Res<CharacterState>,
    config: Res<SceneConfig>,
    mut skins: Query<&mut ExpressionPlayers>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    mut morphs: Query<&mut MorphWeights>,
) {
    let fade = Duration::from_secs_f32(config.expression_fade_secs.max(0.0));
    for mut skin in &mut skins {
        let skin = &mut *skin;
        let wanted = state.expression().filter(|name| skin.nodes.contains_key(*name));

        if wanted != skin.playing.as_deref() {
            skin.playing = wanted.map(str::to_string);
            match wanted.and_then(|name| skin.nodes.get(name)) {
                Some(&node) => {
                    skin.easing = None;
                    for &entity in &skin.players {
                        if let Ok((mut player, mut transitions)) = players.get_mut(entity) {
                            transitions.play(&mut player, node, fade).repeat();
                        }
                    }
                }
                None => {
                    for &entity in &skin.players {
                        if let Ok((mut player, _)) = players.get_mut(entity) {
                            player.stop_all();
                        }
                    }
                    let from = skin
                        .rest
                        .iter()
                        .map(|(e, rest)| {
                            morphs.get(*e).map_or(rest.clone(), |m| m.weights().to_vec())
                        })
                        .collect();
                    skin.easing = Some((from, Duration::ZERO));
                }
            }
        }

        // Easing back to neutral; a clip, once one plays, writes the weights
        // itself.
        let Some((from, elapsed)) = &mut skin.easing else {
            continue;
        };
        *elapsed += time.delta();
        let t = if fade.is_zero() {
            1.0
        } else {
            (elapsed.as_secs_f32() / fade.as_secs_f32()).min(1.0)
        };
        for ((entity, rest), start) in skin.rest.iter().zip(from.iter()) {
            if let Ok(mut morph) = morphs.get_mut(*entity) {
                for ((weight, &end), &begin) in morph.weights_mut().iter_mut().zip(rest).zip(start)
                {
                    *weight = begin + (end - begin) * t;
                }
            }
        }
        if t >= 1.0 {
            skin.easing = None;
        }
    }
}
