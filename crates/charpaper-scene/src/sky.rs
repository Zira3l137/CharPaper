// The sky behind the scene, drawn in place of Bevy's skybox so it can blur by any amount.
// Bevy's draws the sharpest level of its cubemap only. This one also reads the reflection map,
// whose levels the baker blurred for rougher and rougher surfaces, and samples between them, so
// the blur grows smoothly from none.
//
// It hands Bevy's opaque pass the pipeline and bind group it draws a skybox with, under
// Bevy's own components, so it is drawn where and as Bevy's would be.

use bevy::camera::Exposure;
use bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT;
use bevy::core_pipeline::skybox::SkyboxBindGroup;
use bevy::core_pipeline::skybox::SkyboxPipelineId;
use bevy::prelude::*;
use bevy::render::Extract;
use bevy::render::ExtractSchedule;
use bevy::render::GpuResourceAppExt;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderStartup;
use bevy::render::RenderSystems;
use bevy::render::extract_component::ComponentUniforms;
use bevy::render::extract_component::DynamicUniformIndex;
use bevy::render::extract_component::UniformComponentPlugin;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::sampler;
use bevy::render::render_resource::binding_types::texture_cube;
use bevy::render::render_resource::binding_types::uniform_buffer;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::sync_component::SyncComponent;
use bevy::render::sync_component::SyncComponentPlugin;
use bevy::render::sync_world::RenderEntity;
use bevy::render::texture::GpuImage;
use bevy::render::view::ExtractedView;
use bevy::render::view::Msaa;
use bevy::render::view::ViewUniform;
use bevy::render::view::ViewUniforms;
use bevy::shader::Shader;

// The reflection map's perceptual roughness at full blur.
const MAX_ROUGHNESS: f32 = 0.5;

pub(crate) struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "sky.wgsl");
        app.add_plugins((
            SyncComponentPlugin::<SceneSky, Self>::default(),
            UniformComponentPlugin::<SkyUniform>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<SkyPipeline>>()
            .add_systems(ExtractSchedule, extract_sky)
            .add_systems(RenderStartup, init_sky_pipeline)
            .add_systems(
                Render,
                (
                    prepare_sky_pipelines.in_set(RenderSystems::Prepare),
                    prepare_sky_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

// On a scene camera while its environment has a sky.
#[derive(Component, Clone)]
pub(crate) struct SceneSky {
    pub image: Handle<Image>,
    // The environment's reflection map, to blur with. Without one the sky stays sharp.
    pub reflections: Option<Handle<Image>>,
    // cd/m², before the camera's exposure.
    pub brightness: f32,
    pub rotation: Quat,
    // From 0, sharp, to 1.
    pub blur: f32,
}

impl SyncComponent<SkyPlugin> for SceneSky {
    type Target = (ExtractedSky, SkyUniform, SkyboxPipelineId, SkyboxBindGroup);
}

#[derive(Component)]
pub(crate) struct ExtractedSky {
    image: Handle<Image>,
    reflections: Option<Handle<Image>>,
}

#[derive(Component, ShaderType, Clone)]
pub(crate) struct SkyUniform {
    brightness: f32,
    // The perceptual roughness the reflection map is read at, 0 for the sharp sky alone.
    roughness: f32,
    transform: Mat4,
}

fn extract_sky(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    cameras: Extract<Query<(RenderEntity, &SceneSky, Option<&Exposure>)>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, sky, exposure) in &cameras {
        let exposure = exposure.copied().unwrap_or_default().exposure();
        // Squared, so the first steps only soften the sky and the blur builds from there.
        let blur = if sky.reflections.is_some() { sky.blur.clamp(0.0, 1.0) } else { 0.0 };
        let uniform = SkyUniform {
            brightness: sky.brightness * exposure,
            roughness: MAX_ROUGHNESS * blur * blur,
            transform: Transform::from_rotation(sky.rotation.inverse()).to_matrix(),
        };
        let extracted =
            ExtractedSky { image: sky.image.clone(), reflections: sky.reflections.clone() };
        values.push((entity, (extracted, uniform)));
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}

#[derive(Resource)]
struct SkyPipeline {
    layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
    // Its own rather than the image's: blending between mip levels is what makes the blur
    // smooth, and an image's sampler may pick the nearest level.
    sampler: Sampler,
}

fn init_sky_pipeline(mut commands: Commands, assets: Res<AssetServer>, device: Res<RenderDevice>) {
    // Bevy's opaque pass binds the view's uniform offset, then the sky's, so those two keep
    // the order Bevy's skybox has them in.
    let layout = BindGroupLayoutDescriptor::new(
        "charpaper_sky_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_cube(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<ViewUniform>(true).visibility(ShaderStages::VERTEX_FRAGMENT),
                uniform_buffer::<SkyUniform>(true),
                texture_cube(TextureSampleType::Float { filterable: true }),
            ),
        ),
    );
    let shader = bevy::asset::load_embedded_asset!(assets.as_ref(), "sky.wgsl");
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("charpaper_sky"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Linear,
        ..default()
    });
    commands.insert_resource(SkyPipeline { layout, shader, sampler });
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct SkyPipelineKey {
    target_format: TextureFormat,
    samples: u32,
}

impl SpecializedRenderPipeline for SkyPipeline {
    type Key = SkyPipelineKey;

    // As Bevy's skybox: drawn at the far plane, only where nothing nearer is.
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("charpaper_sky".into()),
            layout: vec![self.layout.clone()],
            vertex: VertexState { shader: self.shader.clone(), ..default() },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

fn prepare_sky_pipelines(
    mut commands: Commands,
    cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SkyPipeline>>,
    pipeline: Res<SkyPipeline>,
    views: Query<(Entity, &ExtractedView, &Msaa), With<ExtractedSky>>,
) {
    for (entity, view, msaa) in &views {
        let key = SkyPipelineKey { target_format: view.target_format, samples: msaa.samples() };
        let id = pipelines.specialize(&cache, &pipeline, key);
        commands.entity(entity).insert(SkyboxPipelineId(id));
    }
}

fn prepare_sky_bind_groups(
    mut commands: Commands,
    pipeline: Res<SkyPipeline>,
    view_uniforms: Res<ViewUniforms>,
    sky_uniforms: Res<ComponentUniforms<SkyUniform>>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    views: Query<(Entity, &ExtractedSky, &DynamicUniformIndex<SkyUniform>)>,
) {
    for (entity, sky, index) in &views {
        let (Some(view_uniforms), Some(sky_uniforms)) =
            (view_uniforms.uniforms.binding(), sky_uniforms.binding())
        else {
            continue;
        };
        let Some(image) = images.get(&sky.image).filter(|image| is_cube(image)) else {
            commands.entity(entity).remove::<SkyboxBindGroup>();
            continue;
        };
        // Until the reflection map loads, the sharp sky stands in; the roughness is 0 then
        // anyway, or the blur shows once it arrives.
        let reflections = sky
            .reflections
            .as_ref()
            .and_then(|handle| images.get(handle))
            .filter(|image| is_cube(image))
            .unwrap_or(image);
        let bind_group = device.create_bind_group(
            "charpaper_sky",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                &image.texture_view,
                &pipeline.sampler,
                view_uniforms,
                sky_uniforms,
                &reflections.texture_view,
            )),
        );
        commands.entity(entity).insert(SkyboxBindGroup((bind_group, index.index())));
    }
}

// A 2D image in a cube binding fails validation and stops rendering.
fn is_cube(image: &GpuImage) -> bool {
    let dimension = image.texture_view_descriptor.as_ref().and_then(|view| view.dimension);
    dimension == Some(TextureViewDimension::Cube)
}
