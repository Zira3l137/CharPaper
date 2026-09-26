//! Plays the suite's clips on the armature.

use std::collections::BTreeMap;
use std::time::Duration;

use bevy::animation::AnimatedBy;
use bevy::animation::AnimationTargetId;
use bevy::prelude::*;
use charpaper_suite::AnimationFile;
use charpaper_suite::ClipSet;
use charpaper_suite::PlayMode;

use crate::SceneConfig;
use crate::binding::InstanceReady;
use crate::character::Armature;
use crate::character::CharacterState;
use crate::character::prefer;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;
use crate::suite::asset_path;

/// Every clip the suite offers, under the name `suite.toml` and the UI use.
/// Present once the animation files have loaded.
#[derive(Resource, Default)]
pub struct CharacterClips {
    clips: BTreeMap<String, Clip>,
}

#[derive(Clone, Copy)]
pub(crate) struct Clip {
    pub node: AnimationNodeIndex,
    pub mode: PlayMode,
}

impl CharacterClips {
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.clips.keys().map(String::as_str)
    }

    pub(crate) fn get(&self, name: &str) -> Option<Clip> {
        self.clips.get(name).copied()
    }
}

/// What the armature is actually playing, as opposed to what
/// [`CharacterState::animation`] asks for.
#[derive(Component, Default)]
pub(crate) struct Playing(Option<String>);

/// Animation files still loading. Removed once the graph is built.
#[derive(Resource)]
pub(crate) struct PendingClips(Vec<(Handle<Gltf>, AnimationFile)>);

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

pub(crate) fn load_clips(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
) {
    let Some(suite) = suite else {
        return;
    };
    let files = suite
        .animations
        .iter()
        .map(|file| (assets.load(asset_path(&suite, &file.path)), file.clone()))
        .collect();
    commands.insert_resource(PendingClips(files));
}

/// Waits for every animation file to load or fail, then puts all their clips
/// side by side in one graph. A plain list is enough: the transitions
/// component does the blending when switching, and nothing plays layered yet.
pub(crate) fn build_graph(
    mut commands: Commands,
    pending: Option<Res<PendingClips>>,
    armature: Query<Entity, (With<Animatable>, Without<AnimationGraphHandle>)>,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    suite: Option<Res<ActiveSuite>>,
    remembered: Res<Remembered>,
    mut state: ResMut<CharacterState>,
) {
    let (Some(pending), Some(suite)) = (pending, suite) else {
        return;
    };
    let Ok(armature) = armature.single() else {
        return;
    };
    let loading = pending
        .0
        .iter()
        .any(|(h, _)| gltfs.get(h).is_none() && !assets.load_state(h.id()).is_failed());
    if loading {
        return;
    }

    let mut graph = AnimationGraph::new();
    let mut clips = BTreeMap::new();
    for (handle, file) in &pending.0 {
        let Some(gltf) = gltfs.get(handle) else {
            warn!("skipping the clips in {}: it failed to load", file.path.display());
            continue;
        };
        for (name, clip, mode) in select(gltf, file) {
            let node = graph.add_clip(clip, 1.0, graph.root);
            if clips.insert(name.clone(), Clip { node, mode }).is_some() {
                warn!("animation {name:?} is defined twice; the later one wins");
            }
        }
    }

    info!("{} animation(s): {:?}", clips.len(), clips.keys().collect::<Vec<_>>());
    if let Some(default) = &suite.default_animation {
        if !clips.contains_key(default) {
            warn!("default animation {default:?} is not among them");
        }
    }

    state.animation = prefer(
        suite.remembered(&remembered).animation,
        |name| clips.contains_key(name),
        suite.default_animation.clone(),
    );
    commands.entity(armature).insert((AnimationGraphHandle(graphs.add(graph)), Playing::default()));
    commands.insert_resource(CharacterClips { clips });
    commands.remove_resource::<PendingClips>();
}

/// A `once` clip hands back to the default animation when it ends. When the
/// default is itself `once`, it plays through and holds its last frame.
pub(crate) fn finish_once(
    mut state: ResMut<CharacterState>,
    clips: Option<Res<CharacterClips>>,
    suite: Option<Res<ActiveSuite>>,
    armature: Query<(&AnimationPlayer, &Playing)>,
) {
    let (Some(clips), Some(suite)) = (clips, suite) else {
        return;
    };
    let Ok((player, Playing(Some(name)))) = armature.single() else {
        return;
    };
    let Some(clip) = clips.get(name) else {
        return;
    };
    if clip.mode != PlayMode::Once || suite.default_animation.as_ref() == Some(name) {
        return;
    }
    if player.animation(clip.node).is_some_and(|active| active.is_finished()) {
        state.animation = suite.default_animation.clone();
    }
}

/// Runs every frame rather than on change: a request can arrive before the
/// graph exists, and comparing two names is cheaper than tracking that.
pub(crate) fn play_selected(
    state: Res<CharacterState>,
    clips: Option<Res<CharacterClips>>,
    config: Res<SceneConfig>,
    mut armature: Query<(&mut AnimationPlayer, &mut AnimationTransitions, &mut Playing)>,
) {
    let Some(clips) = clips else {
        return;
    };
    let Ok((mut player, mut transitions, mut playing)) = armature.single_mut() else {
        return;
    };
    if playing.0 == state.animation {
        return;
    }
    playing.0 = state.animation.clone();

    let Some(name) = &state.animation else {
        player.stop_all();
        return;
    };
    let Some(clip) = clips.get(name) else {
        warn!("no animation named {name:?}");
        return;
    };

    let fade = Duration::from_secs_f32(config.animation_crossfade_secs.max(0.0));
    let active = transitions.play(&mut player, clip.node, fade);
    match clip.mode {
        PlayMode::Loop => {
            active.repeat();
        }
        PlayMode::Once => {}
        // Speed 0 rather than pausing: `AnimationTransitions` never fades out
        // a paused animation, so a paused pose would keep full weight under
        // every animation played after it.
        PlayMode::Pose => {
            active.set_speed(0.0);
        }
    }
    debug!("playing {name:?} ({:?})", clip.mode);
}

fn select(gltf: &Gltf, file: &AnimationFile) -> Vec<(String, Handle<AnimationClip>, PlayMode)> {
    match &file.clips {
        ClipSet::All => {
            let mut all: Vec<_> = gltf
                .named_animations
                .iter()
                .map(|(name, clip)| (name.to_string(), clip.clone(), PlayMode::Loop))
                .collect();
            all.sort_by(|a, b| a.0.cmp(&b.0));
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
                clip.map(|clip| (binding.name.clone(), clip, binding.mode))
            })
            .collect(),
    }
}

fn kids<'a>(children: &'a Query<&Children>, entity: Entity) -> &'a [Entity] {
    children.get(entity).map(|c| &**c).unwrap_or_default()
}
