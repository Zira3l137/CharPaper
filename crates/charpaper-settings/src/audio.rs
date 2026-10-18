use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// From 0 (silent) to 1 (as loud as the file).
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub ambience: f32,
    pub character: f32,
}

// Quiet at first: a wallpaper that suddenly plays loud sound is unwelcome.
impl Default for Volumes {
    fn default() -> Self {
        Self { master: 0.25, music: 1.0, ambience: 1.0, character: 1.0 }
    }
}
