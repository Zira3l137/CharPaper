use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

// Doc comments in this file become the hover text of the generated JSON Schema, so they
// are written for suite authors. `deny_unknown_fields` everywhere: a misspelt key that
// silently does nothing is worse than an error.

pub const MANIFEST_FILE: &str = "suite.toml";

// Bumped only for changes old files can't be read under, not for new optional keys. Only the
// current version is read: the Blender add-on writes every suite, so an old one is exported
// again.
pub const SCHEMA_VERSION: u32 = 2;

/// A CharPaper character suite: the `suite.toml` at the root of the suite's
/// folder.
#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
#[schemars(title = "CharPaper suite manifest")]
pub struct Manifest {
    /// Version of this file's format. Use 2.
    #[schemars(range(min = SCHEMA_VERSION, max = SCHEMA_VERSION))]
    pub schema: u32,
    /// Name shown in the app. Defaults to the folder name.
    pub name: Option<String>,
    #[serde(default)]
    pub timeline: Timeline,
    #[serde(default)]
    pub character: CharacterSection,
    #[serde(default)]
    pub environment: EnvironmentSection,
    /// Settings for each environment, keyed by its name: the `.glb` file name
    /// in `environment/` without its extension, or the name of its folder.
    /// That folder may also hold the environment's sound: `music/`, whose
    /// `.ogg` files play in file-name order and then start over, and
    /// `ambience.ogg`, which loops under them. Its `animated/` folder holds
    /// GIFs that play on the scene's materials, each on the material it is
    /// named after: `animated/TV screen.gif` replaces the base color of the
    /// material named "TV screen".
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
    /// section it never does.
    pub gaze: Option<Gaze>,
    /// Places on the character or on an environment's armatures that react
    /// to the mouse, keyed by name. Each plays clips when clicked or hovered.
    #[serde(default)]
    pub hotspots: BTreeMap<String, Hotspot>,
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
    /// Clip played at start: a file name in `clips/` without its extension.
    /// Without one the character holds its rest pose.
    pub default_clip: Option<String>,
    /// Settings for the character's clips, keyed by clip name: a file name in
    /// `clips/` without its extension. Every clip found there plays on the
    /// timeline unless set otherwise here.
    #[serde(default)]
    pub clips: BTreeMap<String, ClipEntry>,
    /// Settings for the character's expressions, keyed by name: a file name in
    /// `expressions/` without its extension. An expression sets shape keys by
    /// name on whichever skin is worn and loops while chosen.
    #[serde(default)]
    pub expressions: BTreeMap<String, ExpressionEntry>,
}

/// The loop everything plays on, as Blender's timeline: the scene's frame
/// range at its frame rate. Clips in `timeline` mode show the timeline's
/// frame, so they stay in step with each other.
#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Timeline {
    /// First frame, as Blender's Frame Start. Defaults to 1.
    #[serde(default = "Timeline::default_start")]
    pub start: i32,
    /// Last frame, as Blender's Frame End. It is shown for one frame before
    /// the loop starts over. Defaults to 250.
    #[serde(default = "Timeline::default_end")]
    pub end: i32,
    /// Frames per second, as Blender's Frame Rate. Defaults to 24.
    #[serde(default = "Timeline::default_fps")]
    #[schemars(range(min = 0.0))]
    pub fps: f32,
}

impl Timeline {
    fn default_start() -> i32 {
        1
    }

    fn default_end() -> i32 {
        250
    }

    fn default_fps() -> f32 {
        24.0
    }

    /// How long one loop lasts, in seconds: from the first frame to the end of the last.
    pub fn seconds(&self) -> f64 {
        f64::from(self.end - self.start + 1) / f64::from(self.fps)
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self { start: 1, end: 250, fps: 24.0 }
    }
}

/// How one clip plays.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClipEntry {
    #[serde(default)]
    pub mode: ClipMode,
    /// Name shown in the app. Defaults to the clip's name.
    pub label: Option<String>,
    /// Whether the character may look towards the cursor while this plays.
    /// Defaults to true. Turn it off where the character has to look somewhere on
    /// purpose, like a dance or a sleeping pose. Only the character's clips
    /// have it.
    pub gaze: Option<bool>,
    /// Sounds to play as the clip passes given moments, from the pose markers
    /// of its action. A clip on the timeline plays them again on every loop.
    /// They stop when the clip does.
    #[serde(default)]
    pub cues: Vec<Cue>,
    /// Keeps the clip out of the app's list of animations, for clips only
    /// hotspots play, like a flinch.
    #[serde(default)]
    pub reaction_only: bool,
}

