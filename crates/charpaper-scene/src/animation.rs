use std::collections::BTreeMap;
use std::time::Duration;

use bevy::animation::AnimatedBy;
use bevy::animation::AnimationTargetId;
use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfLoaderSettings;
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

#[derive(Component, Default)]
pub(crate) struct Playing(Option<String>);

#[derive(Resource)]
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
    armature: Query<Entity, (With<Armature>, With<InstanceReady>, Without<Animatable>)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok(armature) = armature.single() else {
        return;
    };

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
        .map(|file| {
            let handle = assets
                .load_builder()
                .with_settings(clips_only)
                .load(asset_path(&suite, &file.path));
            (handle, file.clone())
        })
        .collect();
    commands.insert_resource(PendingClips(files));
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

// Runs every frame, since a request can arrive before the graph exists.
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
        // Speed 0 rather than paused: AnimationTransitions never fades out a paused animation,
        // so its pose would keep full weight under everything played after it.
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

pub(crate) fn kids<'a>(children: &'a Query<&Children>, entity: Entity) -> &'a [Entity] {
    children.get(entity).map(|c| &**c).unwrap_or_default()
}
