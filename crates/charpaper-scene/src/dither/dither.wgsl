// See-through materials drawn as solid pixels with gaps, more of them the less opaque the
// surface: like EEVEE's Dithered render method. Being solid where drawn, they're sorted by
// depth pixel by pixel, cast shadows as see-through as they are, and show in the depth that
// depth of field, fog and strokes read. Under TAA the pattern moves every frame and blends
// into smooth transparency; without it, it stays still.
//
// One file for both passes, so the main pass and the prepass decide each pixel alike: a pixel
// the prepass keeps but the main pass drops would be a hole showing the clear color.

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    mesh_view_bindings::view,
    pbr_bindings,
    pbr_functions,
    pbr_types,
}
#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d}

#ifdef BINDLESS
#import bevy_pbr::pbr_bindings::material_indices
#endif  // BINDLESS

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    pbr_prepass_functions,
    prepass_io::{VertexOutput, FragmentOutput},
}
#else   // PREPASS_PIPELINE
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif  // PREPASS_PIPELINE

// The prepass keeps a pixel only this much more surely than the main pass, so rounding never
// makes it keep one the main pass drops. Shadows and depth are a hair thinner for it.
const PREPASS_MARGIN: f32 = 0.004;

fn pcg(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// The material's alpha, as the prepass reads it for a cutout: base color times its texture.
// Vertex colors are left out on purpose, since the prepass doesn't have them.
fn surface_alpha(in: VertexOutput) -> f32 {
#ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    var alpha = pbr_bindings::material_array[material_indices[slot].material].base_color.a;
    let flags = pbr_bindings::material_array[material_indices[slot].material].flags;
#else   // BINDLESS
    var alpha = pbr_bindings::material.base_color.a;
    let flags = pbr_bindings::material.flags;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef STANDARD_MATERIAL_BASE_COLOR_UV_B
    var uv = in.uv_b;
#else   // STANDARD_MATERIAL_BASE_COLOR_UV_B
    var uv = in.uv;
#endif  // STANDARD_MATERIAL_BASE_COLOR_UV_B

#ifdef BINDLESS
    let uv_transform = pbr_bindings::material_array[material_indices[slot].material].uv_transform;
#else   // BINDLESS
    let uv_transform = pbr_bindings::material.uv_transform;
#endif  // BINDLESS

    uv = (uv_transform * vec3(uv, 1.0)).xy;
    if (flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        alpha *= textureSampleBias(
#ifdef BINDLESS
            bindless_textures_2d[material_indices[slot].base_color_texture],
            bindless_samplers_filtering[material_indices[slot].base_color_sampler],
#else   // BINDLESS
            pbr_bindings::base_color_texture,
            pbr_bindings::base_color_sampler,
#endif  // BINDLESS
            uv,
            view.mip_bias
        ).a;
    }
#endif  // VERTEX_UVS
    return alpha;
}

// Interleaved gradient noise (Jimenez 2014): evenly spread thresholds from 0 to 1, offset per
// object so two layers don't share one pattern, and per TAA jitter, so it changes every frame
// under TAA and not at all without it, or in a shadow map.
fn threshold(in: VertexOutput) -> f32 {
    let origin = bitcast<vec3<u32>>(mesh_functions::get_world_from_local(in.instance_index)[3].xyz);
    let object = pcg(origin.x ^ pcg(origin.y ^ pcg(origin.z)));
    let jitter = bitcast<vec2<u32>>(view.clip_from_view[2].xy);
    let frame = pcg(jitter.x ^ pcg(jitter.y)) % 64u;
    let pixel = floor(in.position.xy)
        + vec2(f32(object & 0xffu), f32((object >> 8u) & 0xffu))
        + 5.588238 * f32(frame);
    return fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
}

fn kept(in: VertexOutput, margin: f32) -> bool {
    return surface_alpha(in) > threshold(in) + margin;
}

#ifdef PREPASS_PIPELINE

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
#ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    let flags = pbr_bindings::material_array[material_indices[slot].material].flags;
    let uv_transform = pbr_bindings::material_array[material_indices[slot].material].uv_transform;
#else   // BINDLESS
    let flags = pbr_bindings::material.flags;
    let uv_transform = pbr_bindings::material.uv_transform;
#endif  // BINDLESS

#ifdef VISIBILITY_RANGE_DITHER
    pbr_functions::visibility_range_dither(in.position, in.visibility_range_dither);
#endif  // VISIBILITY_RANGE_DITHER

    if !kept(in, PREPASS_MARGIN) {
        discard;
    }

    var out: FragmentOutput;

#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.frag_depth = in.unclipped_depth;
#endif  // UNCLIPPED_DEPTH_ORTHO_EMULATION

    // Alpha 0 marks the pixel as dithered for the strokes pass, which leaves such pixels
    // alone rather than inking every gap. Bevy reads only the normal's xyz.
#ifdef NORMAL_PREPASS
    if (flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        let double_sided = (flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;
        var normal = pbr_functions::prepare_world_normal(in.world_normal, double_sided, is_front);

#ifdef VERTEX_UVS
#ifdef VERTEX_TANGENTS
#ifdef STANDARD_MATERIAL_NORMAL_MAP
#ifdef STANDARD_MATERIAL_NORMAL_MAP_UV_B
        let uv = (uv_transform * vec3(in.uv_b, 1.0)).xy;
#else   // STANDARD_MATERIAL_NORMAL_MAP_UV_B
        let uv = (uv_transform * vec3(in.uv, 1.0)).xy;
#endif  // STANDARD_MATERIAL_NORMAL_MAP_UV_B
        let Nt = textureSampleBias(
#ifdef BINDLESS
            bindless_textures_2d[material_indices[slot].normal_map_texture],
            bindless_samplers_filtering[material_indices[slot].normal_map_sampler],
#else   // BINDLESS
            pbr_bindings::normal_map_texture,
            pbr_bindings::normal_map_sampler,
#endif  // BINDLESS
            uv,
            view.mip_bias,
        ).rgb;
        let TBN = pbr_functions::calculate_tbn_mikktspace(normal, in.world_tangent);
        normal = pbr_functions::apply_normal_mapping(flags, TBN, double_sided, is_front, Nt);
#endif  // STANDARD_MATERIAL_NORMAL_MAP
#endif  // VERTEX_TANGENTS
#endif  // VERTEX_UVS

        out.normal = vec4(normal * 0.5 + vec3(0.5), 0.0);
    } else {
        out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 0.0);
    }
#endif  // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    out.motion_vector = pbr_prepass_functions::calculate_motion_vector(
        in.world_position,
        in.previous_world_position,
    );
#endif  // MOTION_VECTOR_PREPASS

    return out;
}
#else   // PREPASS_FRAGMENT
@fragment
fn fragment(in: VertexOutput) {
    if !kept(in, PREPASS_MARGIN) {
        discard;
    }
}
#endif  // PREPASS_FRAGMENT

#else   // PREPASS_PIPELINE

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    if !kept(in, 0.0) {
        discard;
    }
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color.a = 1.0;
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

#endif  // PREPASS_PIPELINE
