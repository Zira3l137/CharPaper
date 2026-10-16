// Draws the fog. This takes over from Bevy's own volumetric fog pass, which it follows closely
// (Bevy 0.19.1, bevy_pbr/src/volumetric_fog), so that CharPaper decides how the fog is drawn:
// only the fog boxes on a camera's own stage, and timed by the performance monitor like the
// other stages. Bevy still sends the lights' fog flags and shadow maps to the GPU; this pass
// reads them the way Bevy's does.
//
// Bevy's pass stays idle: it only runs for cameras with Bevy's VolumetricFog component, and
// the scene cameras get SceneFog instead.

use bevy::asset::embedded_asset;
use bevy::asset::load_embedded_asset;
use bevy::camera::visibility::RenderLayers;
use bevy::color::ColorToComponents;
use bevy::core_pipeline::core_3d::prepare_core_3d_depth_textures;
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::schedule::Core3dSystems;
use bevy::ecs::query::QueryItem;
use bevy::light::FogVolume;
use bevy::math::Affine3A;
use bevy::math::Vec3A;
use bevy::math::vec4;
use bevy::pbr::MeshPipelineSystems;
use bevy::pbr::MeshPipelineViewLayoutKey;
use bevy::pbr::MeshPipelineViewLayouts;
use bevy::pbr::MeshViewBindGroup;
use bevy::pbr::ViewKeyCache;
use bevy::prelude::*;
use bevy::render::Extract;
use bevy::render::ExtractSchedule;
use bevy::render::GpuResourceAppExt;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderStartup;
use bevy::render::RenderSystems;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::render_resource::binding_types::texture_depth_2d;
use bevy::render::render_resource::binding_types::texture_depth_2d_multisampled;
use bevy::render::render_resource::binding_types::uniform_buffer;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderContext;
use bevy::render::renderer::RenderDevice;
use bevy::render::renderer::RenderQueue;
use bevy::render::renderer::ViewQuery;
use bevy::render::sync_component::SyncComponent;
use bevy::render::view::ExtractedView;
use bevy::render::view::ViewDepthTexture;
use bevy::render::view::ViewTarget;
use bevy::shader::Shader;

pub(crate) struct FogPassPlugin;

impl Plugin for FogPassPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "fog.wgsl");
        app.add_plugins(ExtractComponentPlugin::<SceneFog>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<FogBoxes>()
            .init_gpu_resource::<SpecializedRenderPipelines<FogPipeline>>()
            .init_gpu_resource::<FogUniforms>()
            .add_systems(RenderStartup, init_fog_pipeline.after(MeshPipelineSystems))
            .add_systems(ExtractSchedule, extract_fog_boxes)
            .add_systems(
                Render,
                (
                    prepare_fog.in_set(RenderSystems::Prepare),
                    read_depth_in_fog
                        .in_set(RenderSystems::Prepare)
                        .before(prepare_core_3d_depth_textures),
                ),
            )
            .add_systems(
                Core3d,
                draw_fog.after(Core3dSystems::MainPass).before(Core3dSystems::EarlyPostProcess),
            );
    }
}

// On a scene camera while it draws fog.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct SceneFog {
    pub steps: u32,
    // How far, in meters, a pixel's first step may be pushed in.
    pub jitter: f32,
}

#[derive(Component)]
pub(crate) struct ExtractedFog {
    steps: u32,
    jitter: f32,
    layers: RenderLayers,
}

impl SyncComponent for SceneFog {
    type Target = ExtractedFog;
}

impl ExtractComponent for SceneFog {
    type QueryData = (&'static SceneFog, Option<&'static RenderLayers>);
    type QueryFilter = ();
    type Out = ExtractedFog;

    fn extract_component((fog, layers): QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(ExtractedFog {
            steps: fog.steps,
            jitter: fog.jitter,
            layers: layers.cloned().unwrap_or_default(),
        })
    }
}

// Every fog box in every stage, made anew each frame.
#[derive(Resource, Default)]
struct FogBoxes(Vec<FogBox>);

struct FogBox {
    local_from_world: Affine3A,
    color: Vec3,
    light_tint: Vec3,
    absorption: f32,
    scattering: f32,
    density: f32,
    asymmetry: f32,
    light_intensity: f32,
    layers: RenderLayers,
}

fn extract_fog_boxes(
    mut boxes: ResMut<FogBoxes>,
    volumes: Extract<Query<(&FogVolume, &GlobalTransform, Option<&RenderLayers>)>>,
) {
    boxes.0.clear();
    boxes.0.extend(volumes.iter().map(|(volume, transform, layers)| FogBox {
        local_from_world: transform.affine().inverse(),
        color: volume.fog_color.to_linear().to_vec3(),
        light_tint: volume.light_tint.to_linear().to_vec3(),
        absorption: volume.absorption,
        scattering: volume.scattering,
        density: volume.density_factor,
        asymmetry: volume.scattering_asymmetry,
        light_intensity: volume.light_intensity,
        layers: layers.cloned().unwrap_or_default(),
    }));
}

