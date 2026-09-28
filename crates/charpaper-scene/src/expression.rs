//! Expressions: the looping clips stored in the worn skin's own file, which
//! key its shape keys.
//!
//! Each skin has its own players, put in place by Bevy's glTF loader because
//! the clips move nodes of that same file, so expressions never touch the
//! armature's player and the body animation plays on regardless. Switching
//! crossfades between clips; going back to neutral stops the clip and eases
//! the expression meshes' shape keys back to 0, since a stopped clip
//! otherwise leaves the face as it last posed it.
//!
//! Neutral is always every such key at 0, the Basis shape, and never the
//! weights the file was exported with: Blender writes those from whatever the
//! shape keys showed at export time, which with the expressions stacked in
//! the NLA is the top track's pose.

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
    /// The meshes whose shape keys the expressions own.
    faces: Vec<Entity>,
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
    mut morphs: Query<&mut MorphWeights>,
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
        // Only the expressions' own meshes are neutral at 0; correctives
        // belong to the body animation. They start there, so the skin is
        // neutral from its first frame whatever the file's defaults say.
        let owned =
            |e: Entity| names.get(e).is_ok_and(|n| clips.expression_meshes.contains(n.as_str()));
        let faces: Vec<Entity> =
            descendants.iter().copied().filter(|&e| owned(e) && morphs.contains(e)).collect();
        for &face in &faces {
            if let Ok(mut morph) = morphs.get_mut(face) {
                morph.weights_mut().fill(0.0);
            }
        }

        commands.entity(root).insert(ExpressionPlayers {
            players: found,
            nodes: clips.clips.keys().cloned().zip(indices).collect(),
            faces,
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
                        .faces
                        .iter()
                        .map(|&e| morphs.get(e).map(|m| m.weights().to_vec()).unwrap_or_default())
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
        for (entity, start) in skin.faces.iter().zip(from.iter()) {
            if let Ok(mut morph) = morphs.get_mut(*entity) {
                for (weight, &begin) in morph.weights_mut().iter_mut().zip(start) {
                    *weight = begin * (1.0 - t);
                }
            }
        }
        if t >= 1.0 {
            skin.easing = None;
        }
    }
}
