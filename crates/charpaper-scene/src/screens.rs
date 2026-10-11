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

// Which monitors the wallpaper shows on. None chosen is the primary monitor alone, whichever
// one that is at the time. A chosen monitor that is unplugged stays chosen and comes back with
// its monitor; while none of the chosen ones is connected, the primary one stands in.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenSettings {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub monitors: Vec<ChosenMonitor>,
    // One wide picture across the monitors, rather than the whole picture on each.
    pub span: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChosenMonitor {
    pub id: String,
    // Only to name it in the panel while it is unplugged.
    pub name: String,
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
    // The wallpaper is on it now.
    pub shown: bool,
}

impl Monitors {
    pub fn get(&self, id: &str) -> Option<&MonitorEntry> {
        self.0.iter().find(|monitor| monitor.id == id)
    }
}
