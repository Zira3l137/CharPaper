// Draws the fog. This takes over from Bevy's own volumetric fog pass, which it follows closely
// (Bevy 0.19.1, bevy_pbr/src/volumetric_fog), so that CharPaper decides how the fog is drawn:
// only the fog boxes on a camera's own stage, and timed by the performance monitor like the
// other stages. Bevy still sends the lights' fog flags and shadow maps to the GPU; this pass
// reads them the way Bevy's does.
//
// Bevy's pass stays idle: it only runs for cameras with Bevy's VolumetricFog component, and
// the scene cameras get SceneFog instead.
//
// The fog is marched into a texture of its own, kept from frame to frame, at full, half or a
// quarter of the screen's resolution, then fog_upsample.wgsl lays it over the scene. Fog is
// soft, so a lower resolution costs little in looks and saves most of the fog's work.
//
// While the camera, the fog boxes and the fog settings stay as they were, only one fog pixel
// in each 2×2 is marched again each frame, in turn, and the rest keep what they had. A
// wallpaper's camera is still nearly all the time, so this saves about three quarters of the
// march nearly all the time. What does change meanwhile, the character moving through the fog
// or the shadows it casts into it, shows up within four frames, which TAA blends over.

use bevy::asset::embedded_asset;
use bevy::asset::load_embedded_asset;
use bevy::camera::visibility::RenderLayers;
use bevy::color::ColorToComponents;
use bevy::core_pipeline::FullscreenShader;
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
use bevy::render::render_resource::binding_types::texture_2d;
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
use bevy::shader::ShaderDefVal;

pub(crate) struct FogPassPlugin;

impl Plugin for FogPassPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "fog.wgsl");
        embedded_asset!(app, "fog_upsample.wgsl");
        app.add_plugins(ExtractComponentPlugin::<SceneFog>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<FogBoxes>()
            .init_gpu_resource::<SpecializedRenderPipelines<FogPipeline>>()
            .init_gpu_resource::<SpecializedRenderPipelines<UpsamplePipeline>>()
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
                draw_fog
                    .in_set(FogPass)
                    .after(Core3dSystems::MainPass)
                    .before(Core3dSystems::EarlyPostProcess),
            );
    }
}

// Where the fog is drawn over the scene, for passes that must come before or after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FogPass;

// On a scene camera while it draws fog.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct SceneFog {
    pub max_steps: u32,
    // In meters.
    pub step_length: f32,
    // Each pixel's ray starts up to a step further in, different every frame, which turns
    // the stripes between steps into noise that TAA smooths out.
    pub dithering: bool,
    // Screen pixels per fog pixel, across and down: 1, 2 or 4.
    pub downscale: u32,
}

#[derive(Component, Clone, PartialEq)]
pub(crate) struct ExtractedFog {
    max_steps: u32,
    step_length: f32,
    dithering: bool,
    downscale: u32,
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
            max_steps: fog.max_steps,
            step_length: fog.step_length,
            dithering: fog.dithering,
            downscale: fog.downscale,
            layers: layers.cloned().unwrap_or_default(),
        })
    }
}

// Every fog box in every stage, made anew each frame.
#[derive(Resource, Default)]
struct FogBoxes(Vec<FogBox>);

#[derive(Clone, PartialEq)]
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
    max_steps: u32,
    step_length: f32,
    bounding_radius: f32,
    absorption: f32,
    scattering: f32,
    density: f32,
    scattering_asymmetry: f32,
    light_intensity: f32,
    // 1 with dithering, 0 without.
    dithering: f32,
    // Which fog pixel of each 2×2 is marched this frame, from 0 to 3.
    phase: u32,
    // 1 while only that one is, 0 when all are.
    interleaved: u32,
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
    downscale: u32,
    // Clears the fog pixels about to be marched again, rather than marching a box.
    clear: bool,
}

// The fog's own texture keeps the fog's light and how much it hides. Precise enough for light
// and blendable on every GPU.
const FOG_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

// Lays the fog's own texture over the scene.
#[derive(Resource)]
struct UpsamplePipeline {
    // Without and with MSAA, as for the boxes.
    layouts: [BindGroupLayoutDescriptor; 2],
    vertex: VertexState,
    shader: Handle<Shader>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct UpsamplePipelineKey {
    multisampled: bool,
    target_format: TextureFormat,
    downscale: u32,
}

fn init_fog_pipeline(
    mut commands: Commands,
    view_layouts: Res<MeshPipelineViewLayouts>,
    fullscreen: Res<FullscreenShader>,
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

    let layout = |label: &'static str, depth| {
        BindGroupLayoutDescriptor::new(
            label,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (texture_2d(TextureSampleType::Float { filterable: false }), depth),
            ),
        )
    };
    commands.insert_resource(UpsamplePipeline {
        layouts: [
            layout("charpaper_fog_upsample", texture_depth_2d()),
            layout("charpaper_fog_upsample_multisampled", texture_depth_2d_multisampled()),
        ],
        vertex: fullscreen.to_vertex_state(),
        shader: load_embedded_asset!(asset_server.as_ref(), "fog_upsample.wgsl"),
    });
}