/// One expression's settings.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpressionEntry {
    /// Name shown in the app. Defaults to the expression's name.
    pub label: Option<String>,
}

/// One animated object or armature of an environment.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActorEntry {
    /// Settings for its clips, keyed by clip name: a file name in its
    /// `clips/<actor>/` folder without its extension.
    #[serde(default)]
    pub clips: BTreeMap<String, ClipEntry>,
}

/// A sound played at a moment of an animation.
#[derive(Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Cue {
    /// Seconds from the start of the clip.
    #[schemars(range(min = 0.0))]
    pub at: f32,
    /// A file name in `sounds/` without its `.ogg` extension.
    pub sound: String,
}

/// Looking towards the mouse cursor. Where the character looks comes from
/// where the cursor is on screen: with the cursor on its eyes it looks at
/// the viewer, and the further from them the cursor is, the further aside
/// it looks. The head and neck go part of the way and the eyes cover the
/// rest. The head's angles are measured from the bone above the neck, with
/// the animation's own turn counted in, and the eyes' from the head. A
/// cursor the character can't turn to, it lets go of.
#[derive(Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Gaze {
    /// The eye bones, usually two. Gaze sets them outright while it is on,
    /// so an animation's own keys on them only show in a clip with
    /// `gaze = false`. May be left out: eyes can follow by `shape_keys`
    /// alone, and a head can follow without eyes.
    #[serde(default)]
    pub eyes: Vec<String>,
    #[serde(default)]
    pub eye_kind: EyeKind,
    /// The head bone. Without it, only the neck and eyes turn.
    pub head: Option<String>,
    /// The neck bone. Without it, the head does all of the turning.
    pub neck: Option<String>,
    /// How far the eye bones turn sideways, in degrees. Defaults to 30.
    pub eye_yaw_deg: Option<f32>,
    /// How far the eye bones turn up or down, in degrees. Defaults to 20.
    pub eye_pitch_deg: Option<f32>,
    /// How far the head itself turns sideways, in degrees. Together with the
    /// neck's, it is how far the head may point from the bone above the
    /// neck, whether an animation or gaze turned it there. Defaults to 50.
    pub head_yaw_deg: Option<f32>,
    /// How far the head itself turns up or down, in degrees. Defaults to 25.
    pub head_pitch_deg: Option<f32>,
    /// How far the neck turns sideways, in degrees. Defaults to 25.
    pub neck_yaw_deg: Option<f32>,
    /// How far the neck turns up or down, in degrees. Defaults to 15.
    pub neck_pitch_deg: Option<f32>,
    /// With both a head and a neck: the head's share of their turn, from 0
    /// to 1. The neck takes the rest, up to its limit. Defaults to 0.7.
    pub head_share: Option<f32>,
    /// How much of the way to where the character looks the head and neck
    /// go, from 0 to 1, leaving the rest to the eyes. At 0 the head only moves for what
    /// the eyes can't reach; at 1 it points straight there and the eyes stay
    /// centered. Defaults to 0.6. Without eyes the head goes all the way.
    pub head_turn: Option<f32>,
    /// Which way the character's face points in the rest pose, in Blender's
    /// world axes as exported: `[0, -1, 0]` faces Blender's front view.
    /// Rarely needed: it is worked out from where the eye bones sit from
    /// the head bone. Without those and without this, the character is
    /// taken to face Blender's front view. Only its horizontal part counts.
    pub front: Option<[f32; 3]>,
    /// Shape keys that follow the eyes: eyelids, or on a character without
    /// eye bones, the eyes themselves. Each is set in proportion to how far
    /// the character looks that way, reaching 1 at a full look. Any may be
    /// left out, and a skin without one ignores it.
    #[serde(default)]
    pub shape_keys: GazeShapeKeys,
}

/// What the eye bones turn. Defaults to `ball`.
#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EyeKind {
    /// A round eyeball in its socket. It is aimed exactly, up to its limits.
    #[default]
    Ball,
    /// Anything else, like an iris that slides over a flat eye. Aimed, such
    /// a bone would shoot past, so it turns in proportion to how far aside
    /// the character looks instead, reaching its limits for a look about
    /// 35° aside.
    Drawn,
}

impl Gaze {
    /// `front` in glTF's axes, which are Bevy's: Blender's +Z up is +Y, its -Y
    /// front is +Z.
    pub fn front_gltf(&self) -> [f32; 3] {
        let [x, y, z] = self.front.unwrap_or([0.0, -1.0, 0.0]);
        [x, z, -y]
    }

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

