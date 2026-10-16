// Volumetric fog, adapted from Bevy 0.19.1's volumetric_fog.wgsl (copyright the Bevy
// contributors, MIT OR Apache-2.0). Bevy's atmosphere, density textures and ambient light are
// left out, since CharPaper's fog uses none of them.
//
// Each fog box is drawn as a cube, or as a plane over the whole screen while the camera is
// inside it. Every pixel it covers marches a ray from the box's near side to its far side or
// the first surface, adding up the light each lamp scatters toward the camera at every step.
//
// At half resolution each fog pixel marches the ray of the top-left screen pixel of the 2×2
// it covers, which fog_upsample.wgsl relies on.

#import bevy_pbr::mesh_view_bindings::{globals, lights, view, clustered_lights}
#import bevy_pbr::mesh_view_types::{
    DIRECTIONAL_LIGHT_FLAGS_VOLUMETRIC_BIT,
    POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    POINT_LIGHT_FLAGS_VOLUMETRIC_BIT,
    POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE,
}
#import bevy_pbr::shadow_sampling::{
    sample_shadow_map_hardware,
    sample_shadow_cubemap,
    sample_shadow_map,
    SPOT_SHADOW_TEXEL_SIZE
}
#import bevy_pbr::shadows::{get_cascade_index, world_to_directional_light_local}
#import bevy_pbr::utils::interleaved_gradient_noise
#import bevy_pbr::view_transformations::{
    depth_ndc_to_view_z,
    frag_coord_to_ndc,
    position_ndc_to_view,
    position_ndc_to_world,
    position_view_to_world
}
#import bevy_render::maths::orthonormalize
#import bevy_pbr::clustered_forward as clustering
#import bevy_pbr::lighting::getDistanceAttenuation;

struct FogBox {
    clip_from_local: mat4x4<f32>,
    // View space planes of the box's far sides, (normal, -normal · point).
    far_planes: array<vec4<f32>, 6>,
    fog_color: vec3<f32>,
    light_tint: vec3<f32>,
    step_count: u32,
    bounding_radius: f32,
    absorption: f32,
    scattering: f32,
    density: f32,
    scattering_asymmetry: f32,
    light_intensity: f32,
    jitter_strength: f32,
}

@group(1) @binding(0) var<uniform> fog_box: FogBox;

#ifdef MULTISAMPLED
@group(1) @binding(1) var depth_texture: texture_depth_multisampled_2d;
#else
@group(1) @binding(1) var depth_texture: texture_depth_2d;
#endif

// 1 / (4π)
const FRAC_4_PI: f32 = 0.07957747154594767;

// The 1 m cube's corners, numbered x + 2y + 4z, by triangle, each facing out counter-clockwise
// so that back faces are culled. Then the screen-filling plane for a camera inside the box.
var<private> CUBE: array<u32, 36> = array<u32, 36>(
    1u, 3u, 7u, 1u, 7u, 5u,
    0u, 4u, 6u, 0u, 6u, 2u,
    2u, 6u, 7u, 2u, 7u, 3u,
    0u, 1u, 5u, 0u, 5u, 4u,
    4u, 5u, 7u, 4u, 7u, 6u,
    0u, 2u, 3u, 0u, 3u, 1u,
);
var<private> PLANE: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
    vec2(-1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0),
);

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var local: vec3<f32>;
    if (index < 36u) {
        let corner = CUBE[index];
        local = vec3(f32(corner & 1u), f32((corner >> 1u) & 1u), f32((corner >> 2u) & 1u)) - 0.5;
    } else {
        local = vec3(PLANE[index - 36u], 0.0);
    }
    return fog_box.clip_from_local * vec4<f32>(local, 1.0);
}