impl SpecializedRenderPipeline for FogPipeline {
    type Key = FogPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Bevy's choice too: one hardware-filtered shadow sample per step. Softer filtering
        // costs several samples a step and is lost in the fog anyway.
        let mut shader_defs = vec![
            "SHADOW_FILTER_METHOD_HARDWARE_2X2".into(),
            ShaderDefVal::UInt("DOWNSCALE".into(), key.downscale),
        ];
        let multisampled = key.view.contains(MeshPipelineViewLayoutKey::MULTISAMPLED);
        if multisampled {
            shader_defs.push("MULTISAMPLED".into());
        }
        let view_layout = self.view_layouts.get_view_layout(key.view);
        // The clear overwrites; the boxes add up over what's there.
        let (vertex, fragment, blend) = match key.clear {
            true => ("clear_vertex", "clear_fragment", None),
            false => ("vertex", "fragment", Some(fog_blend(true))),
        };

        RenderPipelineDescriptor {
            label: Some("charpaper_fog".into()),
            layout: vec![
                view_layout.main_layout,
                self.box_layouts[usize::from(multisampled)].clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some(vertex.into()),
                ..default()
            },
            // The clear's triangle covers the screen whichever way it faces.
            primitive: PrimitiveState {
                cull_mode: (!key.clear).then_some(Face::Back),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some(fragment.into()),
                targets: vec![Some(ColorTargetState {
                    format: FOG_FORMAT,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            ..default()
        }
    }
}

impl SpecializedRenderPipeline for UpsamplePipeline {
    type Key = UpsamplePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![ShaderDefVal::UInt("DOWNSCALE".into(), key.downscale)];
        if key.multisampled {
            shader_defs.push("MULTISAMPLED".into());
        }
        RenderPipelineDescriptor {
            label: Some("charpaper_fog_upsample".into()),
            layout: vec![self.layouts[usize::from(key.multisampled)].clone()],
            vertex: self.vertex.clone(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: Some(fog_blend(false)),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

// Fog over what's already drawn: its light added, what's behind dimmed by how much the fog
// hides it. The scene keeps its own alpha. In the fog's own texture, apart from the scene,
// how much it hides adds up instead, so that it can be laid over the scene in one go.
fn fog_blend(apart: bool) -> BlendState {
    let over = BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    };
    let keep = BlendComponent {
        src_factor: BlendFactor::Zero,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    };
    BlendState { color: over, alpha: if apart { over } else { keep } }
}

// What a camera draws this frame: the pipelines, and each fog box on its stage.
#[derive(Component)]
struct ViewFog {
    pipeline: CachedRenderPipelineId,
    clear: CachedRenderPipelineId,
    upsample: CachedRenderPipelineId,
    boxes: Vec<ViewFogBox>,
    texture: TextureView,
    // Only some fog pixels are marched again, the rest kept.
    interleaved: bool,
}

// A camera's fog texture, kept from frame to frame, and what its fog was last marched for.
#[derive(Component)]
struct FogHistory {
    _texture: Texture,
    view: TextureView,
    size: UVec2,
    marched_for: MarchedFor,
    phase: u32,
}

// Everything but the lights and the scene's depth that decides how the fog comes out.
#[derive(PartialEq)]
struct MarchedFor {
    world_from_view: GlobalTransform,
    clip_from_view: Mat4,
    fog: ExtractedFog,
    boxes: Vec<FogBox>,
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
    mut upsample_pipelines: ResMut<SpecializedRenderPipelines<UpsamplePipeline>>,
    upsample_pipeline: Res<UpsamplePipeline>,
    mut uniforms: ResMut<FogUniforms>,
    boxes: Res<FogBoxes>,
    views: Query<(Entity, &ExtractedView, &ExtractedFog)>,
    mut histories: Query<&mut FogHistory>,
    stale: Query<Entity, (Or<(With<ViewFog>, With<FogHistory>)>, Without<ExtractedFog>)>,
    view_keys: Res<ViewKeyCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    for view in &stale {
        commands.entity(view).remove::<(ViewFog, FogHistory)>();
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
        let view_key: MeshPipelineViewLayoutKey = (*view_key).into();
        let upsample = upsample_pipelines.specialize(
            &pipeline_cache,
            &upsample_pipeline,
            UpsamplePipelineKey {
                multisampled: view_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED),
                target_format: view.target_format,
                downscale: fog.downscale,
            },
        );
        let key = |clear| FogPipelineKey { view: view_key, downscale: fog.downscale, clear };
        let pipeline = pipelines.specialize(&pipeline_cache, &fog_pipeline, key(false));
        let clear = pipelines.specialize(&pipeline_cache, &fog_pipeline, key(true));

        // Rounded up, so the last row and column of an odd-sized screen still have fog.
        let screen = view.viewport.zw();
        let size = ((screen + UVec2::splat(fog.downscale - 1)) / fog.downscale).max(UVec2::ONE);
        let marched_for = MarchedFor {
            world_from_view: view.world_from_view,
            clip_from_view: view.clip_from_view,
            fog: fog.clone(),
            boxes: on_stage(&boxes, fog).cloned().collect(),
        };
        let (texture, interleaved, phase) = match histories.get_mut(entity) {
            Ok(mut history) if history.size == size => {
                let interleaved = history.marched_for == marched_for;
                history.marched_for = marched_for;
                history.phase = (history.phase + 1) % 4;
                (history.view.clone(), interleaved, history.phase)
            }
            // New, or the screen changed size: every fog pixel is marched afresh.
            _ => {
                let (texture, view) = fog_texture(&device, size);
                commands.entity(entity).insert(FogHistory {
                    _texture: texture,
                    view: view.clone(),
                    size,
                    marched_for,
                    phase: 0,
                });
                (view, false, 0)
            }
        };

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
                    max_steps: fog.max_steps,
                    step_length: fog.step_length,
                    // Of the sphere around the box.
                    bounding_radius: view_from_local.transform_vector3a(Vec3A::splat(0.5)).length(),
                    absorption: fog_box.absorption,
                    scattering: fog_box.scattering,
                    density: fog_box.density,
                    scattering_asymmetry: fog_box.asymmetry,
                    light_intensity: fog_box.light_intensity,
                    dithering: if fog.dithering { 1.0 } else { 0.0 },
                    phase,
                    interleaved: u32::from(interleaved),
                });
                ViewFogBox { uniform_offset, outside: !inside }
            })
            .collect();
        commands.entity(entity).insert(ViewFog {
            pipeline,
            clear,
            upsample,
            boxes,
            texture,
            interleaved,
        });
    }
}

