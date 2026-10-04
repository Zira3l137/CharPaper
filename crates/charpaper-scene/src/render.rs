use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::camera::MipBias;
use bevy::render::camera::TemporalJitter;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureFormat;
use bevy::window::PrimaryWindow;
use serde::Deserialize;
use serde::Serialize;

use crate::SceneSet;
use crate::camera::SceneCamera;

pub(crate) struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (fit_scene_target, apply_anti_aliasing.run_if(resource_changed::<RenderSettings>))
                .in_set(SceneSet::Run),
        );
    }
}

// Saved between runs by the app, whose frame pacing reads the fps and pause settings.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderSettings {
    pub fps_limit: FpsLimit,
    pub render_scale: u32,
    pub anti_aliasing: AntiAliasing,
    pub depth_of_field: DepthOfFieldQuality,
    pub fog: bool,
    pub fog_quality: FogQuality,
    pub fog_dithering: bool,
    pub pause_when_fullscreen: bool,
    pub pause_when_covered: bool,
    pub pause_on_battery: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            fps_limit: FpsLimit::default(),
            render_scale: 100,
            anti_aliasing: AntiAliasing::default(),
            depth_of_field: DepthOfFieldQuality::default(),
            fog: true,
            fog_quality: FogQuality::default(),
            fog_dithering: false,
            pause_when_fullscreen: true,
            pause_when_covered: true,
            pause_on_battery: false,
        }
    }
}

pub const RENDER_SCALES: [u32; 3] = [50, 75, 100];

impl RenderSettings {
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

    fn msaa(self) -> Msaa {
        match self {
            AntiAliasing::Off | AntiAliasing::Taa => Msaa::Off,
            AntiAliasing::Msaa4 => Msaa::Sample4,
        }
    }
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

// How many samples each pixel takes through the fog. Too few show as stripes wherever
// shadows cross it; each step up doubles the cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FogQuality {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl FogQuality {
    pub const ALL: [FogQuality; 4] =
        [FogQuality::Low, FogQuality::Medium, FogQuality::High, FogQuality::Ultra];

    pub fn steps(self) -> u32 {
        match self {
            FogQuality::Low => 32,
            FogQuality::Medium => 64,
            FogQuality::High => 128,
            FogQuality::Ultra => 256,
        }
    }
}

#[derive(Resource)]
pub(crate) struct SceneTarget(Handle<Image>);

// The 3D camera draws into an image, and a second camera shows that image full-screen
// under the UI. So the scene can render at a lower resolution while the UI stays sharp.
pub(crate) fn scene_target(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    window: Option<&Window>,
    settings: &RenderSettings,
) -> impl Bundle {
    let size = window.map_or(UVec2::ONE, |w| scaled(w, settings));
    let image =
        images.add(Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None));
    commands.insert_resource(SceneTarget(image.clone()));

    commands.spawn((
        Name::new("Display camera"),
        Camera2d,
        Camera { order: 1, ..default() },
        IsDefaultUiCamera,
        Msaa::Off,
        Tonemapping::None,
    ));
    commands.spawn((
        Name::new("Scene image"),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ImageNode::new(image.clone()),
        GlobalZIndex(i32::MIN),
    ));

    (RenderTarget::from(image), settings.anti_aliasing.msaa())
}

// Resizing the image is enough: Bevy recreates the GPU texture behind the same handle.
pub(crate) fn fit_scene_target(
    settings: Res<RenderSettings>,
    target: Option<Res<SceneTarget>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(target), Ok(window)) = (target, windows.single()) else {
        return;
    };
    let size = scaled(window, &settings);
    if images.get(&target.0).is_none_or(|image| image.size() == size) {
        return;
    }
    if let Some(mut image) = images.get_mut(&target.0) {
        image.resize(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 });
        debug!("scene renders at {}×{}", size.x, size.y);
    }
}

// TAA can't run with MSAA. Inserting it brings the prepasses and camera jitter it needs, but
// removing it leaves them behind, so they go explicitly.
pub(crate) fn apply_anti_aliasing(
    mut commands: Commands,
    settings: Res<RenderSettings>,
    camera: Query<Entity, With<SceneCamera>>,
) {
    for camera in &camera {
        let mut camera = commands.entity(camera);
        camera.insert(settings.anti_aliasing.msaa());
        if settings.anti_aliasing == AntiAliasing::Taa {
            camera.insert(TemporalAntiAliasing::default());
        } else {
            camera.remove::<(
                TemporalAntiAliasing,
                TemporalJitter,
                MipBias,
                DepthPrepass,
                MotionVectorPrepass,
            )>();
        }
    }
}

fn scaled(window: &Window, settings: &RenderSettings) -> UVec2 {
    let scale = settings.scale_percent() as f32 / 100.0;
    let physical = UVec2::new(window.physical_width(), window.physical_height()).as_vec2();
    (physical * scale).round().as_uvec2().max(UVec2::ONE)
}
