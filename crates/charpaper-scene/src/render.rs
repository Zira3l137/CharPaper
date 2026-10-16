use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::asset::embedded_asset;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::DirectionalLightShadowMap;
use bevy::light::PointLightShadowMap;
use bevy::light::ShadowFilteringMethod;
use bevy::prelude::*;
use bevy::render::camera::MipBias;
use bevy::render::camera::TemporalJitter;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::ShaderType;
use bevy::render::render_resource::TextureFormat;
use bevy::shader::ShaderRef;
use bevy::window::WindowRef;
use serde::Deserialize;
use serde::Serialize;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::screens::Screen;
use crate::screens::Unseen;
use crate::stage::StageOf;

pub(crate) struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "scene_image.wgsl");
        let new_camera = any_match_filter::<Added<SceneCamera>>;
        app.add_plugins(UiMaterialPlugin::<SceneImage>::default())
            .add_observer(remove_screen_parts)
            .add_systems(
                Update,
                (
                    rest_unseen,
                    fit_scene_targets,
                    apply_anti_aliasing
                        .run_if(any_match_filter::<Changed<Graphics>>.or_eager(new_camera)),
                    apply_shadow_quality
                        .run_if(resource_changed::<RenderSettings>.or_eager(new_camera)),
                )
                    .in_set(SceneSet::Run),
            );
    }
}

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

    // Pixels a side of each of a point light's six maps.
    fn point_map(self) -> usize {
        match self {
            ShadowQuality::Low => 512,
            ShadowQuality::Medium => 1024,
            ShadowQuality::High => 2048,
        }
    }

    // Pixels a side of each of a sun's cascades.
    fn sun_map(self) -> usize {
        self.point_map() * 2
    }

    fn filtering(self) -> ShadowFilteringMethod {
        match self {
            ShadowQuality::Low => ShadowFilteringMethod::Hardware2x2,
            ShadowQuality::Medium | ShadowQuality::High => ShadowFilteringMethod::Gaussian,
        }
    }
}

// How many samples each pixel takes through the fog. Too few show as stripes wherever
// shadows cross it; each step up doubles the cost. Very low is meant for dithering with TAA,
// which blends the noise of a few frames into a smooth result.
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

    pub fn steps(self) -> u32 {
        match self {
            FogQuality::VeryLow => 16,
            FogQuality::Low => 32,
            FogQuality::Medium => 64,
            FogQuality::High => 128,
            FogQuality::Ultra => 256,
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

// What the scene added to a Screen to draw on it. Despawned along with the screen.
#[derive(Component)]
pub(crate) struct ScreenParts {
    pub camera: Entity,
    pub display: Entity,
    pub image_node: Entity,
    image: Handle<Image>,
    pub material: Handle<SceneImage>,
}

// Shows the scene image graded by the LUT, then with film grain on top. Both happen here,
// after the 3D camera, so grain stays one screen pixel fine at any render scale and TAA
// can't smear it. The LUT is read without a sampler: the shader blends its entries itself,
// which lets it be 32-bit float, a format not every GPU can filter.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct SceneImage {
    #[texture(0)]
    #[sampler(1)]
    image: Handle<Image>,
    #[uniform(2)]
    pub settings: SceneImageSettings,
    #[texture(3, dimension = "3d", sample_type = "float", filterable = false)]
    pub lut: Option<Handle<Image>>,
}

#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SceneImageSettings {
    // Intensity, grain size in pixels, 1 for colored grain, unused.
    pub grain: Vec4,
    // 0 while there is no LUT.
    pub lut_strength: f32,
}

impl UiMaterial for SceneImage {
    fn fragment_shader() -> ShaderRef {
        "embedded://charpaper_scene/scene_image.wgsl".into()
    }
}

// The 3D camera draws into an image, and a second camera shows that image across the
// screen's window under the UI. So the scene can render at a lower resolution while the UI
// stays sharp. Returns what the 3D camera needs, and the parts to keep on the screen, still
// without the 3D camera, which the caller spawns.
pub(crate) fn screen_parts(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    materials: &mut Assets<SceneImage>,
    screen: &Screen,
    window: &Window,
    graphics: &Graphics,
) -> (impl Bundle, ScreenParts) {
    let size = scaled(window, graphics);
    let image =
        images.add(Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None));
    let material =
        materials.add(SceneImage { image: image.clone(), settings: default(), lut: None });

    let display = commands
        .spawn((
            Name::new("Display camera"),
            Camera2d,
            Camera { order: 1, ..default() },
            RenderTarget::Window(WindowRef::Entity(screen.window)),
            Msaa::Off,
            Tonemapping::None,
        ))
        .id();
    let image_node = commands
        .spawn((
            Name::new("Scene image"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            MaterialNode(material.clone()),
            GlobalZIndex(i32::MIN),
            UiTargetCamera(display),
        ))
        .id();

    let parts = ScreenParts {
        camera: Entity::PLACEHOLDER,
        display,
        image_node,
        image: image.clone(),
        material,
    };
    ((RenderTarget::from(image), graphics.anti_aliasing.msaa()), parts)
}

