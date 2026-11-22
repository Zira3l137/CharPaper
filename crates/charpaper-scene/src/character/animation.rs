use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use charpaper_anim::Actor;
use charpaper_anim::ClipId;
use charpaper_anim::Event;
use charpaper_anim::Mode;
use charpaper_anim::Rig;
use charpaper_audio::AudioSource;
use charpaper_audio::FadeOut;
use charpaper_audio::Hush;
use charpaper_audio::Layer;
use charpaper_audio::Sound;
use charpaper_suite::ClipCue;
use charpaper_suite::ClipMode;

use crate::assets::asset_path;
use crate::character::Armature;
use crate::character::binding::InstanceReady;
use crate::character::expression::Expressions;
use crate::character::skin::ShownSkin;
use crate::character::skin_keys::SkinKeys;
use crate::clips::ClipAsset;
use crate::clips::Timeline;
use crate::clips::due;
use crate::hotspots::Reactions;
use crate::stage::StageOf;
use crate::state::CharacterState;
use crate::state::prefer;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;

// How long one clip blends into the next, and how long the sounds of the one that ends take
// to fade out.
const CROSSFADE: Duration = Duration::from_millis(250);

// The mixer's slots for the character, bottom up. A Once clip plays above the body's clip, so
// that clip shows again when it ends. The expression plays above both, so it wins on the shape
// keys it keys and leaves the rest to the body. Reactions play above all that, with the
// expression a hotspot sets on top.
const BODY: usize = 0;
const BODY_ONCE: usize = 1;
const FACE: usize = 2;
pub(crate) const REACTIONS: Reactions = Reactions { replacing: 3, layers: 4..7 };
const REACTION_FACE: usize = 7;
const SLOTS: usize = 8;

// The character's clips as suite.toml and the clips folder name them, each loading from the
// moment the suite is picked.
#[derive(Component, Default)]
pub struct CharacterClips {
    clips: BTreeMap<String, Clip>,
    expressions: BTreeMap<String, Handle<ClipAsset>>,
    // The cues' sounds, loaded up front so the first one isn't late.
    sounds: BTreeMap<PathBuf, Handle<AudioSource>>,
}

pub(crate) struct Clip {
    pub mode: ClipMode,
    pub gaze: bool,
    pub reaction_only: bool,
    file: Handle<ClipAsset>,
    cues: Vec<ClipCue>,
}

impl CharacterClips {
    /// The clips the viewer can pick: all but those only hotspots play.
    pub fn offered(&self) -> impl Iterator<Item = &str> {
        self.clips.iter().filter(|(_, clip)| !clip.reaction_only).map(|(name, _)| name.as_str())
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Clip> {
        self.clips.get(name)
    }
}

// The armature's mixer, the entity each of its rig's nodes is, and its clips once loaded.
#[derive(Component)]
pub(crate) struct CharacterActor {
    actor: Actor,
    nodes: Vec<Entity>,
    ids: BTreeMap<String, ClipId>,
    expressions: BTreeMap<String, ClipId>,
    // The pick this has acted on, and the clip under a Once clip, which the pick goes back to.
    chosen: Option<String>,
    under: Option<String>,
    face: Option<String>,
    events: Vec<Event>,
}

impl CharacterActor {
    pub(crate) fn mixer(&self) -> &Actor {
        &self.actor
    }

    pub(crate) fn mixer_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    pub(crate) fn node(&self, name: &str) -> Option<Entity> {
        let at = self.actor.rig().nodes.iter().position(|(own, _)| own == name)?;
        self.nodes.get(at).copied()
    }

    // None while it is still loading.
    pub(crate) fn clip_id(&self, name: &str) -> Option<ClipId> {
        self.ids.get(name).copied()
    }

    // Switches to a timeline or hold clip at once, as if the viewer had picked it: for a switch
    // a reaction fading out above hides. False until the clip has loaded.
    pub(crate) fn cut_to(&mut self, name: &str, clips: &CharacterClips) -> bool {
        let (Some(&id), Some(clip)) = (self.ids.get(name), clips.get(name)) else {
            return false;
        };
        let mode = match clip.mode {
            ClipMode::Timeline => Mode::Timeline,
            ClipMode::Hold => Mode::Hold,
            ClipMode::Once => return false,
        };
        self.actor.cut(BODY, id, mode);
        self.actor.stop(BODY_ONCE);
        self.under = Some(name.to_string());
        self.chosen = Some(name.to_string());
        true
    }

    // The animation the viewer picked to play on and on, also while a clip picked to play once
    // plays over it.
    pub(crate) fn base(&self) -> Option<&str> {
        self.under.as_deref()
    }

