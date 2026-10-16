use std::collections::BTreeSet;
use std::fmt;

use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// A window a stage is shown in. The app spawns one per window; the scene gives each a camera
// of its own, at that window's size, looking from the view of the stage it shows. Despawning
// the entity takes all of that away again.
#[derive(Component, Debug, Clone, Copy)]
pub struct Screen {
    pub window: Entity,
}

// The stage a Screen shows. The app sets it from the screen's monitor.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowsStage(pub StageId);

// Names a stage, as the letters A, B, C… the panel shows. Monitors showing the same stage show
// the same scene.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct StageId(u8);

impl StageId {
    pub const FIRST: Self = Self(0);
    pub const MAX: usize = 26;

    pub fn new(index: usize) -> Option<Self> {
        (index < Self::MAX).then_some(Self(index as u8))
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn letter(self) -> char {
        (b'A' + self.0) as char
    }
}

impl fmt::Display for StageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.letter())
    }
}

impl From<StageId> for String {
    fn from(id: StageId) -> Self {
        id.to_string()
    }
}

impl TryFrom<String> for StageId {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let mut letters = text.chars();
        match (letters.next(), letters.next()) {
            (Some(letter @ 'A'..='Z'), None) => Ok(Self(letter as u8 - b'A')),
            _ => Err(format!("{text:?} is not a stage; stages are named A to Z")),
        }
    }
}

// Where a Screen's window sits on the desktop, in physical pixels. The app keeps it current;
// span mode cuts the one wide picture up by it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ScreenPlace(pub IRect);

// Nobody can see this Screen now, so its camera rests while the others go on.
#[derive(Component, Debug)]
pub struct Unseen;

// The camera a Screen's UI draws through, added once the scene has made it.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScreenUi(pub Entity);

// What the panel needs to show and change where the wallpaper is. The app fills these in and
// acts on them; they live here only so the panel, which can't see the app, can reach them.

// Which monitors the wallpaper shows on, and which stage each shows. None chosen is the primary
// monitor alone, showing stage A, whichever monitor that is at the time. A chosen monitor that
// is unplugged stays chosen and comes back with its monitor; while none of the chosen ones is
// connected, the primary one stands in.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenSettings {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub monitors: Vec<ChosenMonitor>,
    // The stages that spread one wide picture across their monitors, rather than showing the
    // whole picture on each.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub spanned: BTreeSet<StageId>,
}

impl ScreenSettings {
    // The stage a monitor shows while the wallpaper is on it.
    pub fn stage_of(&self, monitor: &str) -> StageId {
        let chosen = self.monitors.iter().find(|chosen| chosen.id == monitor);
        chosen.map_or(StageId::FIRST, |chosen| chosen.stage)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChosenMonitor {
    pub id: String,
    // Only to name it in the panel while it is unplugged.
    pub name: String,
    #[serde(default)]
    pub stage: StageId,
}

// The monitors connected now, in the order they sit on the desk, left to right.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct Monitors(pub Vec<MonitorEntry>);

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorEntry {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    // The stage it shows now, if the wallpaper is on it.
    pub stage: Option<StageId>,
}

impl Monitors {
    pub fn get(&self, id: &str) -> Option<&MonitorEntry> {
        self.0.iter().find(|monitor| monitor.id == id)
    }
}
