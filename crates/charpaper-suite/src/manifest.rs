use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;

// Doc comments in this file become the hover text of the generated JSON Schema, so they
// are written for suite authors. `deny_unknown_fields` everywhere: a misspelt key that
// silently does nothing is worse than an error.

pub const MANIFEST_FILE: &str = "suite.toml";

// Bumped only for changes old files can't be read under, not for new optional keys.
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
    /// Settings for each exported camera, keyed by its name: the file name in
    /// `cameras/` without its extension, or the camera's own name in a file
    /// that holds several.
    #[serde(default)]
    pub cameras: BTreeMap<String, CameraEntry>,
    /// Makes the character look towards the mouse cursor. Without this
    /// section she never does.
    pub gaze: Option<Gaze>,
}

/// The character itself.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CharacterSection {
    /// Path to the model (the armature), relative to this file. Only needed
    /// when more than one .glb/.gltf sits next to this file. Without a model
    /// file at all, the armature comes from the default skin, since every
    /// skin carries a copy of it.
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
    /// Whether the character may look towards the cursor while this plays.
    /// Defaults to true. Turn it off where she has to look somewhere on
    /// purpose, like a dance or a sleeping pose.
    pub gaze: Option<bool>,
}

/// Looking towards the mouse cursor. The eyes turn first; the head and neck
/// take over what the eyes cannot reach. Every turn is added on top of the
/// animation playing, and angles are measured from where it has her look.
#[derive(Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Gaze {
    /// The eye bones, usually two.
    pub eyes: Vec<String>,
    /// The head bone. Without it, only the neck and eyes turn.
    pub head: Option<String>,
    /// The neck bone. Without it, the head takes all of what the eyes cannot
    /// reach.
    pub neck: Option<String>,
    /// How far the eyes turn sideways, in degrees. Defaults to 30.
    pub eye_yaw_deg: Option<f32>,
    /// How far the eyes turn up or down, in degrees. Defaults to 20.
    pub eye_pitch_deg: Option<f32>,
    /// How far the head turns sideways, in degrees. Defaults to 50.
    pub head_yaw_deg: Option<f32>,
    /// How far the head turns up or down, in degrees. Defaults to 25.
    pub head_pitch_deg: Option<f32>,
    /// How far the neck turns sideways, in degrees. Defaults to 25.
    pub neck_yaw_deg: Option<f32>,
    /// How far the neck turns up or down, in degrees. Defaults to 15.
    pub neck_pitch_deg: Option<f32>,
    /// With both a head and a neck: the head's share of what the eyes cannot
    /// reach, from 0 to 1. The neck takes the rest. Defaults to 0.7.
    pub head_share: Option<f32>,
    /// Shape keys that follow the eyes, such as eyelids. Each is set in
    /// proportion to how far the eyes turn that way, reaching 1 at the eyes'
    /// limit. Any may be left out, and a skin without one ignores it.
    #[serde(default)]
    pub shape_keys: GazeShapeKeys,
}

impl Gaze {
    pub fn eye_limits_deg(&self) -> [f32; 2] {
        [self.eye_yaw_deg.unwrap_or(30.0), self.eye_pitch_deg.unwrap_or(20.0)]
    }

    pub fn head_limits_deg(&self) -> [f32; 2] {
        [self.head_yaw_deg.unwrap_or(50.0), self.head_pitch_deg.unwrap_or(25.0)]
    }

    pub fn neck_limits_deg(&self) -> [f32; 2] {
        [self.neck_yaw_deg.unwrap_or(25.0), self.neck_pitch_deg.unwrap_or(15.0)]
    }

    pub fn head_share(&self) -> f32 {
        self.head_share.unwrap_or(0.7).clamp(0.0, 1.0)
    }

    /// Every bone the section names.
    pub fn bones(&self) -> impl Iterator<Item = &str> {
        self.eyes.iter().map(String::as_str).chain(self.head.as_deref()).chain(self.neck.as_deref())
    }
}

/// Shape keys set as the eyes turn, by direction. Left and right are the
/// character's own, as in Blender's `.L` and `.R`.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GazeShapeKeys {
    pub left: Option<String>,
    pub right: Option<String>,
    pub up: Option<String>,
    pub down: Option<String>,
}

impl GazeShapeKeys {
    pub fn named(&self) -> impl Iterator<Item = &str> {
        [&self.left, &self.right, &self.up, &self.down].into_iter().flatten().map(String::as_str)
    }
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
    /// Boxes of fog, keyed by the name of their object in the environment's
    /// file: in Blender, usually a cube with a Principled Volume material. The
    /// fog fills the object's bounding box, or the 2 m cube Blender draws for
    /// an empty, and moves, turns and scales with it. Only lights light fog,
    /// not the sky.
    #[serde(default)]
    pub fog: BTreeMap<String, Fog>,
    /// Lights that do not light the fog, by object name, like a Blender light
    /// whose Volume Scatter is 0. Every other light does. A sun only lights
    /// fog while it casts shadows.
    #[serde(default)]
    pub no_volume_scatter: Vec<String>,
}

