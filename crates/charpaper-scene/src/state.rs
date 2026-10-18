use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bevy::prelude::*;
use charpaper_settings::Picks;
use charpaper_suite::ORBIT_CAMERA;

// What the viewer asked for on a stage. Systems compare it with what is shown and catch up,
// so writing here is how the UI switches things.
#[derive(Component, Default, Debug)]
pub struct CharacterState {
    pub suite: Option<String>,
    pub skin: Option<String>,
    pub animation: Option<String>,
    // None is the orbit camera.
    pub camera: Option<String>,
    pub environment: Option<String>,
    pub hidden: BTreeMap<String, BTreeSet<String>>,
    // Per skin. Neutral is an empty name, so a merge still overwrites an older pick.
    pub expressions: BTreeMap<String, String>,
}

impl CharacterState {
    pub fn expression(&self) -> Option<&str> {
        let skin = self.skin.as_ref()?;
        self.expressions.get(skin).map(String::as_str).filter(|e| !e.is_empty())
    }
}

// What a stage remembers of its picks in a suite, for when it comes back to that suite.
pub(crate) fn picks_of(state: &CharacterState) -> Picks {
    Picks {
        skin: state.skin.clone(),
        animation: state.animation.clone(),
        camera: Some(state.camera.clone().unwrap_or_else(|| ORBIT_CAMERA.to_string())),
        environment: state.environment.clone(),
        hidden: state.hidden.clone(),
        expressions: state.expressions.clone(),
    }
}

pub(crate) fn prefer(
    remembered: Option<String>,
    offered: impl Fn(&str) -> bool,
    default: Option<String>,
) -> Option<String> {
    match remembered {
        Some(pick) if offered(&pick) => Some(pick),
        Some(pick) => {
            debug!("remembered choice {pick:?} is no longer offered; using the default");
            default
        }
        None => default,
    }
}
