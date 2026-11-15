// Bevy's skybox shader, reading the reflection map too, at the roughness the blur asks for.

#import bevy_render::view::View
#import bevy_pbr::utils::coords_to_viewport_uv

struct SkyUniform {
    brightness: f32,
    roughness: f32,
    transform: mat4x4<f32>,
}

@group(0) @binding(0) var sky: texture_cube<f32>;
@group(0) @binding(1) var sky_sampler: sampler;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> uniforms: SkyUniform;
@group(0) @binding(4) var reflections: texture_cube<f32>;

// From the point on the near plane, which stays finite where the far one is at infinity.
fn ray_direction(position: vec2<f32>, viewport: vec4<f32>) -> vec3<f32> {
    let uv = coords_to_viewport_uv(position, viewport);
    let view_position = view.view_from_clip * vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 1.0, 1.0);
    let direction = (view.world_from_view * vec4(view_position.xyz / view_position.w, 0.0)).xyz;
    return normalize((uniforms.transform * vec4(direction, 0.0)).xyz);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

// One triangle covering the screen, at the far plane.
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    let clip = vec2(f32(index & 1u), f32((index >> 1u) & 1u)) * 4.0 - vec2(1.0);
    return VertexOutput(vec4(clip, 0.0, 1.0));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Cubemaps are left-handed.
    let direction = ray_direction(in.position.xy, view.viewport) * vec3(1.0, 1.0, -1.0);
    var color = textureSample(sky, sky_sampler, direction).rgb;
    if uniforms.roughness > 0.0 {
        // As Bevy's lighting reads it: level = roughness × (levels − 1). The sharp sky gives way
        // over the first level, where the reflection map is still about as sharp.
        let level = uniforms.roughness * f32(textureNumLevels(reflections) - 1u);
        let blurred = textureSampleLevel(reflections, sky_sampler, direction, level).rgb;
        color = mix(color, blurred, saturate(level));
    }
    return vec4(color * uniforms.brightness, 1.0);
}
