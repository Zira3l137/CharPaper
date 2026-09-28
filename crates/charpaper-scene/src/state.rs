use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bevy::prelude::*;
use charpaper_suite::ORBIT_CAMERA;
use serde::Deserialize;
use serde::Serialize;

// What the viewer asked for. Systems compare it with what is shown and catch up, so
// writing here is how the UI switches things.
#[derive(Resource, Default, Debug)]
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Picks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    // ORBIT_CAMERA for the orbit camera. Unlike in CharacterState, None means "no preference".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub hidden: BTreeMap<String, BTreeSet<String>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub expressions: BTreeMap<String, String>,
}

impl Picks {
    // An unset choice keeps the older value: the scene fills choices in over several
    // frames, and a gap must not erase what was remembered.
    pub fn merge(&mut self, newer: Picks) {
        let keep = |new: Option<String>, old: &mut Option<String>| *old = new.or(old.take());
        keep(newer.skin, &mut self.skin);
        keep(newer.animation, &mut self.animation);
        keep(newer.camera, &mut self.camera);
        keep(newer.environment, &mut self.environment);
        self.hidden.extend(newer.hidden);
        self.expressions.extend(newer.expressions);
    }

    pub fn from_state(state: &CharacterState) -> Self {
        Self {
            skin: state.skin.clone(),
            animation: state.animation.clone(),
            camera: Some(state.camera.clone().unwrap_or_else(|| ORBIT_CAMERA.to_string())),
            environment: state.environment.clone(),
            hidden: state.hidden.clone(),
            expressions: state.expressions.clone(),
        }
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