// The screen pixel whose ray this fragment marches.
fn screen_coord(position: vec4<f32>) -> vec4<f32> {
#ifdef HALF_RESOLUTION
    let screen = vec2<i32>(textureDimensions(depth_texture));
    let pixel = min(vec2<i32>(floor(position.xy)) * 2, screen - 1);
    return vec4(vec2<f32>(pixel) + 0.5, position.zw);
#else
    return position;
#endif
}

// Henyey-Greenstein: how much light goes on toward the viewer rather than away, which makes
// light shafts fade in and out as the view turns toward the light.
fn henyey_greenstein(neg_LdotV: f32) -> f32 {
    let g = fog_box.scattering_asymmetry;
    let denom = 1.0 + g * g - 2.0 * g * neg_LdotV;
    return FRAC_4_PI * (1.0 - g * g) / (denom * sqrt(denom));
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let fog_color = fog_box.fog_color;
    let step_count = fog_box.step_count;
    let bounding_radius = fog_box.bounding_radius;
    let absorption = fog_box.absorption;
    let scattering = fog_box.scattering;
    let density = fog_box.density;
    let light_tint = fog_box.light_tint;
    let light_intensity = fog_box.light_intensity;
    let jitter_strength = fog_box.jitter_strength;
    let exposure = view.exposure;

    let frag_coord = screen_coord(position);

    // With MSAA, the first sample's depth: approximate but good enough.
    let depth = textureLoad(depth_texture, vec2<i32>(frag_coord.xy), 0);
    let surface_depth_view = -position_ndc_to_view(
        frag_coord_to_ndc(vec4(frag_coord.xy, depth, 1.0))).z;

    // Only the box's near sides are drawn, so this fragment is where the ray enters it.
    let view_start_pos = position_ndc_to_view(frag_coord_to_ndc(frag_coord));

    // Where the ray leaves the box: the far side it hits in front of all the others.
    var end_depth_view = 0.0;
    for (var plane_index = 0; plane_index < 6; plane_index += 1) {
        let plane = fog_box.far_planes[plane_index];
        let other_plane_a = fog_box.far_planes[(plane_index + 1) % 6];
        let other_plane_b = fog_box.far_planes[(plane_index + 2) % 6];
        let other_plane_c = fog_box.far_planes[(plane_index + 3) % 6];
        let other_plane_d = fog_box.far_planes[(plane_index + 4) % 6];
        let other_plane_e = fog_box.far_planes[(plane_index + 5) % 6];

        let t = -plane.w / dot(plane.xyz, view_start_pos.xyz);
        if (t < 0.0) {
            continue;
        }
        let hit_pos = view_start_pos.xyz * t;

        if (dot(vec4(hit_pos, 1.0), other_plane_a) >= 0.0 &&
            dot(vec4(hit_pos, 1.0), other_plane_b) >= 0.0 &&
            dot(vec4(hit_pos, 1.0), other_plane_c) >= 0.0 &&
            dot(vec4(hit_pos, 1.0), other_plane_d) >= 0.0 &&
            dot(vec4(hit_pos, 1.0), other_plane_e) >= 0.0)
        {
            end_depth_view = -hit_pos.z;
            break;
        }
    }
    end_depth_view = min(end_depth_view, surface_depth_view);

    let start_depth_view = -depth_ndc_to_view_z(frag_coord.z);
    let ray_length_view = max(0.0, end_depth_view - start_depth_view);
    // Behind a surface: nothing to draw.
    if (ray_length_view == 0.0) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }
    let step_size_world = ray_length_view / f32(step_count);

    let Rd_ndc = vec3(frag_coord_to_ndc(frag_coord).xy, 1.0);
    let Rd_view = normalize(position_ndc_to_view(Rd_ndc));
    var Ro_world = position_view_to_world(view_start_pos.xyz);
    let Rd_world = normalize(position_ndc_to_world(Rd_ndc) - view.world_position);

    // Dithering: each pixel starts up to a step further in, which turns banding into noise.
    let jitter = interleaved_gradient_noise(position.xy, globals.frame_count) * jitter_strength;
    Ro_world += Rd_world * jitter;

    var accumulated_color = vec3(0.0);
    // How much of what's behind the fog still shows through.
    var background_alpha = 1.0;

    // Suns. Volumetric ones are sorted first, so the first one that isn't ends the list.
    for (var light_index = 0u; light_index < lights.n_directional_lights; light_index += 1u) {
        let light = &lights.directional_lights[light_index];
        if (((*light).flags & DIRECTIONAL_LIGHT_FLAGS_VOLUMETRIC_BIT) == 0) {
            break;
        }

        let L = (*light).direction_to_light.xyz;
        let depth_offset = (*light).shadow_depth_bias * L;
        let phase = henyey_greenstein(dot(normalize(L), Rd_world));

        background_alpha = 1.0;
        for (var step = 0u; step < step_count; step += 1u) {
            if (background_alpha < 0.001) {
                break;
            }

            let P_world = Ro_world + Rd_world * f32(step) * step_size_world;
            let P_view = view_start_pos + Rd_view * f32(step) * step_size_world;

            background_alpha *= exp(-step_size_world * density * (absorption + scattering));

            let cascade_index = get_cascade_index(light_index, P_view.z);
            let light_local = world_to_directional_light_local(
                light_index,
                cascade_index,
                vec4(P_world + depth_offset, 1.0)
            );

            // Outside the shadow map entirely: unlit.
            var local_light_attenuation = f32(light_local.w != 0.0);
            if (local_light_attenuation != 0.0) {
                let array_index = i32((*light).depth_texture_base_index + cascade_index);
                local_light_attenuation =
                    sample_shadow_map_hardware(light_local.xy, light_local.z, array_index);
            }

            if (local_light_attenuation != 0.0) {
                let light_attenuation = exp(-density * bounding_radius * (absorption + scattering));
                let light_factors_per_step = fog_color * light_tint * light_attenuation *
                    scattering * density * step_size_world * light_intensity * exposure;
                let light_color_per_step = (*light).color.rgb * phase * light_factors_per_step;
                accumulated_color += light_color_per_step * local_light_attenuation *
                    background_alpha;
            }
        }
    }

    // Point and spot lights, found through the clusters each step falls in.
    let is_orthographic = view.clip_from_view[3].w == 1.0;
    background_alpha = 1.0;
    for (var step = 0u; step < step_count; step += 1u) {
        if (background_alpha < 0.001) {
            break;
        }

        let P_world = Ro_world + Rd_world * f32(step) * step_size_world;
        let P_view = view_start_pos + Rd_view * f32(step) * step_size_world;

        var sample_color = vec3(0.0);

        let cluster_index = clustering::view_fragment_cluster_index(frag_coord.xy, P_view.z, is_orthographic);
        var clusterable_object_index_ranges = clustering::unpack_clusterable_object_index_ranges(cluster_index);
        for (var i: u32 = clusterable_object_index_ranges.first_point_light_index_offset;
            i < clusterable_object_index_ranges.first_reflection_probe_index_offset;
            i = i + 1u)
        {
            let light_id = clustering::get_clusterable_object_id(i);
            let light = &clustered_lights.data[light_id];
            if (((*light).flags & POINT_LIGHT_FLAGS_VOLUMETRIC_BIT) == 0) {
                continue;
            }

            let light_to_frag = (*light).position_radius.xyz - P_world;
            let distance_square = dot(light_to_frag, light_to_frag);
            var local_light_attenuation =
                getDistanceAttenuation(distance_square, (*light).color_inverse_square_range.w);
            if (i < clusterable_object_index_ranges.first_spot_light_index_offset) {
                var shadow: f32 = 1.0;
                if (((*light).flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
                    shadow = fetch_point_shadow_without_normal(light_id, vec4(P_world, 1.0), position.xy);
                }
                local_light_attenuation *= shadow;
            } else {
                var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
                spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
                if ((*light).flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u {
                    spot_dir.y = -spot_dir.y;
                }

                // Filament's spot cone, with scale and offset worked out ahead.
                let cd = dot(-spot_dir, normalize(light_to_frag));
                let attenuation = saturate(cd * (*light).light_custom_data.z + (*light).light_custom_data.w);
                let spot_attenuation = attenuation * attenuation;

                var shadow: f32 = 1.0;
                if (((*light).flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
                    shadow = fetch_spot_shadow_without_normal(light_id, vec4(P_world, 1.0), position.xy);
                }
                local_light_attenuation *= spot_attenuation * shadow;
            }

            let light_attenuation = exp(-density * bounding_radius * (absorption + scattering));
            let light_factors_per_step = fog_color * light_tint * light_attenuation *
                scattering * density * step_size_world * light_intensity * exposure;
            let light_color_per_step = (*light).color_inverse_square_range.rgb * light_factors_per_step;
            sample_color += light_color_per_step * local_light_attenuation;
        }

        background_alpha *= exp(-step_size_world * density * (absorption + scattering));
        accumulated_color += sample_color * background_alpha;
    }

    return vec4(accumulated_color, 1.0 - background_alpha);
}

fn fetch_point_shadow_without_normal(light_id: u32, frag_position: vec4<f32>, frag_coord_xy: vec2<f32>) -> f32 {
    let light = &clustered_lights.data[light_id];

    // The cube map's faces are at 45 degrees to the axes, so the depth along a face is the
    // largest of the three distances along the axes.
    let surface_to_light = (*light).position_radius.xyz - frag_position.xyz;
    let surface_to_light_abs = abs(surface_to_light);
    let distance_to_light = max(surface_to_light_abs.x, max(surface_to_light_abs.y, surface_to_light_abs.z));

    let depth_offset = (*light).shadow_depth_bias * normalize(surface_to_light.xyz);
    let offset_position = frag_position.xyz + depth_offset;

    let frag_ls = offset_position.xyz - (*light).position_radius.xyz;
    let abs_position_ls = abs(frag_ls);
    let major_axis_magnitude = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));

    // The projection applied to (0, 0, -major_axis_magnitude, 1), keeping only what affects
    // depth.
    let zw = -major_axis_magnitude * (*light).light_custom_data.xy + (*light).light_custom_data.zw;
    let depth = zw.x / zw.y;

    // Cube maps are left-handed, so z flips.
    let flip_z = vec3(1.0, 1.0, -1.0);
    return sample_shadow_cubemap(frag_ls * flip_z, distance_to_light, depth, light_id, frag_coord_xy);
}

