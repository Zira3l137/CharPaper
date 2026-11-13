// The passes, each one triangle over its target, as the strokes and the fog draw theirs:
//
// 1. downsample: the scene at half resolution, with each texel's blur radius.
// 2. tiles: the largest near radius in each 16×16 block of those, so the next pass knows how
//    far around to look for something in front that blurs over it.
// 3. gather: the blurred background, the spread-out foreground, and a guess at what is behind
//    the foreground, three half-resolution layers.
// 4. composite: the sharp scene with both laid over it, at full resolution.
//
// The textures come from Bevy's texture cache, which hands the same ones back each frame while
// their size stays the same.

use bevy::asset::embedded_asset;
use bevy::asset::load_embedded_asset;
use bevy::core_pipeline::FullscreenShader;
use bevy::core_pipeline::core_3d::prepare_core_3d_depth_textures;
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::tonemapping::tonemapping;
use bevy::post_process::bloom::bloom;
use bevy::prelude::*;
use bevy::render::GpuResourceAppExt;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderStartup;
use bevy::render::RenderSystems;
use bevy::render::render_resource::binding_types::sampler;
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::render::render_resource::binding_types::texture_depth_2d;
use bevy::render::render_resource::binding_types::texture_depth_2d_multisampled;
use bevy::render::render_resource::binding_types::uniform_buffer;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderContext;
use bevy::render::renderer::RenderDevice;
use bevy::render::renderer::RenderQueue;
use bevy::render::renderer::ViewQuery;
use bevy::render::texture::CachedTexture;
use bevy::render::texture::TextureCache;
use bevy::render::view::ExtractedView;
use bevy::render::view::ViewDepthTexture;
use bevy::render::view::ViewTarget;
use bevy::shader::Shader;
use bevy::shader::ShaderDefVal;

use super::ExtractedDepthOfField;

// Must match the shader's TILE.
const TILE: u32 = 16;
// Samples per layer for each pixel of the gather. Bokeh's harder disk shows gaps sooner.
const BLUR_SAMPLES: u32 = 32;
const BOKEH_SAMPLES: u32 = 48;
const LAYER_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
const TILE_FORMAT: TextureFormat = TextureFormat::R16Float;

pub(super) struct DepthOfFieldPassPlugin;

impl Plugin for DepthOfFieldPassPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "depth_of_field.wgsl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<DepthOfFieldPipeline>>()
            .init_resource::<DepthOfFieldUniforms>()
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(
                Render,
                (
                    prepare_depth_of_field.in_set(RenderSystems::PrepareResources),
                    read_depth_for_blur
                        .in_set(RenderSystems::Prepare)
                        .before(prepare_core_3d_depth_textures),
                ),
            )
            .add_systems(Core3d, draw_depth_of_field.after(bloom).before(tonemapping));
    }
}

#[derive(ShaderType, Clone, Copy)]
struct DepthOfFieldUniform {
    unproject: Vec4,
    focal_distance: f32,
    focal_length: f32,
    coc_scale: f32,
    max_radius: f32,
    max_depth: f32,
    samples: u32,
    bokeh: u32,
}