/// A box of fog, set up like Blender's Principled Volume. Colors are linear,
/// as the node shows them.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Fog {
    /// The color light takes on as the fog scatters it. Defaults to
    /// [0.5, 0.5, 0.5].
    pub color: Option<[f32; 3]>,
    /// How thick the fog is. Defaults to 1.
    pub density: Option<f32>,
    /// From -1 to 1. Above 0, light keeps going the way it was heading, so
    /// the fog glows most when looking towards a light. Defaults to 0.
    pub anisotropy: Option<f32>,
    /// The fog absorbs the light this color lacks. Defaults to black.
    pub absorption_color: Option<[f32; 3]>,
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
    /// Color fringes that grow towards the edges of the image, as a fraction
    /// of its size. 0.0 turns them off; 0.005 is subtle.
    pub chromatic_aberration: Option<f32>,
    #[serde(default)]
    pub vignette: Vignette,
    #[serde(default)]
    pub grain: Grain,
}

/// Darkens the edges of the image, the way a lens does towards its rim.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Vignette {
    /// How dark the edges get, from 0 (off) to 1. Defaults to 0.
    pub intensity: Option<f32>,
    /// The size of the clear middle: higher pushes the darkening out towards
    /// the corners, lower closes it in. Defaults to 1.
    pub size: Option<f32>,
    /// How far in from the edges the darkening reaches: higher spreads it
    /// towards the middle. Defaults to 2.
    pub falloff: Option<f32>,
    /// 1 is a circle; above stretches it tall, below wide. Defaults to 1.
    pub roundness: Option<f32>,
    /// The color the edges fade to, as [r, g, b] from 0 to 1, the way a color
    /// picker shows it. Defaults to black.
    pub color: Option<[f32; 3]>,
}

impl Vignette {
    pub fn intensity(&self) -> f32 {
        self.intensity.unwrap_or(0.0).clamp(0.0, 1.0)
    }

    pub fn size(&self) -> f32 {
        self.size.unwrap_or(1.0).max(0.01)
    }

    pub fn falloff(&self) -> f32 {
        self.falloff.unwrap_or(2.0).max(0.01)
    }

    pub fn roundness(&self) -> f32 {
        self.roundness.unwrap_or(1.0).max(0.01)
    }

    pub fn color(&self) -> [f32; 3] {
        self.color.unwrap_or([0.0; 3])
    }
}

/// Film grain: fine noise over the finished image that changes every frame.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Grain {
    /// How strong the grain is: 0 is off, 0.04 subtle, 0.1 heavy. Defaults to
    /// 0.
    pub intensity: Option<f32>,
    /// The size of a grain in screen pixels, whatever the render scale.
    /// Defaults to 1.5.
    pub size: Option<f32>,
    /// Grain of separate colors, like color film, instead of gray. Defaults to
    /// false.
    pub colored: Option<bool>,
}

impl Grain {
    pub fn intensity(&self) -> f32 {
        self.intensity.unwrap_or(0.0).max(0.0)
    }

    pub fn size(&self) -> f32 {
        self.size.unwrap_or(1.5).max(1.0)
    }

    pub fn colored(&self) -> bool {
        self.colored.unwrap_or(false)
    }
}

// Mirrors Bevy's Tonemapping so this crate doesn't need Bevy.
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
    /// Turns on depth of field for the orbit camera with this f-stop, as in
    /// Blender: lower blurs more. The point it orbits stays sharp.
    pub f_stop: Option<f32>,
}

/// One exported camera's lens, which glTF cannot carry. The Blender exporter
/// fills it in from the camera's Depth of Field settings.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CameraEntry {
    /// Turns on depth of field with this f-stop, as in Blender: lower blurs
    /// more. Without it the whole image is sharp.
    pub f_stop: Option<f32>,
    /// What stays sharp, followed as it moves: the name of an object in the
    /// camera's file, or of a bone or object on the character. Used instead of
    /// `focus_distance` when found.
    pub focus_object: Option<String>,
    /// Distance in meters to what stays sharp, measured along the direction
    /// the camera looks. Defaults to 10.
    pub focus_distance: Option<f32>,
    /// Height of the camera's sensor in millimeters. With the field of view it
    /// sets the lens, and so how strong the blur is. Defaults to 18.66. To
    /// match Blender with Sensor Fit on Auto and a landscape image: its sensor
    /// size × image height ÷ image width.
    pub sensor_height_mm: Option<f32>,
}

impl CameraEntry {
    pub fn focus_distance(&self) -> f32 {
        self.focus_distance.unwrap_or(10.0)
    }

    pub fn sensor_height_mm(&self) -> f32 {
        self.sensor_height_mm.unwrap_or(18.66)
    }
}