fn fetch_spot_shadow_without_normal(light_id: u32, frag_position: vec4<f32>, frag_coord_xy: vec2<f32>) -> f32 {
    let light = &clustered_lights.data[light_id];

    let surface_to_light = (*light).position_radius.xyz - frag_position.xyz;

    var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
    spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
    if (((*light).flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u) {
        spot_dir.y = -spot_dir.y;
    }

    let fwd = -spot_dir;
    let offset_position =
        -surface_to_light
        + ((*light).shadow_depth_bias * normalize(surface_to_light));

    // A pure rotation, so multiplying from the other side applies its inverse.
    let light_inv_rot = orthonormalize(fwd);
    let projected_position = offset_position * light_inv_rot;

    let f_div_minus_z = 1.0 / ((*light).spot_light_tan_angle * -projected_position.z);
    let shadow_xy_ndc = projected_position.xy * f_div_minus_z;
    let shadow_uv = shadow_xy_ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

    // 0.1 must match Bevy's POINT_LIGHT_NEAR_Z.
    let depth = 0.1 / -projected_position.z;

    return sample_shadow_map(
        shadow_uv,
        depth,
        i32(light_id) + lights.spot_light_shadowmap_offset,
        frag_coord_xy,
        SPOT_SHADOW_TEXEL_SIZE
    );
}