    pub fn head_turn(&self) -> f32 {
        self.head_turn.unwrap_or(0.6).clamp(0.0, 1.0)
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

/// A place that reacts to the mouse: capsules around bones. When it is set
/// off, every actor it lists clips for plays one of them, picked at random
/// but never the one it played last time; the character's and a door's clips
/// played together stay in step. A clip plays through once whatever its mode,
/// then what played before shows again. While its clips play, setting the
/// hotspot off again does nothing.
#[derive(Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hotspot {
    #[serde(default)]
    pub trigger: Trigger,
    /// Where the hotspot is: any number of capsules, each around one bone.
    pub areas: Vec<HotspotArea>,
    /// What plays when it is set off.
    pub clips: Vec<HotspotClip>,
    /// Plays its clips over what other hotspots set off, instead of stopping
    /// them. Each layered clip moves only the bones and shape keys it keys.
    /// Defaults to false: it stops the reactions playing on the actors it
    /// plays on, and takes their place.
    #[serde(default)]
    pub layer: bool,
    /// The character's expression while the hotspot's clips play: a name
    /// from `character.expressions`. Then the chosen one comes back.
    pub expression: Option<String>,
    /// Seconds after it is set off before it can be again. Defaults to 0.
    #[schemars(range(min = 0.0))]
    pub cooldown: Option<f32>,
    /// The character's animations during which it can be set off: names
    /// from `character.clips`, of those the viewer picks to play on and on
    /// (`timeline` or `hold`). A clip played once over one of them counts as
    /// that one. Leave it out for any animation. When the animation changes
    /// to one not listed, the hotspot's clips stop.
    #[serde(default)]
    pub during: Vec<String>,
}

/// What sets a hotspot off.
#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// A left click on it: pressed and let go on it, without dragging.
    #[default]
    Click,
    /// The cursor resting on it for a moment. It goes off again only after the
    /// cursor has left it.
    Hover,
}

/// A capsule around a bone: the line from its head to its tail, grown by
/// `radius` on every side. It follows the bone as it moves.
#[derive(Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HotspotArea {
    /// The armature the bone is in: the name of an armature object in an
    /// environment, which needs no clips of its own. Leave it out for the
    /// character's. While an environment without it shows, the area isn't
    /// there.
    pub actor: Option<String>,
    pub bone: String,
    /// The bone's length, from head to tail, in the armature's units.
    #[schemars(range(min = 0.0))]
    pub length: f32,
    /// How far the capsule reaches from the bone, in the armature's units.
    #[schemars(range(min = 0.0))]
    pub radius: f32,
}

/// One clip a hotspot plays.
#[derive(Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HotspotClip {
    /// Whose clip: the name of one of an environment's actors. Leave it out
    /// for the character's.
    pub actor: Option<String>,
    /// The clip's name, as in `character.clips` or the actor's `clips`.
    pub clip: String,
}

