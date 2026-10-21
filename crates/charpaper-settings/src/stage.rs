use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bevy::prelude::*;
use charpaper_suite::LookEdits;
use serde::Deserialize;
use serde::Serialize;

use crate::render::Graphics;
use crate::screen::StageId;

// What a stage keeps between runs, and while none of its monitors is connected.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct StageSeed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<String>,
    pub gaze: GazeSettings,
    pub view: ViewSettings,
    // The picks per suite, by the suite's folder.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub suites: BTreeMap<String, Picks>,
    // The edits to each suite's look, by the suite's folder.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub looks: BTreeMap<String, LookEdits>,
    pub graphics: Graphics,
}

// Every stage's seed, filled from state.toml and brought up to date as a stage goes away.
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(transparent)]
pub struct StageSeeds(pub BTreeMap<StageId, StageSeed>);

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GazeSettings {
    pub follow_cursor: bool,
}

#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewSettings {
    pub locked: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self { locked: false }
    }
}

impl Default for GazeSettings {
    fn default() -> Self {
        Self { follow_cursor: true }
    }
}

// What the viewer picked in one suite on a stage.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Picks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    // ORBIT_CAMERA for the orbit camera, so None can mean "no preference".
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
}