    // A hotspot's expression, over the chosen one, or none.
    pub(crate) fn reaction_face(&mut self, expression: Option<&str>) {
        match expression.and_then(|name| self.expressions.get(name)) {
            Some(&id) => self.actor.play(REACTION_FACE, id, Mode::Timeline),
            None => self.actor.stop(REACTION_FACE),
        }
    }
}

// A sound one of a stage's clips played, as a child of that stage.
#[derive(Component)]
pub(crate) struct CueSound;

pub(crate) fn list_clips(
    mut commands: Commands,
    mut stages: Query<(Entity, &ActiveSuite, &Remembered, &mut CharacterState), Added<ActiveSuite>>,
    assets: Res<AssetServer>,
) {
    for (stage, suite, remembered, mut state) in &mut stages {
        let mut sounds = BTreeMap::new();
        let mut clips = BTreeMap::new();
        for clip in &suite.clips {
            for cue in &clip.cues {
                let load = || assets.load(asset_path(suite, &cue.sound));
                sounds.entry(cue.sound.clone()).or_insert_with(load);
            }
            let file = assets.load(asset_path(suite, &clip.json));
            let entry = Clip {
                mode: clip.mode,
                gaze: clip.gaze,
                reaction_only: clip.reaction_only,
                file,
                cues: clip.cues.clone(),
            };
            clips.insert(clip.name.clone(), entry);
        }
        let expressions = (suite.expressions.iter())
            .map(|e| (e.name.clone(), assets.load(asset_path(suite, &e.json))))
            .collect();
        // A Once clip would play at start and leave nothing under it.
        let offered = |name: &str| {
            clips.get(name).is_some_and(|c| c.mode != ClipMode::Once && !c.reaction_only)
        };
        state.animation =
            prefer(suite.remembered(remembered).animation, offered, suite.default_clip.clone());
        commands.entity(stage).insert(CharacterClips { clips, expressions, sounds });
    }
}

// The rig is every named node of the armature, at rest as the file left it: nothing has
// animated it yet.
pub(crate) fn build_actor(
    mut commands: Commands,
    armatures: Query<Entity, (With<Armature>, With<InstanceReady>, Without<CharacterActor>)>,
    children: Query<&Children>,
    nodes: Query<(&Name, &Transform)>,
) {
    for armature in &armatures {
        let mut rig = Rig::default();
        let mut entities = Vec::new();
        for node in children.iter_descendants(armature) {
            let Ok((name, transform)) = nodes.get(node) else {
                continue;
            };
            let rest = charpaper_anim::Transform {
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
            };
            rig.nodes.push((name.to_string(), rest));
            entities.push(node);
        }
        let actor = CharacterActor {
            actor: Actor::new(rig, SLOTS, CROSSFADE.as_secs_f32()),
            nodes: entities,
            ids: BTreeMap::new(),
            expressions: BTreeMap::new(),
            chosen: None,
            under: None,
            face: None,
            events: Vec::new(),
        };
        commands.entity(armature).insert(actor);
    }
}

pub(crate) fn add_loaded_clips(
    mut armatures: Query<(Entity, &mut CharacterActor)>,
    stage_of: StageOf,
    stages: Query<&CharacterClips>,
    files: Res<Assets<ClipAsset>>,
) {
    for (armature, mut actor) in &mut armatures {
        let Some(clips) = stage_of.get(armature).and_then(|stage| stages.get(stage).ok()) else {
            continue;
        };
        let actor = &mut *actor;
        let wanted = (clips.clips.iter().map(|(name, clip)| (name, &clip.file, false)))
            .chain(clips.expressions.iter().map(|(name, file)| (name, file, true)));
        for (name, file, expression) in wanted {
            let ids = if expression { &mut actor.expressions } else { &mut actor.ids };
            if ids.contains_key(name) {
                continue;
            }
            if let Some(file) = files.get(file) {
                ids.insert(name.clone(), actor.actor.add_clip(file.0.clone()));
            }
        }
    }
}

// A pick whose clip is still loading is tried again next frame.
pub(crate) fn play_selected(
    mut commands: Commands,
    mut armatures: Query<(Entity, &mut CharacterActor)>,
    stage_of: StageOf,
    stages: Query<(&CharacterState, &CharacterClips)>,
    sounds: Query<(Entity, &ChildOf), (With<CueSound>, Without<FadeOut>)>,
) {
    for (armature, mut actor) in &mut armatures {
        let Some(stage) = stage_of.get(armature) else {
            continue;
        };
        let Ok((state, clips)) = stages.get(stage) else {
            continue;
        };
        if actor.chosen == state.animation {
            continue;
        }
        let actor = &mut *actor;
        match state.animation.as_deref().and_then(|name| Some((name, clips.get(name)?))) {
            None => {
                actor.actor.stop(BODY);
                actor.actor.stop(BODY_ONCE);
                actor.under = None;
            }
            Some((name, clip)) => {
                let Some(&id) = actor.ids.get(name) else {
                    continue;
                };
                let mode = match clip.mode {
                    ClipMode::Timeline => Mode::Timeline,
                    ClipMode::Hold => Mode::Hold,
                    ClipMode::Once => Mode::Once,
                };
                if mode == Mode::Once {
                    actor.actor.play(BODY_ONCE, id, mode);
                } else {
                    actor.actor.play(BODY, id, mode);
                    actor.actor.stop(BODY_ONCE);
                    actor.under = Some(name.to_string());
                }
            }
        }
        actor.chosen = state.animation.clone();
        // Every sound stops with the pick that played it, fading out with its clip.
        for (sound, parent) in &sounds {
            if parent.parent() == stage {
                commands.entity(sound).insert(FadeOut(CROSSFADE));
            }
        }
    }
}

// The expressions the worn skin can play are those keying at least one of its shape keys.
pub(crate) fn play_expression(
    mut armatures: Query<(Entity, &mut CharacterActor)>,
    stage_of: StageOf,
    mut stages: Query<(&CharacterState, &ShownSkin, &mut Expressions)>,
    skins: Query<&SkinKeys>,
) {
    for (armature, mut actor) in &mut armatures {
        let Some(stage) = stage_of.get(armature) else {
            continue;
        };
        let Ok((state, shown, mut offered)) = stages.get_mut(stage) else {
            continue;
        };
        let Some(keys) = shown.root.and_then(|root| skins.get(root).ok()) else {
            continue;
        };
        let actor = &mut *actor;
        let playable: Vec<String> = (actor.expressions.iter())
            .filter(|(_, id)| {
                let clip = actor.actor.clip(**id);
                clip.shape_keys.iter().any(|(name, _)| keys.names.contains(name))
            })
            .map(|(name, _)| name.clone())
            .collect();
        if offered.0 != playable {
            offered.0 = playable;
        }

        let wanted = state.expression().filter(|name| offered.0.iter().any(|e| e == name));
        if wanted == actor.face.as_deref() {
            continue;
        }
        match wanted.and_then(|name| actor.expressions.get(name)) {
            Some(&id) => actor.actor.play(FACE, id, Mode::Timeline),
            None => actor.actor.stop(FACE),
        }
        actor.face = wanted.map(str::to_string);
    }
}

// Runs before Bevy's own animation and gaze, which aims the head from the pose written here.
pub(crate) fn mix_character(
    mut commands: Commands,
    time: Res<Time>,
    hush: Res<Hush>,
    mut armatures: Query<(Entity, &mut CharacterActor)>,
    stage_of: StageOf,
    mut stages: Query<(&Timeline, &CharacterClips, &mut CharacterState)>,
    mut transforms: Query<&mut Transform>,
) {
    for (armature, mut actor) in &mut armatures {
        let Some(stage) = stage_of.get(armature) else {
            continue;
        };
        let Ok((timeline, clips, mut state)) = stages.get_mut(stage) else {
            continue;
        };
        let actor = &mut *actor;
        actor.events.clear();
        actor.actor.update(timeline.seconds, time.delta_secs(), &mut actor.events);

        for (&entity, pose) in actor.nodes.iter().zip(&actor.actor.pose().nodes) {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                let posed = Transform {
                    translation: pose.translation,
                    rotation: pose.rotation,
                    scale: pose.scale,
                };
                transform.set_if_neq(posed);
            }
        }

        for event in &actor.events {
            match *event {
                Event::Finished { slot: BODY_ONCE, .. } => {
                    state.animation = actor.under.clone();
                    actor.chosen = actor.under.clone();
                }
                Event::Passed { clip, .. } if !hush.0 => {
                    let name = actor.ids.iter().find(|(_, id)| **id == clip).map(|(name, _)| name);
                    let Some(played) = name.and_then(|name| clips.get(name)) else {
                        continue;
                    };
                    for cue in due(&played.cues, event) {
                        let Some(source) = clips.sounds.get(&cue.sound) else {
                            continue;
                        };
                        commands.spawn((
                            CueSound,
                            ChildOf(stage),
                            Sound {
                                source: source.clone(),
                                layer: Layer::Character,
                                looped: false,
                                fade_in: Duration::ZERO,
                            },
                        ));
                    }
                }
                Event::Passed { .. } | Event::Finished { .. } => {}
            }
        }
    }
}