/// How a clip plays. Defaults to `timeline`.
#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipMode {
    /// Shows the timeline's frame and loops with it, in step with every
    /// other clip on the timeline.
    #[default]
    Timeline,
    /// Plays through from its first frame when started, then hands back to
    /// what played before.
    Once,
    /// Holds its first frame. For still poses.
    Hold,
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
    /// How bright the sky is and how strongly it lights the scene, as the
    /// Strength of the Background node in Blender's World: 1 shows the
    /// panorama as it is. Defaults to 1.
    pub sky_strength: Option<f32>,
    /// Older suites' sky brightness in cd/m², 1000 for a strength of 1. Read
    /// only without `sky_strength`.
    pub brightness: Option<f32>,
    /// Whether the environment's lights cast shadows. glTF cannot say, so it
    /// is set here. Defaults to true.
    pub shadows: Option<bool>,
    /// Whether the environment's sun shines from its sky's sun, the sky's
    /// brightest spot, turning as the sky is turned, so that its light and
    /// shadows match the sun in the background. A sky only has a sun when
    /// that spot stands far out from the rest: not an overcast one, nor one
    /// whose brightest spot is below the horizon. With several suns, the
    /// brightest follows. Defaults to true.
    pub sun_follows_sky: Option<bool>,
    /// Exposure compensation in EV stops while this environment is shown,
    /// replacing `post.exposure`. A sunlit scene needs far less than a room.
    pub exposure: Option<f32>,
    /// Turns the sky, and the light it casts, around the vertical axis, in
    /// degrees, as the Z Rotation of a Point-type Mapping node turns Blender's
    /// Environment Texture. Defaults to 0.
    pub sky_rotation_deg: Option<f32>,
    /// How bright the sky looks behind the scene, as a share of how brightly
    /// it lights it: 0 hides it, 1 shows it as it is. Like Blender's Light
    /// Path trick, it leaves the lighting alone. Defaults to 1.
    pub background: Option<f32>,
    /// How blurred the sky looks behind the scene, from 0, sharp, to 1, as
    /// if a lens were focused close: about what a Principled BSDF of
    /// Roughness 0.5 reflects. The lighting stays as it is. Defaults to 0.
    pub background_blur: Option<BackgroundBlur>,
    /// Boxes of fog, keyed by the name of their object in the environment's
    /// file: in Blender, usually a cube with a Principled Volume material. The
    /// fog fills the object's bounding box, or the 2 m cube Blender draws for
    /// an empty, and moves, turns and scales with it. The sky lights fog
    /// evenly; lights also cast light shafts through it.
    #[serde(default)]
    pub fog: BTreeMap<String, Fog>,
    /// Lights that do not light the fog, by object name, like a Blender light
    /// whose Volume Scatter is 0. Every other light does. A sun only lights
    /// fog while it casts shadows.
    #[serde(default)]
    pub no_volume_scatter: Vec<String>,
    /// Lights and objects that cast no shadows, by object name, like a Blender
    /// light whose Shadow is off or an object whose Visibility > Ray
    /// Visibility > Shadow is. Every other light does while `shadows` is on.
    /// A light without shadows costs far less: a point light draws the scene
    /// six times a frame for its shadows. A sun without shadows doesn't light
    /// fog. An object left out of shadows is drawn into none of them, and an
    /// animated one no longer makes lamps draw their shadows again.
    #[serde(default)]
    pub no_shadows: Vec<String>,
    /// What glTF can't carry of the environment's lights, keyed by object
    /// name.
    #[serde(default)]
    pub lights: BTreeMap<String, Light>,
    /// Settings for the environment's animated objects and armatures, keyed by
    /// object name. Each one's clips are the files in the environment's
    /// `clips/<name>/` folder.
    #[serde(default)]
    pub actors: BTreeMap<String, ActorEntry>,
}

/// A light's settings beyond glTF's.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Light {
    /// The size of a point or spot light's bulb, in meters, as Blender's
    /// Radius: it widens and softens the light's reflections. Its shadows stay
    /// sharp. Defaults to 0, a point.
    pub radius: Option<f32>,
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
    /// Varies how thick the fog is from place to place, as a Noise Texture
    /// feeding Density does. density then scales what it gives.
    pub noise: Option<FogNoise>,
}

/// Blender's Noise Texture in fBM mode, and what the nodes between it and
/// Density make of it.
#[derive(Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FogNoise {
    /// Three rows of the affine matrix from the fog box's own space, the cube
    /// from -0.5 to 0.5 with Y up, to the noise's coordinates, its Scale
    /// included.
    pub transform: [[f32; 4]; 3],
    /// How far the noise's coordinates move each second, as an animated
    /// Mapping node's Location moves them. Defaults to none.
    pub drift: Option<[f32; 3]>,
    /// Octaves past the first; the app stops at 3. Defaults to 2.
    pub detail: Option<f32>,
    /// How much each octave counts next to the one before. Defaults to 0.5.
    pub roughness: Option<f32>,
    /// How much finer each octave is than the one before. Defaults to 2.
    pub lacunarity: Option<f32>,
    /// Keeps the noise between 0 and 1. Defaults to true.
    pub normalize: Option<bool>,
    /// The fog's density at evenly spaced noise values from 0 to 1, in place
    /// of density; at least two.
    pub density: Vec<f32>,
}

/// Effects applied to the finished image.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Post {
    pub tonemapping: Option<Tonemapping>,
    /// Exposure compensation in EV stops; positive is brighter.
    pub exposure: Option<f32>,
    /// How much glow is added around light brighter than the threshold, as
    /// Blender's Glare node does. 0.0 turns bloom off; 0.1 is subtle.
    pub bloom: Option<f32>,
    /// How bright light must be to glow, before tonemapping: 1.0 is white,
    /// like the Glare node's threshold. Defaults to 1.0.
    pub bloom_threshold: Option<f32>,
    /// Color fringes that grow towards the edges of the image, as a fraction
    /// of its size. 0.0 turns them off; 0.005 is subtle.
    pub chromatic_aberration: Option<f32>,
    #[serde(default)]
    pub vignette: Vignette,
    #[serde(default)]
    pub grain: Grain,
    #[serde(default)]
    pub grading: Grading,
    #[serde(default)]
    pub lut: Lut,
}