// The same layout as FogBox in fog.wgsl.
#[derive(ShaderType)]
struct FogUniform {
    clip_from_local: Mat4,
    far_planes: [Vec4; 6],
    fog_color: Vec3,
    light_tint: Vec3,
    step_count: u32,
    bounding_radius: f32,
    absorption: f32,
    scattering: f32,
    density: f32,
    scattering_asymmetry: f32,
    light_intensity: f32,
    jitter_strength: f32,
}

#[derive(Resource, Default, Deref, DerefMut)]
struct FogUniforms(DynamicUniformBuffer<FogUniform>);

#[derive(Resource)]
struct FogPipeline {
    view_layouts: MeshPipelineViewLayouts,
    // Without and with MSAA, whose depth texture is read differently.
    box_layouts: [BindGroupLayoutDescriptor; 2],
    shader: Handle<Shader>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct FogPipelineKey {
    view: MeshPipelineViewLayoutKey,
    target_format: TextureFormat,
}

fn init_fog_pipeline(
    mut commands: Commands,
    view_layouts: Res<MeshPipelineViewLayouts>,
    asset_server: Res<AssetServer>,
) {
    let layout = |label: &'static str, depth| {
        BindGroupLayoutDescriptor::new(
            label,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (uniform_buffer::<FogUniform>(true), depth),
            ),
        )
    };
    commands.insert_resource(FogPipeline {
        view_layouts: view_layouts.clone(),
        box_layouts: [
            layout("charpaper_fog_box", texture_depth_2d()),
            layout("charpaper_fog_box_multisampled", texture_depth_2d_multisampled()),
        ],
        shader: load_embedded_asset!(asset_server.as_ref(), "fog.wgsl"),
    });
}

impl SpecializedRenderPipeline for FogPipeline {
    type Key = FogPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Bevy's choice too: one hardware-filtered shadow sample per step. Softer filtering
        // costs several samples a step and is lost in the fog anyway.
        let mut shader_defs = vec!["SHADOW_FILTER_METHOD_HARDWARE_2X2".into()];
        let multisampled = key.view.contains(MeshPipelineViewLayoutKey::MULTISAMPLED);
        if multisampled {
            shader_defs.push("MULTISAMPLED".into());
        }
        let view_layout = self.view_layouts.get_view_layout(key.view);

