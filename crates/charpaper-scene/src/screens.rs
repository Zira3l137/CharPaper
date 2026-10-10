use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// A window the scene is shown in. The app spawns one per window; the scene gives each a
// camera of its own, at that window's size, all looking from the one scene view. Despawning
// the entity takes all of that away again.
#[derive(Component, Debug, Clone, Copy)]
pub struct Screen {
    pub window: Entity,
}

// What the panel needs to show and change where the wallpaper is. The app fills these in and
// acts on them; they live here only so the panel, which can't see the app, can reach them.

// Where the wallpaper goes. None is the primary monitor, whichever one that is at the time.
// A chosen monitor that is unplugged stays chosen: the primary one stands in until it is back.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
}

// The monitors connected now.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct Monitors(pub Vec<MonitorEntry>);

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorEntry {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

impl Monitors {
    pub fn get(&self, id: &str) -> Option<&MonitorEntry> {
        self.0.iter().find(|monitor| monitor.id == id)
    }
}
