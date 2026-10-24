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
    pub accent: Hsv,
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
            accent: Hsv::DEFAULT_ACCENT,
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

// A color as its hue in degrees, and saturation and value from 0 to 1, which is what the panel
// changes. Kept as these rather than as RGB, which loses the hue and saturation of a color
// turned all the way to black, so turning it back up would start from gray.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Hsv {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
}

impl Hsv {
    // #5AA6F2.
    pub const DEFAULT_ACCENT: Self = Self { hue: 210.0, saturation: 0.63, value: 0.95 };

    pub fn new(hue: f32, saturation: f32, value: f32) -> Self {
        Self {
            hue: hue.rem_euclid(360.0),
            saturation: saturation.clamp(0.0, 1.0),
            value: value.clamp(0.0, 1.0),
        }
    }

    pub fn color(self) -> Color {
        Color::hsv(self.hue, self.saturation, self.value)
    }

    pub fn hex(self) -> String {
        self.color().to_srgba().to_hex()
    }
}

// Also reads the hex string 0.2.6 wrote. Anything else falls back to the default accent instead
// of failing: a value the app can't read makes it put the whole of state.toml aside.
impl<'de> Deserialize<'de> for Hsv {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Parts { hue: f32, saturation: f32, value: f32 },
            Hex(String),
            Other(serde::de::IgnoredAny),
        }
        let color = match Written::deserialize(deserializer)? {
            Written::Parts { hue, saturation, value } => Some(Self::new(hue, saturation, value)),
            Written::Hex(hex) => Srgba::hex(hex).ok().map(|rgb| {
                let hsv = Hsva::from(rgb);
                Self::new(hsv.hue, hsv.saturation, hsv.value)
            }),
            Written::Other(_) => None,
        };
        Ok(color.unwrap_or(Self::DEFAULT_ACCENT))
    }
}