#[derive(Resource, Default)]
struct DepthOfFieldUniforms(DynamicUniformBuffer<DepthOfFieldUniform>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Step {
    Downsample,
    Tiles,
    Gather,
    Composite,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct PipelineKey {
    step: Step,
    multisampled: bool,
    target_format: TextureFormat,
}

#[derive(Resource)]
struct DepthOfFieldPipeline {
    // Downsample and composite read the depth, which takes another binding with MSAA.
    downsample: [BindGroupLayoutDescriptor; 2],
    tiles: BindGroupLayoutDescriptor,
    gather: BindGroupLayoutDescriptor,
    composite: [BindGroupLayoutDescriptor; 2],
    sampler: Sampler,
    vertex: VertexState,
    shader: Handle<Shader>,
}

impl DepthOfFieldPipeline {
    fn layout(&self, step: Step, multisampled: bool) -> &BindGroupLayoutDescriptor {
        let msaa = usize::from(multisampled);
        match step {
            Step::Downsample => &self.downsample[msaa],
            Step::Tiles => &self.tiles,
            Step::Gather => &self.gather,
            Step::Composite => &self.composite[msaa],
        }
    }
}

fn init_pipeline(
    mut commands: Commands,
    fullscreen: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
    device: Res<RenderDevice>,
) {
    let uniform = || uniform_buffer::<DepthOfFieldUniform>(true);
    let readable = || texture_2d(TextureSampleType::Float { filterable: false });
    let filterable = || texture_2d(TextureSampleType::Float { filterable: true });
    let fragment = ShaderStages::FRAGMENT;
    let downsample = |label, depth| {
        BindGroupLayoutDescriptor::new(
            label,
            &BindGroupLayoutEntries::sequential(fragment, (uniform(), depth, readable())),
        )
    };
    let composite = |label, depth| {
        BindGroupLayoutDescriptor::new(
            label,
            &BindGroupLayoutEntries::sequential(
                fragment,
                (
                    uniform(),
                    depth,
                    readable(),
                    filterable(),
                    filterable(),
                    filterable(),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        )
    };
    commands.insert_resource(DepthOfFieldPipeline {
        downsample: [
            downsample("charpaper_dof_downsample", texture_depth_2d()),
            downsample("charpaper_dof_downsample_multisampled", texture_depth_2d_multisampled()),
        ],
        tiles: BindGroupLayoutDescriptor::new(
            "charpaper_dof_tiles",
            &BindGroupLayoutEntries::sequential(fragment, (uniform(), readable())),
        ),
        gather: BindGroupLayoutDescriptor::new(
            "charpaper_dof_gather",
            &BindGroupLayoutEntries::sequential(fragment, (uniform(), readable(), readable())),
        ),
        composite: [
            composite("charpaper_dof_composite", texture_depth_2d()),
            composite("charpaper_dof_composite_multisampled", texture_depth_2d_multisampled()),
        ],
        sampler: device.create_sampler(&SamplerDescriptor {
            label: Some("charpaper_dof"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        vertex: fullscreen.to_vertex_state(),
        shader: load_embedded_asset!(asset_server.as_ref(), "depth_of_field.wgsl"),
    });
}

impl SpecializedRenderPipeline for DepthOfFieldPipeline {
    type Key = PipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let (def, entry_point, targets) = match key.step {
            Step::Downsample => ("DOWNSAMPLE", "downsample", vec![LAYER_FORMAT]),
            Step::Tiles => ("TILES", "tiles", vec![TILE_FORMAT]),
            Step::Gather => ("GATHER", "gather", vec![LAYER_FORMAT; 3]),
            Step::Composite => ("COMPOSITE", "composite", vec![key.target_format]),
        };
        let mut shader_defs: Vec<ShaderDefVal> = vec![def.into()];
        if matches!(key.step, Step::Downsample | Step::Composite) {
            shader_defs.push("DOWNSAMPLE_OR_COMPOSITE".into());
        }
        if key.multisampled {
            shader_defs.push("MULTISAMPLED".into());
        }
        RenderPipelineDescriptor {
            label: Some(format!("charpaper_dof_{entry_point}").into()),
            layout: vec![self.layout(key.step, key.multisampled).clone()],
            vertex: self.vertex.clone(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some(entry_point.into()),
                targets: targets
                    .into_iter()
                    .map(|format| {
                        Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })
                    })
                    .collect(),
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
struct ViewDepthOfField {
    pipelines: [CachedRenderPipelineId; 4],
    uniform_offset: u32,
    half: CachedTexture,
    tiles: CachedTexture,
    far: CachedTexture,
    near: CachedTexture,
    behind: CachedTexture,
}

// The blur reads the depth of whatever it falls on, which Bevy doesn't allow by default.
fn read_depth_for_blur(mut cameras: Query<&mut Camera3d, With<ExtractedDepthOfField>>) {
    for mut camera in &mut cameras {
        camera.depth_texture_usages.0 |= TextureUsages::TEXTURE_BINDING.bits();
    }
}

fn prepare_depth_of_field(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DepthOfFieldPipeline>>,
    pipeline: Res<DepthOfFieldPipeline>,
    mut uniforms: ResMut<DepthOfFieldUniforms>,
    mut textures: ResMut<TextureCache>,
    views: Query<(Entity, &ExtractedView, &ExtractedDepthOfField, &Msaa)>,
    stale: Query<Entity, (With<ViewDepthOfField>, Without<ExtractedDepthOfField>)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    for view in &stale {
        commands.entity(view).remove::<ViewDepthOfField>();
    }
    uniforms.0.clear();
    let mut prepared = Vec::new();
    for (entity, view, settings, msaa) in &views {
        // A depth-buffer value d turns into a view-space z as (z·d + w) / (z'·d + w'), from the
        // third and fourth columns of the inverse projection: perspective or orthographic alike.
        let view_from_clip = view.clip_from_view.inverse();
        let uniform_offset = uniforms.0.push(&DepthOfFieldUniform {
            unproject: Vec4::new(
                view_from_clip.z_axis.z,
                view_from_clip.w_axis.z,
                view_from_clip.z_axis.w,
                view_from_clip.w_axis.w,
            ),
            focal_distance: settings.focal_distance,
            focal_length: settings.focal_length,
            // So the shader's radii come out in pixels of this view.
            coc_scale: settings.coc_scale * view.viewport.w as f32,
            max_radius: settings.max_radius,
            max_depth: settings.max_depth,
            samples: if settings.bokeh { BOKEH_SAMPLES } else { BLUR_SAMPLES },
            bokeh: u32::from(settings.bokeh),
        });

        let multisampled = *msaa != Msaa::Off;
        let specialize = |step| {
            let key = PipelineKey { step, multisampled, target_format: view.target_format };
            pipelines.specialize(&pipeline_cache, &pipeline, key)
        };
        let pipelines =
            [Step::Downsample, Step::Tiles, Step::Gather, Step::Composite].map(specialize);

        let half = (view.viewport.zw() + UVec2::ONE) / 2;
        let tiles = (half + UVec2::splat(TILE - 1)) / TILE;
        let mut texture = |label: &'static str, size: UVec2, format| {
            textures.get(
                &device,
                TextureDescriptor {
                    label: Some(label),
                    size: Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        prepared.push((
            entity,
            ViewDepthOfField {
                pipelines,
                uniform_offset,
                half: texture("charpaper_dof_half", half, LAYER_FORMAT),
                tiles: texture("charpaper_dof_tiles", tiles, TILE_FORMAT),
                far: texture("charpaper_dof_far", half, LAYER_FORMAT),
                near: texture("charpaper_dof_near", half, LAYER_FORMAT),
                behind: texture("charpaper_dof_behind", half, LAYER_FORMAT),
            },
        ));
    }
    uniforms.0.write_buffer(&device, &queue);
    for (entity, view) in prepared {
        commands.entity(entity).insert(view);
    }
}

fn draw_depth_of_field(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture, &ViewDepthOfField, &Msaa)>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<DepthOfFieldPipeline>,
    uniforms: Res<DepthOfFieldUniforms>,
    mut ctx: RenderContext,
) {
    let (target, depth, dof, msaa) = view.into_inner();
    let Some(uniform) = uniforms.0.binding() else {
        return;
    };
    let pipelines: Option<Vec<_>> =
        dof.pipelines.iter().map(|&id| pipeline_cache.get_render_pipeline(id)).collect();
    let Some(pipelines) = pipelines else {
        return;
    };
    let [downsample, tiles, gather, composite] = pipelines.as_slice() else {
        return;
    };
    let multisampled = *msaa != Msaa::Off;
    let layout = |step| pipeline_cache.get_bind_group_layout(pipeline.layout(step, multisampled));
    let device = ctx.render_device().clone();
    let post_process = target.post_process_write();

    let bind_groups = [
        device.create_bind_group(
            "charpaper_dof_downsample",
            &layout(Step::Downsample),
            &BindGroupEntries::sequential((uniform.clone(), depth.view(), post_process.source)),
        ),
        device.create_bind_group(
            "charpaper_dof_tiles",
            &layout(Step::Tiles),
            &BindGroupEntries::sequential((uniform.clone(), &dof.half.default_view)),
        ),
        device.create_bind_group(
            "charpaper_dof_gather",
            &layout(Step::Gather),
            &BindGroupEntries::sequential((
                uniform.clone(),
                &dof.half.default_view,
                &dof.tiles.default_view,
            )),
        ),
        device.create_bind_group(
            "charpaper_dof_composite",
            &layout(Step::Composite),
            &BindGroupEntries::sequential((
                uniform,
                depth.view(),
                post_process.source,
                &dof.far.default_view,
                &dof.near.default_view,
                &dof.behind.default_view,
                &pipeline.sampler,
            )),
        ),
    ];
    let steps = [
        ("charpaper_dof_downsample", *downsample, vec![&dof.half.default_view]),
        ("charpaper_dof_tiles", *tiles, vec![&dof.tiles.default_view]),
        (
            "charpaper_dof_gather",
            *gather,
            vec![&dof.far.default_view, &dof.near.default_view, &dof.behind.default_view],
        ),
        ("charpaper_dof_composite", *composite, vec![post_process.destination]),
    ];
    for ((label, render_pipeline, outputs), bind_group) in steps.into_iter().zip(&bind_groups) {
        let attachments: Vec<_> = outputs
            .into_iter()
            .map(|view| {
                Some(RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations { load: LoadOp::Clear(default()), store: StoreOp::Store },
                })
            })
            .collect();
        let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(render_pipeline);
        pass.set_bind_group(0, bind_group, &[dof.uniform_offset]);
        pass.draw(0..3, 0..1);
    }
}
