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

#[derive(Resource, Default, Debug)]
pub struct Expressions(pub Vec<String>);

#[derive(Component)]
pub(crate) struct SkinClips {
    clips: BTreeMap<String, Handle<AnimationClip>>,
    pub expression_meshes: BTreeSet<String>,
}

#[derive(Component)]
pub(crate) struct ExpressionPlayers {
    players: Vec<Entity>,
    nodes: BTreeMap<String, AnimationNodeIndex>,
    faces: Vec<Entity>,
    playing: Option<String>,
    easing: Option<(Vec<Vec<f32>>, Duration)>,
}

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
        // Bevy's clips no longer say what they animate, so the file itself says which are expressions.
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
        // Neutral is every expression shape key at 0, never the exported weights: Blender exports
        // whatever the top NLA track showed at the time.
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

        // A stopped clip leaves the face as it was, so going back to neutral is eased by hand.
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
