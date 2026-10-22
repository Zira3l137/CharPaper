use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// What every stage shares, saved between runs by the app, whose frame pacing reads the fps and
// pause settings. Bevy gives all lights' shadow maps one size, so their quality is shared too.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderSettings {
    pub fps_limit: FpsLimit,
    pub shadow_quality: ShadowQuality,
    pub pause_when_fullscreen: bool,
    pub pause_when_covered: bool,
    pub pause_on_battery: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            fps_limit: FpsLimit::default(),
            shadow_quality: ShadowQuality::default(),
            pause_when_fullscreen: true,
            pause_when_covered: true,
            pause_on_battery: false,
        }
    }
}

// How finely a stage is drawn. Each stage has its own, saved with it.
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Graphics {
    pub render_scale: u32,
    pub anti_aliasing: AntiAliasing,
    pub depth_of_field: DepthOfFieldQuality,
    pub fog: bool,
    pub fog_quality: FogQuality,
    pub fog_dithering: bool,
    pub fog_resolution: FogResolution,
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            render_scale: 100,
            anti_aliasing: AntiAliasing::default(),
            depth_of_field: DepthOfFieldQuality::default(),
            fog: true,
            fog_quality: FogQuality::default(),
            fog_dithering: false,
            fog_resolution: FogResolution::default(),
        }
    }
}

pub const RENDER_SCALES: [u32; 3] = [50, 75, 100];

impl Graphics {
    pub fn scale_percent(&self) -> u32 {
        RENDER_SCALES.into_iter().min_by_key(|s| s.abs_diff(self.render_scale)).unwrap_or(100)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FpsLimit {
    #[serde(rename = "15")]
    Fps15,
    #[default]
    #[serde(rename = "30")]
    Fps30,
    #[serde(rename = "60")]
    Fps60,
    #[serde(rename = "max")]
    Max,
}

impl FpsLimit {
    pub const ALL: [FpsLimit; 4] =
        [FpsLimit::Fps15, FpsLimit::Fps30, FpsLimit::Fps60, FpsLimit::Max];

    pub fn per_second(self) -> Option<f64> {
        match self {
            FpsLimit::Fps15 => Some(15.0),
            FpsLimit::Fps30 => Some(30.0),
            FpsLimit::Fps60 => Some(60.0),
            FpsLimit::Max => None,
        }
    }
}

// MSAA only at a sample count every GPU supports: an unsupported one crashes on the first
// frame. TAA blends each frame with the ones before, which also smooths fog dithering, but
// can leave faint trails behind motion, more so at low frame rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiAliasing {
    Off,
    #[default]
    #[serde(rename = "4x")]
    Msaa4,
    Taa,
}

impl AntiAliasing {
    pub const ALL: [AntiAliasing; 3] = [AntiAliasing::Off, AntiAliasing::Msaa4, AntiAliasing::Taa];
}

// Only for cameras whose suite asks for depth of field. Bokeh turns bright out-of-focus spots
// into lens-shaped discs, as Blender does; Blur is a cheaper plain blur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepthOfFieldQuality {
    Off,
    Blur,
    #[default]
    Bokeh,
}

impl DepthOfFieldQuality {
    pub const ALL: [DepthOfFieldQuality; 3] =
        [DepthOfFieldQuality::Off, DepthOfFieldQuality::Blur, DepthOfFieldQuality::Bokeh];
}

// What shadows cost the GPU. Every point light draws the scene six times a frame into its
// shadow map, a sun once per cascade; the size of those maps is what the setting changes,
// along with how many times each lit pixel looks its shadow up. Medium is Bevy's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowQuality {
    // One look-up a pixel, so edges are hard and can look jagged.
    Low,
    #[default]
    Medium,
    // Each point light's maps take 96 MB of video memory instead of 24.
    High,
}

impl ShadowQuality {
    pub const ALL: [ShadowQuality; 3] =
        [ShadowQuality::Low, ShadowQuality::Medium, ShadowQuality::High];
}

// How closely each pixel samples the fog along its ray: one sample per step length, up to a
// most a ray may take. Too few show as stripes wherever shadows cross the fog; each step up
// halves the step and doubles the most, so long rays cost twice as much. Short rays, such as
// fog just in front of a wall, stay cheap at any quality. Very low is meant for dithering
// with TAA, which blends the noise of a few frames into a smooth result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FogQuality {
    VeryLow,
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl FogQuality {
    pub const ALL: [FogQuality; 5] = [
        FogQuality::VeryLow,
        FogQuality::Low,
        FogQuality::Medium,
        FogQuality::High,
        FogQuality::Ultra,
    ];

    pub fn max_steps(self) -> u32 {
        match self {
            FogQuality::VeryLow => 16,
            FogQuality::Low => 32,
            FogQuality::Medium => 64,
            FogQuality::High => 128,
            FogQuality::Ultra => 256,
        }
    }

    // In meters. A ray longer than the most steps allow, about 10 m at any quality, takes
    // longer steps instead.
    pub fn step_length(self) -> f32 {
        match self {
            FogQuality::VeryLow => 0.6,
            FogQuality::Low => 0.3,
            FogQuality::Medium => 0.15,
            FogQuality::High => 0.075,
            FogQuality::Ultra => 0.0375,
        }
    }
}

// Fog is soft, so one fog pixel can stand for a few on screen. Half draws one for every four,
// about four times cheaper, and spreads it back out without blurring across the edges of
// what's in front of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FogResolution {
    Full,
    #[default]
    Half,
}

impl FogResolution {
    pub const ALL: [FogResolution; 2] = [FogResolution::Full, FogResolution::Half];
}
