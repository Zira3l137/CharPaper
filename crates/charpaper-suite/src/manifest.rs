//! `suite.toml`, exactly as written.
//!
//! Every `Option` means "this suite has no opinion". Keeping that distinct
//! from a concrete value is what lets engine defaults, suite values and the
//! user's own overrides stack on top of each other later.
//!
//! `deny_unknown_fields` everywhere: these files are written by hand, and a
//! misspelt key silently doing nothing is worse than a load error.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

pub const MANIFEST_FILE: &str = "suite.toml";

/// Bumped only for changes old files cannot be read under. Adding an optional
/// key does not need a bump.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    /// Defaults to the folder name.
    pub name: Option<String>,
    #[serde(default)]
    pub character: CharacterSection,
    /// Keyed by the name the UI and `character.default_animation` use.
    #[serde(default)]
    pub animations: BTreeMap<String, ClipEntry>,
    #[serde(default)]
    pub environment: EnvironmentSection,
    #[serde(default)]
    pub lighting: Lighting,
    #[serde(default)]
    pub post: Post,
    #[serde(default)]
    pub camera: Camera,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CharacterSection {
    pub model: Option<PathBuf>,
    /// Defaults to the first skin in name order.
    pub default_skin: Option<String>,
    /// Without one the character holds its rest pose.
    pub default_animation: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ClipEntry {
    pub file: PathBuf,
    /// The clip's name inside the file. May be left out when the file holds
    /// exactly one clip.
    pub clip: Option<String>,
    #[serde(default)]
    pub mode: PlayMode,
}

#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    #[default]
    Loop,
    /// Plays once, then returns to the default animation.
    Once,
    /// Holds the clip's first frame.
    Pose,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentSection {
    pub scene: Option<PathBuf>,
    pub skybox: Option<PathBuf>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub sun: Option<Sun>,
    pub ambient: Option<Ambient>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Sun {
    /// Lux.
    pub illuminance: Option<f32>,
    /// sRGB, 0.0-1.0.
    pub color: Option<[f32; 3]>,
    /// The direction the light travels, not where it comes from.
    pub direction: Option<[f32; 3]>,
    pub shadows: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Ambient {
    /// sRGB, 0.0-1.0.
    pub color: Option<[f32; 3]>,
    pub brightness: Option<f32>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Post {
    pub tonemapping: Option<Tonemapping>,
    /// Exposure compensation in EV stops; positive is brighter.
    pub exposure: Option<f32>,
    /// Bloom intensity; 0.0 turns bloom off.
    pub bloom: Option<f32>,
}

/// Mirrors Bevy's `Tonemapping` so this crate does not need Bevy.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tonemapping {
    None,
    Reinhard,
    ReinhardLuminance,
    AcesFitted,
    Agx,
    SomewhatBoringDisplayTransform,
    TonyMcMapface,
    BlenderFilmic,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub focus: Option<[f32; 3]>,
    pub radius: Option<f32>,
    pub yaw_deg: Option<f32>,
    pub pitch_deg: Option<f32>,
}