fn fog_texture(device: &RenderDevice, size: UVec2) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("charpaper_fog"),
        size: Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: FOG_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
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
    upsample_pipeline: Res<UpsamplePipeline>,
    uniforms: Res<FogUniforms>,
    mut ctx: RenderContext,
) {
    let (target, depth, fog, view_bind_group, msaa) = view.into_inner();
    let pipelines = (
        pipeline_cache.get_render_pipeline(fog.pipeline),
        pipeline_cache.get_render_pipeline(fog.clear),
        pipeline_cache.get_render_pipeline(fog.upsample),
    );
    let ((Some(pipeline), Some(clear), Some(upsample)), Some(uniform)) =
        (pipelines, uniforms.binding())
    else {
        return;
    };
    let Some(first) = fog.boxes.first() else {
        return;
    };
    let multisampled = usize::from(!matches!(*msaa, Msaa::Off));
    let box_bind_group = ctx.render_device().create_bind_group(
        "charpaper_fog_box",
        &pipeline_cache.get_bind_group_layout(&fog_pipeline.box_layouts[multisampled]),
        &BindGroupEntries::sequential((uniform, BindingResource::TextureView(depth.view()))),
    );

    {
        // Every fog pixel marched this frame starts from no fog at all. Interleaved, the
        // others keep what they had, so only those are cleared, which the first box's uniform
        // says.
        let attachment = attachment(&fog.texture, !fog.interleaved);
        let mut pass = ctx.command_encoder().begin_render_pass(&color_pass(&attachment));
        pass.set_bind_group(0, &view_bind_group.main, &view_bind_group.main_offsets);
        if fog.interleaved {
            pass.set_pipeline(clear);
            pass.set_bind_group(1, &box_bind_group, &[first.uniform_offset]);
            pass.draw(0..3, 0..1);
        }
        pass.set_pipeline(pipeline);
        for fog_box in &fog.boxes {
            pass.set_bind_group(1, &box_bind_group, &[fog_box.uniform_offset]);
            pass.draw(fog_box.vertices(), 0..1);
        }
    }

    let upsample_bind_group = ctx.render_device().create_bind_group(
        "charpaper_fog_upsample",
        &pipeline_cache.get_bind_group_layout(&upsample_pipeline.layouts[multisampled]),
        &BindGroupEntries::sequential((&fog.texture, BindingResource::TextureView(depth.view()))),
    );
    let attachment = attachment(target.main_texture_view(), false);
    let mut pass = ctx.command_encoder().begin_render_pass(&color_pass(&attachment));
    pass.set_pipeline(upsample);
    pass.set_bind_group(0, &upsample_bind_group, &[]);
    pass.draw(0..3, 0..1);
}

fn color_pass<'a>(
    attachment: &'a [Option<RenderPassColorAttachment<'a>>],
) -> RenderPassDescriptor<'a> {
    RenderPassDescriptor {
        label: Some("charpaper_fog"),
        color_attachments: attachment,
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    }
}

fn attachment(view: &TextureView, clear: bool) -> [Option<RenderPassColorAttachment<'_>>; 1] {
    let load = if clear { LoadOp::Clear(LinearRgba::NONE.into()) } else { LoadOp::Load };
    [Some(RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: Operations { load, store: StoreOp::Store },
    })]
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
