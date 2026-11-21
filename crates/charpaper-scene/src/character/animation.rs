use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::state::CharacterState;
use crate::state::prefer;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;

// The character's clips as suite.toml and the clips folder name them, for the panel's picker
// and for gaze.
#[derive(Component, Default)]
pub struct CharacterClips {
    clips: BTreeMap<String, Clip>,
}

pub(crate) struct Clip {
    pub gaze: bool,
}

impl CharacterClips {
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.clips.keys().map(String::as_str)
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Clip> {
        self.clips.get(name)
    }
}

pub(crate) fn list_clips(
    mut commands: Commands,
    mut stages: Query<(Entity, &ActiveSuite, &Remembered, &mut CharacterState), Added<ActiveSuite>>,
) {
    for (stage, suite, remembered, mut state) in &mut stages {
        let clips: BTreeMap<String, Clip> =
            suite.clips.iter().map(|clip| (clip.name.clone(), Clip { gaze: clip.gaze })).collect();
        state.animation = prefer(
            suite.remembered(remembered).animation,
            |name| clips.contains_key(name),
            suite.default_clip.clone(),
        );
        commands.entity(stage).insert(CharacterClips { clips });
    }
}