fn remove_screen_parts(
    remove: On<Remove, ScreenParts>,
    parts: Query<&ScreenParts>,
    mut commands: Commands,
) {
    let Ok(parts) = parts.get(remove.entity) else {
        return;
    };
    for entity in [parts.camera, parts.display, parts.image_node] {
        commands.entity(entity).try_despawn();
    }
}

// Bevy recreates the GPU texture behind the same handle. The material that shows it keeps
// the old texture until it is marked changed, which makes Bevy build it again.
pub(crate) fn fit_scene_targets(
    screens: Query<(&Screen, &ScreenParts)>,
    windows: Query<&Window>,
    stage_of: StageOf,
    graphics: Query<&Graphics>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SceneImage>>,
) {
    for (screen, parts) in &screens {
        let (Ok(window), Some(graphics)) = (
            windows.get(screen.window),
            stage_of.get(parts.camera).and_then(|s| graphics.get(s).ok()),
        ) else {
            continue;
        };
        let size = scaled(window, graphics);
        if images.get(&parts.image).is_none_or(|image| image.size() == size) {
            continue;
        }
        if let Some(mut image) = images.get_mut(&parts.image) {
            image.resize(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 });
            debug!("scene renders at {}×{} on {}", size.x, size.y, screen.window);
        }
        if let Some(material) = materials.get_mut(&parts.material) {
            let _ = material.into_inner();
        }
    }
}

fn rest_unseen(
    screens: Query<(&ScreenParts, Has<Unseen>)>,
    mut cameras: Query<(&mut Camera, Option<&mut TemporalAntiAliasing>), With<SceneCamera>>,
) {
    for (parts, unseen) in &screens {
        let Ok((mut camera, taa)) = cameras.get_mut(parts.camera) else {
            continue;
        };
        if camera.is_active != unseen {
            continue;
        }
        camera.is_active = !unseen;
        // Its past frames show the scene as it was before it rested.
        if !unseen && let Some(mut taa) = taa {
            taa.reset = true;
        }
    }
}

// TAA can't run with MSAA. Inserting it brings the prepasses and camera jitter it needs, but
// removing it leaves them behind, so they go explicitly. Only new cameras, and those of a stage
// whose graphics changed, are touched.
pub(crate) fn apply_anti_aliasing(
    mut commands: Commands,
    cameras: Query<(Entity, Ref<SceneCamera>)>,
    stage_of: StageOf,
    graphics: Query<Ref<Graphics>>,
) {
    for (camera, marker) in &cameras {
        let new = marker.is_added();
        let Some(graphics) = stage_of.get(camera).and_then(|s| graphics.get(s).ok()) else {
            continue;
        };
        if !new && !graphics.is_changed() {
            continue;
        }
        let mut camera = commands.entity(camera);
        camera.insert(graphics.anti_aliasing.msaa());
        if graphics.anti_aliasing == AntiAliasing::Taa {
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

// The maps' sizes are shared by every light; the filtering belongs to each camera.
fn apply_shadow_quality(
    mut commands: Commands,
    settings: Res<RenderSettings>,
    cameras: Query<Entity, With<SceneCamera>>,
    mut point: ResMut<PointLightShadowMap>,
    mut sun: ResMut<DirectionalLightShadowMap>,
) {
    let quality = settings.shadow_quality;
    if point.size != quality.point_map() {
        point.size = quality.point_map();
    }
    if sun.size != quality.sun_map() {
        sun.size = quality.sun_map();
    }
    for camera in &cameras {
        commands.entity(camera).insert(quality.filtering());
    }
}

fn scaled(window: &Window, graphics: &Graphics) -> UVec2 {
    let scale = graphics.scale_percent() as f32 / 100.0;
    let physical = UVec2::new(window.physical_width(), window.physical_height()).as_vec2();
    (physical * scale).round().as_uvec2().max(UVec2::ONE)
}
