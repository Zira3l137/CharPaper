// Draws the strokes after the scene's opaque and transparent meshes and before the fog, so
// distant lines fade into the fog like everything else. Built like the fog's upsample: one
// triangle over the screen, reading the prepasses' depth and normals, which hold several
// samples per pixel with MSAA and are read differently then.

use bevy::asset::embedded_asset;
use bevy::asset::load_embedded_asset;
use bevy::core_pipeline::FullscreenShader;
use bevy::core_pipeline::prepass::ViewPrepassTextures;
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::schedule::Core3dSystems;
use bevy::prelude::*;
use bevy::render::GpuResourceAppExt;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderStartup;
use bevy::render::RenderSystems;
use bevy::render::extract_component::ComponentUniforms;
use bevy::render::extract_component::DynamicUniformIndex;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::extract_component::UniformComponentPlugin;
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::render::render_resource::binding_types::texture_2d_multisampled;
use bevy::render::render_resource::binding_types::texture_depth_2d;
use bevy::render::render_resource::binding_types::texture_depth_2d_multisampled;
use bevy::render::render_resource::binding_types::uniform_buffer;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderContext;
use bevy::render::renderer::ViewQuery;
use bevy::render::view::ExtractedView;
use bevy::render::view::ViewTarget;
use bevy::shader::Shader;

use super::SceneStrokes;
use crate::fog::FogPass;

pub(super) struct StrokesPassPlugin;

impl Plugin for StrokesPassPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "strokes.wgsl");
        app.add_plugins((
            ExtractComponentPlugin::<SceneStrokes>::default(),
            UniformComponentPlugin::<SceneStrokes>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<StrokesPipeline>>()
            .add_systems(RenderStartup, init_strokes_pipeline)
            .add_systems(Render, prepare_strokes.in_set(RenderSystems::Prepare))
            .add_systems(
                Core3d,
                draw_strokes
                    .after(Core3dSystems::MainPass)
                    .before(FogPass)
                    .before(Core3dSystems::EarlyPostProcess),
            );
    }
}

#[derive(Resource)]
struct StrokesPipeline {
    // Without and with MSAA.
    layouts: [BindGroupLayoutDescriptor; 2],
    vertex: VertexState,
    shader: Handle<Shader>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct StrokesPipelineKey {
    multisampled: bool,
    target_format: TextureFormat,
}

#[derive(Component)]
struct ViewStrokes(CachedRenderPipelineId);

fn init_strokes_pipeline(
    mut commands: Commands,
    fullscreen: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let layout = |label: &'static str, depth, normals| {
        BindGroupLayoutDescriptor::new(
            label,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (uniform_buffer::<SceneStrokes>(true), depth, normals),
            ),
        )
    };
    let normals = TextureSampleType::Float { filterable: false };
    commands.insert_resource(StrokesPipeline {
        layouts: [
            layout("charpaper_strokes", texture_depth_2d(), texture_2d(normals)),
            layout(
                "charpaper_strokes_multisampled",
                texture_depth_2d_multisampled(),
                texture_2d_multisampled(normals),
            ),
        ],
        vertex: fullscreen.to_vertex_state(),
        shader: load_embedded_asset!(asset_server.as_ref(), "strokes.wgsl"),
    });
}

impl SpecializedRenderPipeline for StrokesPipeline {
    type Key = StrokesPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = if key.multisampled { vec!["MULTISAMPLED".into()] } else { vec![] };
        // What's drawn is multiplied by the shader's output: 1 leaves it be, less darkens it
        // in its own color. The scene keeps its alpha.
        let multiply = BlendComponent {
            src_factor: BlendFactor::Dst,
            dst_factor: BlendFactor::Zero,
            operation: BlendOperation::Add,
        };
        let keep = BlendComponent {
            src_factor: BlendFactor::Zero,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        };
        RenderPipelineDescriptor {
            label: Some("charpaper_strokes".into()),
            layout: vec![self.layouts[usize::from(key.multisampled)].clone()],
            vertex: self.vertex.clone(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: Some(BlendState { color: multiply, alpha: keep }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

fn prepare_strokes(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<StrokesPipeline>>,
    strokes_pipeline: Res<StrokesPipeline>,
    views: Query<(Entity, &ExtractedView, &Msaa), With<SceneStrokes>>,
    stale: Query<Entity, (With<ViewStrokes>, Without<SceneStrokes>)>,
) {
    for view in &stale {
        commands.entity(view).remove::<ViewStrokes>();
    }
    for (entity, view, msaa) in &views {
        let key = StrokesPipelineKey {
            multisampled: *msaa != Msaa::Off,
            target_format: view.target_format,
        };
        let pipeline = pipelines.specialize(&pipeline_cache, &strokes_pipeline, key);
        commands.entity(entity).insert(ViewStrokes(pipeline));
    }
}

type StrokedView = (
    &'static ViewTarget,
    &'static ViewPrepassTextures,
    &'static ViewStrokes,
    &'static DynamicUniformIndex<SceneStrokes>,
    &'static Msaa,
);

fn draw_strokes(
    view: ViewQuery<StrokedView, With<SceneStrokes>>,
    pipeline_cache: Res<PipelineCache>,
    strokes_pipeline: Res<StrokesPipeline>,
    uniforms: Res<ComponentUniforms<SceneStrokes>>,
    mut ctx: RenderContext,
) {
    let (target, prepass, strokes, index, msaa) = view.into_inner();
    let Some(pipeline) = pipeline_cache.get_render_pipeline(strokes.0) else {
        return;
    };
    let (Some(uniform), Some(depth), Some(normals)) =
        (uniforms.uniforms().binding(), prepass.depth_view(), prepass.normal_view())
    else {
        return;
    };
    let layout = &strokes_pipeline.layouts[usize::from(*msaa != Msaa::Off)];
    let bind_group = ctx.render_device().create_bind_group(
        "charpaper_strokes",
        &pipeline_cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((uniform, depth, normals)),
    );

    let attachment = [Some(RenderPassColorAttachment {
        view: target.main_texture_view(),
        depth_slice: None,
        resolve_target: None,
        ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
    })];
    let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
        label: Some("charpaper_strokes"),
        color_attachments: &attachment,
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[index.index()]);
    pass.draw(0..3, 0..1);
}
