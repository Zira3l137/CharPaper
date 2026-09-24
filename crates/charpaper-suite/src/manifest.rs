//! `suite.toml`, exactly as written.
//!
//! Every `Option` means "this suite has no opinion". Keeping that distinct
//! from a concrete value is what lets engine defaults, suite values and the
//! user's own overrides stack on top of each other later.
//!
//! `deny_unknown_fields` everywhere: these files are written by hand, and a
//! misspelt key silently doing nothing is worse than a load error.
//!
//! Doc comments here double as the descriptions editors show on hover through
//! the generated JSON Schema, so they are written for the suite's author.
//! Notes meant for us stay in `//` comments.

use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;

pub const MANIFEST_FILE: &str = "suite.toml";

/// Bumped only for changes old files cannot be read under. Adding an optional
/// key does not need a bump.
pub const SCHEMA_VERSION: u32 = 1;

/// A CharPaper character suite: the `suite.toml` at the root of the suite's
/// folder.
#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
#[schemars(title = "CharPaper suite manifest")]
pub struct Manifest {
    /// Version of this file's format. Use 1.
    #[schemars(range(min = 1, max = SCHEMA_VERSION))]
    pub schema: u32,
    /// Name shown in the app. Defaults to the folder name.
    pub name: Option<String>,
    #[serde(default)]
    pub character: CharacterSection,
    /// Named animations, used to rename, pick or change how clips play. Files
    /// in `animations/` that are not mentioned here are used whole, each clip
    /// under its own name. Once a file is mentioned, only the clips listed
    /// for it are used.
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

/// The character itself.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CharacterSection {
    /// Path to the model (the armature), relative to this file. Only needed
    /// when more than one .glb/.gltf sits next to this file.
    pub model: Option<PathBuf>,
    /// Skin worn at start: a file name in `skins/` without its extension.
    /// Defaults to the first skin in name order.
    pub default_skin: Option<String>,
    /// Animation played at start. Without one the character holds its rest
    /// pose.
    pub default_animation: Option<String>,
}

/// One named animation.
#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ClipEntry {
    /// Path to the file holding the clip, relative to this file.
    pub file: PathBuf,
    /// The clip's name inside the file (the Blender action name). May be left
    /// out when the file holds exactly one clip.
    pub clip: Option<String>,
    #[serde(default)]
    pub mode: PlayMode,
}

/// How the animation plays. Defaults to `loop`.
#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    /// Repeats forever.
    #[default]
    Loop,
    /// Plays once, then returns to the default animation.
    Once,
    /// Holds the clip's first frame.
    Pose,
}

/// What surrounds the character.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentSection {
    /// A .glb/.gltf scene, relative to this file. Defaults to the only one in
    /// `environment/`, if any.
    pub scene: Option<PathBuf>,
    /// A .ktx2 cubemap, relative to this file. Defaults to the only one in
    /// `environment/`, if any.
    pub skybox: Option<PathBuf>,
}

/// Scene lighting. Anything left out uses the app's defaults.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub sun: Option<Sun>,
    pub ambient: Option<Ambient>,
}

/// A distant light shining in one direction everywhere, like the sun.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Sun {
    /// Brightness in lux. Overcast daylight is about 1000, direct sunlight
    /// about 100000.
    pub illuminance: Option<f32>,
    /// sRGB color as [r, g, b], each 0.0 to 1.0.
    pub color: Option<[f32; 3]>,
    /// The direction the light travels as [x, y, z], not where it comes from.
    /// Y is up.
    pub direction: Option<[f32; 3]>,
    pub shadows: Option<bool>,
}

/// Light that reaches everything evenly, so shadowed areas are not black.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Ambient {
    /// sRGB color as [r, g, b], each 0.0 to 1.0.
    pub color: Option<[f32; 3]>,
    pub brightness: Option<f32>,
}

/// Effects applied to the finished image.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Post {
    pub tonemapping: Option<Tonemapping>,
    /// Exposure compensation in EV stops; positive is brighter.
    pub exposure: Option<f32>,
    /// Bloom intensity; 0.0 turns bloom off.
    pub bloom: Option<f32>,
}

// Mirrors Bevy's `Tonemapping` so this crate does not need Bevy.
/// How bright colors are squeezed into what the screen can show.
#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tonemapping {
    None,
    Reinhard,
    ReinhardLuminance,
    AcesFitted,
    Agx,
    SomewhatBoringDisplayTransform,
    TonyMcMapface,
    /// Blender's "Filmic" view transform.
    BlenderFilmic,
}

/// Where the camera starts. The user can still orbit and zoom.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    /// The point the camera orbits around, as [x, y, z]. Y is up.
    pub focus: Option<[f32; 3]>,
    /// Distance from the focus point.
    pub radius: Option<f32>,
    /// Angle around the vertical axis, in degrees.
    pub yaw_deg: Option<f32>,
    /// Angle above the horizon, in degrees.
    pub pitch_deg: Option<f32>,
}
