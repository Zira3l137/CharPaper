// The actors hotspots find bones on and play clips on, and what they play there.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use charpaper_anim::Actor;
use charpaper_anim::ClipId;
use charpaper_anim::Mode;
use charpaper_suite::ActorClip;

use super::Reactions;
use crate::CharacterState;
use crate::character::CharacterActor;
use crate::character::CharacterClips;
use crate::character::REACTIONS as CHARACTER_REACTIONS;
use crate::environment::EnvironmentReady;
use crate::props::PropActor;
use crate::props::REACTIONS as PROP_REACTIONS;
use crate::props::base_mode;
use crate::stage::StageOf;

// A clip a hotspot started, and whether the character may follow the cursor during it.
pub(super) struct Played {
    actor: Entity,
    slot: usize,
    clip: ClipId,
    pub(super) mode: Mode,
    pub(super) gaze: bool,
}

// Where a clip a hotspot plays goes: on and on in its reaction slot or a layer, or once there,
// or once above its `while` clips.
#[derive(Clone, Copy)]
pub(super) enum Slot {
    Loop(bool),
    Once(bool),
    Top,
}

// Each actor on its stage, and the shown environment, for armatures that aren't actors.
#[derive(SystemParam)]
pub(super) struct Actors<'w, 's> {
    characters: Query<'w, 's, (Entity, &'static mut CharacterActor)>,
    props: Query<'w, 's, (Entity, &'static mut PropActor)>,
    pub(super) stage_of: StageOf<'w, 's>,
    environments: Query<'w, 's, (Entity, &'static ChildOf), With<EnvironmentReady>>,
    children: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
}

impl Actors<'_, '_> {
    pub(super) fn character(&mut self, stage: Entity) -> Option<(Entity, Mut<'_, CharacterActor>)> {
        let stage_of = &self.stage_of;
        self.characters.iter_mut().find(|(entity, _)| stage_of.get(*entity) == Some(stage))
    }

    // An environment's actor by name. Only the shown environment has any.
    fn prop(&mut self, stage: Entity, name: &str) -> Option<(Entity, Mut<'_, PropActor>)> {
        let stage_of = &self.stage_of;
        let mut props = self.props.iter_mut();
        props.find(|(entity, prop)| prop.name() == name && stage_of.get(*entity) == Some(stage))
    }

    // The stage's environment, once its scene has spawned.
    pub(super) fn environment(&self, stage: Entity) -> Option<Entity> {
        let mut roots = self.environments.iter();
        roots.find(|(_, parent)| parent.parent() == stage).map(|(root, _)| root)
    }

    pub(super) fn bone(
        &mut self,
        stage: Entity,
        actor: Option<&str>,
        bone: &str,
    ) -> Option<Entity> {
        let Some(name) = actor else {
            return self.character(stage)?.1.node(bone);
        };
        if let Some(found) = self.prop(stage, name).and_then(|(_, prop)| prop.node(bone)) {
            return Some(found);
        }
        let named = |entity: Entity, wanted: &str| {
            self.names.get(entity).is_ok_and(|own| own.as_str() == wanted)
        };
        let root = self.environment(stage)?;
        let armature = self.children.iter_descendants(root).find(|&e| named(e, name))?;
        self.children.iter_descendants(armature).find(|&e| named(e, bone))
    }

    // What the actor plays on and on.
    fn base(&mut self, stage: Entity, actor: Option<&str>) -> Option<String> {
        match actor {
            None => self.character(stage)?.1.base().map(str::to_owned),
            Some(name) => self.prop(stage, name)?.1.base().map(str::to_owned),
        }
    }

    // Whether every actor it waits on plays one of the animations it waits for.
    pub(super) fn allows(&mut self, stage: Entity, during: &[ActorClip]) -> bool {
        during.iter().all(|wanted| {
            let base = self.base(stage, wanted.actor.as_deref());
            let fits = |other: &&ActorClip| other.actor == wanted.actor;
            during.iter().filter(fits).any(|other| base.as_deref() == Some(other.clip.as_str()))
        })
    }

    pub(super) fn still_playing(&self, played: &Played) -> bool {
        let mixer = match self.characters.get(played.actor) {
            Ok((_, character)) => character.mixer(),
            Err(_) => match self.props.get(played.actor) {
                Ok((_, prop)) => prop.mixer(),
                Err(_) => return false,
            },
        };
        mixer.playing(played.slot) == Some((played.clip, played.mode))
    }

    // Fades out a clip a hotspot played, unless something else has taken its slot since.
    pub(super) fn stop(&mut self, played: &Played) {
        let mixer = match self.characters.get_mut(played.actor) {
            Ok((_, character)) => character.into_inner().mixer_mut(),
            Err(_) => match self.props.get_mut(played.actor) {
                Ok((_, prop)) => prop.into_inner().mixer_mut(),
                Err(_) => return,
            },
        };
        if mixer.playing(played.slot) == Some((played.clip, played.mode)) {
            mixer.stop(played.slot);
        }
    }

    pub(super) fn start(
        &mut self,
        stage: Entity,
        actor: Option<&str>,
        name: &str,
        slot: Slot,
        clips: Option<&CharacterClips>,
    ) -> Option<Played> {
        let place = |reactions: &Reactions, mixer: &mut Actor, id, mode: Option<Mode>| {
            Some(match slot {
                Slot::Loop(layer) => (reactions.play(mixer, id, mode?, layer), mode?),
                Slot::Once(layer) => (reactions.play(mixer, id, Mode::Once, layer), Mode::Once),
                Slot::Top => (reactions.top(mixer, id), Mode::Once),
            })
        };
        match actor {
            None => {
                let (entity, mut character) = self.character(stage)?;
                let id = character.clip_id(name)?;
                let clip = clips.and_then(|clips| clips.get(name));
                let mode = clip.and_then(|clip| base_mode(clip.mode));
                let (slot, mode) = place(&CHARACTER_REACTIONS, character.mixer_mut(), id, mode)?;
                let gaze = clip.is_none_or(|clip| clip.gaze);
                Some(Played { actor: entity, slot, clip: id, mode, gaze })
            }
            Some(owner) => {
                let (entity, mut prop) = self.prop(stage, owner)?;
                let id = prop.clip_id(name)?;
                let mode = prop.clip_mode(name).and_then(base_mode);
                let (slot, mode) = place(&PROP_REACTIONS, prop.mixer_mut(), id, mode)?;
                Some(Played { actor: entity, slot, clip: id, mode, gaze: true })
            }
        }
    }

    // Switches each actor to its `then` clip: the character as if the viewer had picked it.
    pub(super) fn switch(
        &mut self,
        stage: Entity,
        then: &[ActorClip],
        mut state: Option<&mut CharacterState>,
        clips: Option<&CharacterClips>,
    ) {
        for wanted in then {
            match wanted.actor.as_deref() {
                None => {
                    if let (Some(state), Some(clips), Some((_, mut character))) =
                        (state.as_deref_mut(), clips, self.character(stage))
                        && character.cut_to(&wanted.clip, clips)
                    {
                        state.animation = Some(wanted.clip.clone());
                    }
                }
                Some(name) => {
                    if let Some((_, mut prop)) = self.prop(stage, name) {
                        prop.switch(&wanted.clip);
                    }
                }
            }
        }
    }
}
