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
    /// Settings for each environment, keyed by its name: the `.glb` file name
    /// in `environment/` without its extension, or the name of its folder.
    #[serde(default)]
    pub environments: BTreeMap<String, EnvironmentEntry>,
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
    /// Environment shown at start. Defaults to the first in name order.
    pub default: Option<String>,
}

/// One environment's settings. Its lights come from its own `.glb`; these
/// only adjust how the scene is shown while it is.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentEntry {
    /// How bright the skybox looks and how strongly the reflection maps light
    /// the scene, in cd/m². Defaults to 1000.
    pub brightness: Option<f32>,
    /// Whether the environment's lights cast shadows. glTF cannot say, so it
    /// is set here. Defaults to true.
    pub shadows: Option<bool>,
    /// Exposure compensation in EV stops while this environment is shown,
    /// replacing `post.exposure`. A sunlit scene needs far less than a room.
    pub exposure: Option<f32>,
}

/// Effects applied to the finished image.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
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

impl Tonemapping {
    pub const ALL: [Tonemapping; 8] = [
        Tonemapping::None,
        Tonemapping::Reinhard,
        Tonemapping::ReinhardLuminance,
        Tonemapping::AcesFitted,
        Tonemapping::Agx,
        Tonemapping::SomewhatBoringDisplayTransform,
        Tonemapping::TonyMcMapface,
        Tonemapping::BlenderFilmic,
    ];

    /// The name `suite.toml` uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Tonemapping::None => "none",
            Tonemapping::Reinhard => "reinhard",
            Tonemapping::ReinhardLuminance => "reinhard_luminance",
            Tonemapping::AcesFitted => "aces_fitted",
            Tonemapping::Agx => "agx",
            Tonemapping::SomewhatBoringDisplayTransform => "somewhat_boring_display_transform",
            Tonemapping::TonyMcMapface => "tony_mc_mapface",
            Tonemapping::BlenderFilmic => "blender_filmic",
        }
    }
}

/// Which camera the app starts with, and where the built-in orbit camera
/// starts. The user can switch cameras, and orbit and zoom the orbit camera.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    /// Camera active at start: a file name in `cameras/` without its
    /// extension, or `orbit` for the built-in orbit camera. Defaults to
    /// `orbit`.
    pub default: Option<String>,
    /// The point the orbit camera orbits around, as [x, y, z]. Y is up.
    pub focus: Option<[f32; 3]>,
    /// The orbit camera's distance from the focus point.
    pub radius: Option<f32>,
    /// Angle around the vertical axis, in degrees.
    pub yaw_deg: Option<f32>,
    /// Angle above the horizon, in degrees.
    pub pitch_deg: Option<f32>,
}
