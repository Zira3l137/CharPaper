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
    pub theme: UiTheme,
}

// How the panel looks. The performance monitor follows it too.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiTheme {
    pub palette: Palette,
    pub accent: Rgb,
    // How solid the panel's background is, from 0 to 1.
    pub opacity: f32,
    // How big everything is drawn, 1 being as designed.
    pub size: f32,
    pub side: PanelSide,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            palette: Palette::Slate,
            accent: Rgb::DEFAULT_ACCENT,
            opacity: 0.95,
            size: 1.0,
            side: PanelSide::Right,
        }
    }
}

// The colors everything but the accent comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Palette {
    #[default]
    Slate,
    Midnight,
    Light,
}

impl Palette {
    pub const ALL: [Self; 3] = [Self::Slate, Self::Midnight, Self::Light];
}

// The edge of the screen the panel keeps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelSide {
    Left,
    #[default]
    Right,
}

impl PanelSide {
    pub const ALL: [Self; 2] = [Self::Left, Self::Right];
}

// A color as it's written in state.toml: "#5AA6F2".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    pub const DEFAULT_ACCENT: Self = Self([0x5A, 0xA6, 0xF2]);

    pub fn hex(self) -> String {
        let [r, g, b] = self.0;
        format!("#{r:02X}{g:02X}{b:02X}")
    }

    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.trim().strip_prefix('#')?;
        if digits.len() != 6 {
            return None;
        }
        let channel = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
        Some(Self([channel(0)?, channel(2)?, channel(4)?]))
    }
}

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex())
    }
}

// A mistyped color falls back to the default accent instead of failing: a value the app can't
// read makes it put the whole of state.toml aside.
impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Ok(Self::parse(&text).unwrap_or(Self::DEFAULT_ACCENT))
    }
}