/// A color lookup table from `luts/`: the .cube files grading apps like
/// Resolve and Photoshop export. It grades the finished image, after
/// everything else but the grain.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lut {
    /// The file in `luts/` without its extension, or `none`. Defaults to
    /// none.
    pub name: Option<String>,
    /// How much of the LUT shows, from 0 to 1. Defaults to 1.
    pub strength: Option<f32>,
}

/// The `[post.lut]` name that picks no LUT.
pub const NO_LUT: &str = "none";

impl Lut {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref().filter(|&name| name != NO_LUT)
    }

    pub fn strength(&self) -> f32 {
        self.strength.unwrap_or(1.0).clamp(0.0, 1.0)
    }
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

/// Color grading: white balance, saturation and contrast for the whole
/// image, and finer controls for its shadows, midtones and highlights. Bevy
/// grades while it tonemaps, so none of this works with `tonemapping =
/// "none"`.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Grading {
    /// Warms the image towards orange (positive) or cools it towards blue
    /// (negative), from -1 to 1. 0.25 is a gentle shift. Defaults to 0.
    pub warmth: Option<f32>,
    /// Shifts the image towards magenta (positive) or green (negative), from
    /// -1 to 1. Defaults to 0.
    pub tint: Option<f32>,
    /// Turns every color around the color wheel, in degrees. Defaults to 0.
    pub hue_deg: Option<f32>,
    /// 0 is black and white, 1 unchanged, above more vivid. Applied after
    /// tonemapping. Defaults to 1.
    pub saturation: Option<f32>,
    /// Spreads tones away from middle gray (above 1) or pulls them towards
    /// it (below 1), as a power of their distance in stops: 1.1 puts
    /// something 3 stops darker than middle gray 3.3 stops darker. Black and
    /// middle gray stay where they are. Each section's own contrast applies
    /// on top. Defaults to 1.
    pub contrast: Option<f32>,
    /// Where the midtones begin and end, as brightness from 0 to 1; the
    /// sections blend into each other smoothly. Defaults to [0.2, 0.7].
    pub midtones_range: Option<[f32; 2]>,
    #[serde(default)]
    pub shadows: GradingSection,
    #[serde(default)]
    pub midtones: GradingSection,
    #[serde(default)]
    pub highlights: GradingSection,
}

/// Grading for one brightness range, as in the ASC CDL formula
/// `out = (in × gain + lift) ^ gamma`.
#[derive(Deserialize, JsonSchema, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GradingSection {
    /// 0 is black and white, 1 unchanged, above more vivid. Defaults to 1.
    pub saturation: Option<f32>,
    /// Spreads colors away from middle gray (above 1) or pulls them towards
    /// it (below 1). Defaults to 1.
    pub contrast: Option<f32>,
    /// Bends brightness, mostly in the upper part of the range: above 1
    /// darker, below brighter. Defaults to 1.
    pub gamma: Option<f32>,
    /// Multiplies brightness, mostly in the middle of the range. Defaults to
    /// 1.
    pub gain: Option<f32>,
    /// Adds to brightness, mostly in the lower part of the range. Defaults to
    /// 0.
    pub lift: Option<f32>,
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

// What older suites' `brightness` was per unit of strength.
const LEGACY_BRIGHTNESS_PER_STRENGTH: f32 = 1000.0;

impl EnvironmentEntry {
    /// `sky_strength`, or what an older suite's `brightness` stands for.
    pub fn strength(&self) -> Option<f32> {
        self.sky_strength.or(self.brightness.map(legacy_strength))
    }
}

pub(crate) fn legacy_strength(brightness: f32) -> f32 {
    brightness / LEGACY_BRIGHTNESS_PER_STRENGTH
}

/// How blurred the sky looks behind the scene, from 0 to 1.
#[derive(Serialize, JsonSchema, Debug, Clone, Copy, PartialEq, Default)]
#[serde(transparent)]
pub struct BackgroundBlur(pub f32);

// Older suites name one of four steps. Each reads as the amount that blurs as much.
impl<'de> Deserialize<'de> for BackgroundBlur {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Saved {
            Amount(f32),
            Step(Step),
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Step {
            Off,
            Low,
            Medium,
            High,
        }

        Ok(BackgroundBlur(match Saved::deserialize(deserializer)? {
            Saved::Amount(amount) => amount.clamp(0.0, 1.0),
            Saved::Step(Step::Off) => 0.0,
            Saved::Step(Step::Low) => 0.71,
            Saved::Step(Step::Medium) => 0.87,
            Saved::Step(Step::High) => 1.0,
        }))
    }
}

// Mirrors Bevy's Tonemapping so this crate doesn't need Bevy.
/// How bright colors are squeezed into what the screen can show.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
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
