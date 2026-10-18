use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// Only tabs with something working behind them exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tab {
    #[default]
    Scene,
    Graphics,
    Sound,
    System,
}

// Saved between runs by the app, so the panel reopens as it was left.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    pub is_menu_closed: bool,
    pub tab: Tab,
    // A tag such as `de-DE`, for a translation in `locales/`. None follows the system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    // Whether the viewer folded a section, by its key. A section missing here starts as it
    // does by default. Keyed by plain names rather than Section itself, so a section renamed
    // in a later version is ignored instead of making the whole file unreadable.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub folded: BTreeMap<String, bool>,
    pub performance_monitor: bool,
}
