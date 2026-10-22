use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::asset::embedded_asset;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::core_pipeline::prepass::NormalPrepass;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::DirectionalLightShadowMap;
use bevy::light::PointLightShadowMap;
use bevy::light::ShadowFilteringMethod;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::prelude::*;
use bevy::render::camera::MipBias;
use bevy::render::camera::TemporalJitter;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::ShaderType;
use bevy::render::render_resource::TextureFormat;
use bevy::shader::ShaderRef;
use bevy::window::WindowRef;
use charpaper_settings::AntiAliasing;
use charpaper_settings::Graphics;
use charpaper_settings::RenderSettings;
use charpaper_settings::ShadowQuality;

use crate::SceneSet;
use crate::ambient_occlusion::ambient_occlusion;
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
    ((RenderTarget::from(image), msaa(graphics.anti_aliasing)), parts)
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

// TAA and SSAO can't run with MSAA. Inserting them brings the prepasses and camera jitter they
// need, but removing them leaves those behind, so they go explicitly, once neither needs them.
// SSAO only darkens light from the sky and its reflections, and is smoothest with TAA, which
// blends away its noise. Only new cameras, and those of a stage whose graphics changed, are
// touched.
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
        let taa = graphics.anti_aliasing == AntiAliasing::Taa;
        let ssao = graphics.ambient_occlusion && graphics.anti_aliasing != AntiAliasing::Msaa4;
        let mut camera = commands.entity(camera);
        camera.insert(msaa(graphics.anti_aliasing));
        if taa {
            camera.insert(TemporalAntiAliasing::default());
        } else {
            camera.remove::<(TemporalAntiAliasing, TemporalJitter, MipBias, MotionVectorPrepass)>();
        }
        if ssao {
            camera.insert(ambient_occlusion(&graphics));
        } else {
            camera.remove::<(ScreenSpaceAmbientOcclusion, NormalPrepass)>();
        }
        if !taa && !ssao {
            camera.remove::<DepthPrepass>();
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
    if point.size != point_map(quality) {
        point.size = point_map(quality);
    }
    if sun.size != sun_map(quality) {
        sun.size = sun_map(quality);
    }
    for camera in &cameras {
        commands.entity(camera).insert(shadow_filtering(quality));
    }
}

fn scaled(window: &Window, graphics: &Graphics) -> UVec2 {
    let scale = graphics.scale_percent() as f32 / 100.0;
    let physical = UVec2::new(window.physical_width(), window.physical_height()).as_vec2();
    (physical * scale).round().as_uvec2().max(UVec2::ONE)
}

fn msaa(anti_aliasing: AntiAliasing) -> Msaa {
    match anti_aliasing {
        AntiAliasing::Off | AntiAliasing::Taa => Msaa::Off,
        AntiAliasing::Msaa4 => Msaa::Sample4,
    }
}

// Pixels a side of each of a point light's six maps.
fn point_map(quality: ShadowQuality) -> usize {
    match quality {
        ShadowQuality::Low => 512,
        ShadowQuality::Medium => 1024,
        ShadowQuality::High => 2048,
    }
}

// Pixels a side of each of a sun's cascades.
fn sun_map(quality: ShadowQuality) -> usize {
    point_map(quality) * 2
}

fn shadow_filtering(quality: ShadowQuality) -> ShadowFilteringMethod {
    match quality {
        ShadowQuality::Low => ShadowFilteringMethod::Hardware2x2,
        ShadowQuality::Medium | ShadowQuality::High => ShadowFilteringMethod::Gaussian,
    }
}
