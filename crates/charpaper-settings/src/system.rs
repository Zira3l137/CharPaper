use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    // Behind the desktop icons.
    #[default]
    Wallpaper,
    // In an ordinary window.
    Windowed,
}

impl DisplayMode {
    pub const ALL: [Self; 2] = [Self::Wallpaper, Self::Windowed];
}

// The app itself rather than what it shows. A changed mode takes effect on the next start,
// which the app makes at once.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemSettings {
    pub mode: DisplayMode,
}

// Whether the app starts when the user signs in. Not in state.toml: the OS keeps it, and the
// app reads it from there, so the panel agrees with the OS's own list of startup apps. None
// where the app can't arrange it.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartAtLogin(pub Option<bool>);
