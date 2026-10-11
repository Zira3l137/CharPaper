use std::collections::BTreeMap;
use std::time::Duration;

use bevy::animation::AnimatedBy;
use bevy::animation::AnimationTargetId;
use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfLoaderSettings;
use bevy::prelude::*;
use charpaper_suite::AnimationFile;
use charpaper_suite::ClipCue;
use charpaper_suite::ClipSet;
use charpaper_suite::PlayMode;

use crate::assets::asset_path;
use crate::character::Armature;
use crate::character::binding::InstanceReady;
use crate::config::SceneConfig;
use crate::stage::StageOf;
use crate::state::CharacterState;
use crate::state::prefer;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;

#[derive(Component, Default)]
pub struct CharacterClips {
    clips: BTreeMap<String, Clip>,
}

pub(crate) struct Clip {
    pub node: AnimationNodeIndex,
    pub mode: PlayMode,
    pub gaze: bool,
    pub cues: Vec<ClipCue>,
}

impl CharacterClips {
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.clips.keys().map(String::as_str)
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Clip> {
        self.clips.get(name)
    }
}

#[derive(Component, Default)]
pub(crate) struct Playing(pub Option<String>);

#[derive(Component)]
pub(crate) struct PendingClips(Vec<(Handle<Gltf>, AnimationFile)>);

#[derive(Component)]
pub(crate) struct Animatable;

// Bevy only marks nodes as animation targets when a clip in the same file moves them, and
// the armature's file has no clips. So they are marked here the way the loader would: a
// node's id hashes the names from its top-level node down to it. That is why animation
// files must repeat those names exactly.
//
// The player sits on the armature's root entity, so one player covers several top-level
// nodes.
pub(crate) fn make_armature_animatable(
    mut commands: Commands,
    armatures: Query<Entity, (With<Armature>, With<InstanceReady>, Without<Animatable>)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for armature in &armatures {
        make_animatable(&mut commands, armature, &children, &names);
    }
}

fn make_animatable(
    commands: &mut Commands,
    armature: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
) {
    let mut pending: Vec<(Entity, Vec<&str>)> = Vec::new();
    for &scene_root in kids(children, armature) {
        pending.extend(kids(children, scene_root).iter().map(|&node| (node, Vec::new())));
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
        pending.extend(kids(children, node).iter().map(|&child| (child, path.clone())));
    }

    commands.entity(armature).insert((
        Animatable,
        AnimationPlayer::default(),
        AnimationTransitions::new(),
    ));
    info!("armature: {targets} node(s) ready to animate");
}

pub(crate) fn load_clips(
    mut commands: Commands,
    stages: Query<(Entity, &ActiveSuite), Added<ActiveSuite>>,
    assets: Res<AssetServer>,
) {
    for (stage, suite) in &stages {
        let files = suite
            .animations
            .iter()
            .map(|file| {
                let handle = assets
                    .load_builder()
                    .with_settings(clips_only)
                    .load(asset_path(suite, &file.path));
                (handle, file.clone())
            })
            .collect();
        commands.entity(stage).insert(PendingClips(files));
    }
}

// An animation file may carry the meshes its correctives key; only its clips are wanted.
fn clips_only(settings: &mut GltfLoaderSettings) {
    settings.load_meshes = RenderAssetUsages::empty();
    settings.load_materials = RenderAssetUsages::empty();
    settings.load_cameras = false;
    settings.load_lights = false;
}

pub(crate) fn build_graph(
    mut commands: Commands,
    armatures: Query<Entity, (With<Animatable>, Without<AnimationGraphHandle>)>,
    stage_of: StageOf,
    mut stages: Query<(&PendingClips, &ActiveSuite, &Remembered, &mut CharacterState)>,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for armature in &armatures {
        let Some(stage) = stage_of.get(armature) else {
            continue;
        };
        let Ok((pending, suite, remembered, mut state)) = stages.get_mut(stage) else {
            continue;
        };
        let clips = build_clips(pending, suite, &assets, &gltfs, &mut graphs);
        let Some((graph, clips)) = clips else {
            continue;
        };
        state.animation = prefer(
            suite.remembered(remembered).animation,
            |name| clips.contains_key(name),
            suite.default_animation.clone(),
        );
        commands.entity(armature).insert((AnimationGraphHandle(graph), Playing::default()));
        commands.entity(stage).insert(CharacterClips { clips }).remove::<PendingClips>();
    }
}