        RenderPipelineDescriptor {
            label: Some("charpaper_fog".into()),
            layout: vec![
                view_layout.main_layout,
                self.box_layouts[usize::from(multisampled)].clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                ..default()
            },
            primitive: PrimitiveState { cull_mode: Some(Face::Back), ..default() },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    // Fog over what's already drawn: its light added, the scene behind dimmed
                    // by how much the fog hides it.
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

// What a camera draws this frame: a pipeline, and each fog box on its stage.
#[derive(Component)]
struct ViewFog {
    pipeline: CachedRenderPipelineId,
    boxes: Vec<ViewFogBox>,
}

struct ViewFogBox {
    uniform_offset: u32,
    // Drawn as a cube from outside, as a plane over the screen from inside.
    outside: bool,
}

impl ViewFogBox {
    fn vertices(&self) -> std::ops::Range<u32> {
        if self.outside { 0..36 } else { 36..42 }
    }
}

fn prepare_fog(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<FogPipeline>>,
    fog_pipeline: Res<FogPipeline>,
    mut uniforms: ResMut<FogUniforms>,
    boxes: Res<FogBoxes>,
    views: Query<(Entity, &ExtractedView, &ExtractedFog)>,
    stale: Query<Entity, (With<ViewFog>, Without<ExtractedFog>)>,
    view_keys: Res<ViewKeyCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    for view in &stale {
        commands.entity(view).remove::<ViewFog>();
    }
    let count = views.iter().map(|(_, _, fog)| on_stage(&boxes, fog).count()).sum();
    let Some(mut writer) = uniforms.get_writer(count, &device, &queue) else {
        for (view, ..) in &views {
            commands.entity(view).remove::<ViewFog>();
        }
        return;
    };

    for (entity, view, fog) in &views {
        let Some(view_key) = view_keys.get(&view.retained_view_entity) else {
            continue;
        };
        let key = FogPipelineKey { view: (*view_key).into(), target_format: view.target_format };
        let pipeline = pipelines.specialize(&pipeline_cache, &fog_pipeline, key);

        let world_from_view = view.world_from_view.affine();
        let boxes = on_stage(&boxes, fog)
            .map(|fog_box| {
                let local_from_view = fog_box.local_from_world * world_from_view;
                let view_from_local = local_from_view.inverse();
                let inside = camera_is_inside(&local_from_view);
                let uniform_offset = writer.write(&FogUniform {
                    clip_from_local: clip_from_local(
                        inside,
                        &view.clip_from_view,
                        &view_from_local,
                    ),
                    far_planes: far_planes(&view_from_local),
                    fog_color: fog_box.color,
                    light_tint: fog_box.light_tint,
                    step_count: fog.steps,
                    // Of the sphere around the box.
                    bounding_radius: view_from_local.transform_vector3a(Vec3A::splat(0.5)).length(),
                    absorption: fog_box.absorption,
                    scattering: fog_box.scattering,
                    density: fog_box.density,
                    scattering_asymmetry: fog_box.asymmetry,
                    light_intensity: fog_box.light_intensity,
                    jitter_strength: fog.jitter,
                });
                ViewFogBox { uniform_offset, outside: !inside }
            })
            .collect();
        commands.entity(entity).insert(ViewFog { pipeline, boxes });
    }
}

fn on_stage<'a>(boxes: &'a FogBoxes, fog: &'a ExtractedFog) -> impl Iterator<Item = &'a FogBox> {
    boxes.0.iter().filter(|fog_box| fog_box.layers.intersects(&fog.layers))
}

// The fog reads the depth of whatever it falls on, which Bevy doesn't allow by default.
fn read_depth_in_fog(mut cameras: Query<&mut Camera3d, With<ExtractedFog>>) {
    for mut camera in &mut cameras {
        camera.depth_texture_usages.0 |= TextureUsages::TEXTURE_BINDING.bits();
    }
}

fn draw_fog(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture, &ViewFog, &MeshViewBindGroup, &Msaa)>,
    pipeline_cache: Res<PipelineCache>,
    fog_pipeline: Res<FogPipeline>,
    uniforms: Res<FogUniforms>,
    mut ctx: RenderContext,
) {
    let (target, depth, fog, view_bind_group, msaa) = view.into_inner();
    let (Some(pipeline), Some(uniform)) =
        (pipeline_cache.get_render_pipeline(fog.pipeline), uniforms.binding())
    else {
        return;
    };
    if fog.boxes.is_empty() {
        return;
    }
    let multisampled = !matches!(*msaa, Msaa::Off);
    let box_bind_group = ctx.render_device().create_bind_group(
        "charpaper_fog_box",
        &pipeline_cache.get_bind_group_layout(&fog_pipeline.box_layouts[usize::from(multisampled)]),
        &BindGroupEntries::sequential((uniform, BindingResource::TextureView(depth.view()))),
    );

    let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
        label: Some("charpaper_fog"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: target.main_texture_view(),
            depth_slice: None,
            resolve_target: None,
            ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &view_bind_group.main, &view_bind_group.main_offsets);
    for fog_box in &fog.boxes {
        pass.set_bind_group(1, &box_bind_group, &[fog_box.uniform_offset]);
        pass.draw(fog_box.vertices(), 0..1);
    }
}

// The box is the 1 m cube around the origin in its own space.
fn camera_is_inside(local_from_view: &Affine3A) -> bool {
    local_from_view.translation.abs().cmple(Vec3A::splat(0.5)).all()
}

fn clip_from_local(inside: bool, clip_from_view: &Mat4, view_from_local: &Affine3A) -> Mat4 {
    if !inside {
        return *clip_from_view * Mat4::from(*view_from_local);
    }
    // A plane over the whole screen at the near clip plane, where the march then starts.
    let z_near = clip_from_view.w_axis[2];
    Mat4::from_cols(
        vec4(z_near, 0.0, 0.0, 0.0),
        vec4(0.0, z_near, 0.0, 0.0),
        vec4(0.0, 0.0, 0.0, 0.0),
        vec4(0.0, 0.0, z_near, z_near),
    )
}

// The planes of the sides facing away from the camera, in view space. Fewer than six leave
// zero planes at the end, which the shader skips.
fn far_planes(view_from_local: &Affine3A) -> [Vec4; 6] {
    let mut planes = [Vec4::ZERO; 6];
    let mut next = 0;
    for normal in [Vec3A::X, Vec3A::NEG_X, Vec3A::Y, Vec3A::NEG_Y, Vec3A::Z, Vec3A::NEG_Z] {
        let view_normal = view_from_local.transform_vector3a(normal).normalize_or_zero();
        let view_point = view_from_local.transform_point3a(-normal * 0.5);
        let plane = view_normal.extend(-view_normal.dot(view_point));
        if plane.w > 0.0 {
            planes[next] = plane;
            next += 1;
        }
    }
    planes
}