// None while a file is still loading.
fn build_clips(
    pending: &PendingClips,
    suite: &ActiveSuite,
    assets: &AssetServer,
    gltfs: &Assets<Gltf>,
    graphs: &mut Assets<AnimationGraph>,
) -> Option<(Handle<AnimationGraph>, BTreeMap<String, Clip>)> {
    let loading = pending
        .0
        .iter()
        .any(|(h, _)| gltfs.get(h).is_none() && !assets.load_state(h.id()).is_failed());
    if loading {
        return None;
    }

    let mut graph = AnimationGraph::new();
    let mut clips = BTreeMap::new();
    for (handle, file) in &pending.0 {
        let Some(gltf) = gltfs.get(handle) else {
            warn!("skipping the clips in {}: it failed to load", file.path.display());
            continue;
        };
        for Selected { name, clip, mode, gaze, cues } in select(gltf, file) {
            let node = graph.add_clip(clip, 1.0, graph.root);
            if clips.insert(name.clone(), Clip { node, mode, gaze, cues }).is_some() {
                warn!("animation {name:?} is defined twice; the later one wins");
            }
        }
    }

    info!("{} animation(s): {:?}", clips.len(), clips.keys().collect::<Vec<_>>());
    if let Some(default) = &suite.default_animation
        && !clips.contains_key(default)
    {
        warn!("default animation {default:?} is not among them");
    }
    Some((graphs.add(graph), clips))
}

pub(crate) fn finish_once(
    armatures: Query<(Entity, &AnimationPlayer, &Playing)>,
    stage_of: StageOf,
    mut stages: Query<(&mut CharacterState, &CharacterClips, &ActiveSuite)>,
) {
    for (armature, player, playing) in &armatures {
        let Some(name) = &playing.0 else {
            continue;
        };
        let Some(stage) = stage_of.get(armature) else {
            continue;
        };
        let Ok((mut state, clips, suite)) = stages.get_mut(stage) else {
            continue;
        };
        let Some(clip) = clips.get(name) else {
            continue;
        };
        if clip.mode != PlayMode::Once || suite.default_animation.as_ref() == Some(name) {
            continue;
        }
        if player.animation(clip.node).is_some_and(|active| active.is_finished()) {
            state.animation = suite.default_animation.clone();
        }
    }
}

// Runs every frame, since a request can arrive before the graph exists.
pub(crate) fn play_selected(
    config: Res<SceneConfig>,
    mut armatures: Query<(Entity, &mut AnimationPlayer, &mut AnimationTransitions, &mut Playing)>,
    stage_of: StageOf,
    stages: Query<(&CharacterState, &CharacterClips)>,
) {
    for (armature, mut player, mut transitions, mut playing) in &mut armatures {
        let Some((state, clips)) = stage_of.get(armature).and_then(|s| stages.get(s).ok()) else {
            continue;
        };
        if playing.0 == state.animation {
            continue;
        }
        playing.0 = state.animation.clone();
        play(&config, &mut player, &mut transitions, state, clips);
    }
}

fn play(
    config: &SceneConfig,
    player: &mut AnimationPlayer,
    transitions: &mut AnimationTransitions,
    state: &CharacterState,
    clips: &CharacterClips,
) {
    let Some(name) = &state.animation else {
        player.stop_all();
        return;
    };
    let Some(clip) = clips.get(name) else {
        warn!("no animation named {name:?}");
        return;
    };

    let fade = Duration::from_secs_f32(config.animation_crossfade_secs.max(0.0));
    let active = transitions.play(player, clip.node, fade);
    match clip.mode {
        PlayMode::Loop => {
            active.repeat();
        }
        PlayMode::Once => {}
        // Speed 0 rather than paused: AnimationTransitions never fades out a paused animation,
        // so its pose would keep full weight under everything played after it.
        PlayMode::Pose => {
            active.set_speed(0.0);
        }
    }
    debug!("playing {name:?} ({:?})", clip.mode);
}

struct Selected {
    name: String,
    clip: Handle<AnimationClip>,
    mode: PlayMode,
    gaze: bool,
    cues: Vec<ClipCue>,
}

fn select(gltf: &Gltf, file: &AnimationFile) -> Vec<Selected> {
    match &file.clips {
        ClipSet::All => {
            let mut all: Vec<_> = gltf
                .named_animations
                .iter()
                .map(|(name, clip)| Selected {
                    name: name.to_string(),
                    clip: clip.clone(),
                    mode: PlayMode::Loop,
                    gaze: true,
                    cues: Vec::new(),
                })
                .collect();
            all.sort_by(|a, b| a.name.cmp(&b.name));
            all
        }
        ClipSet::Listed(bindings) => bindings
            .iter()
            .filter_map(|binding| {
                let clip = match &binding.clip {
                    Some(clip) => gltf.named_animations.get(clip.as_str()).cloned(),
                    None if gltf.animations.len() == 1 => gltf.animations.first().cloned(),
                    None => None,
                };
                if clip.is_none() {
                    warn!(
                        "animation {:?}: no matching clip in {}",
                        binding.name,
                        file.path.display()
                    );
                }
                clip.map(|clip| Selected {
                    name: binding.name.clone(),
                    clip,
                    mode: binding.mode,
                    gaze: binding.gaze,
                    cues: binding.cues.clone(),
                })
            })
            .collect(),
    }
}

pub(crate) fn kids<'a>(children: &'a Query<&Children>, entity: Entity) -> &'a [Entity] {
    children.get(entity).map(|c| &**c).unwrap_or_default()
}
