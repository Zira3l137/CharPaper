// The scene image as the display camera shows it, with film grain added. The grain goes on in
// a roughly perceptual space and mostly in the midtones, as on film; added to linear light it
// would lift the shadows into digital noise.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var scene_texture: texture_2d<f32>;
@group(1) @binding(1) var scene_sampler: sampler;
// Intensity, grain size in pixels, 1 for colored grain, unused.
@group(1) @binding(2) var<uniform> grain: vec4<f32>;

const GAMMA: f32 = 2.2;

// pcg3d, from "Hash Functions for GPU Rendering" (Jarzynski, Olano): uniform in [0, 1].
fn hash(seed: vec3<u32>) -> f32 {
    var v = seed * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> vec3<u32>(16u);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return f32(v.x) / 4294967295.0;
}

// Smoothly blended random values on a grid of grain-sized cells, centered on 0. A new pattern
// every frame and for every channel.
fn noise(pixel: vec2<f32>, size: f32, layer: u32) -> f32 {
    let cell = pixel / size;
    let base = floor(cell);
    let t = smoothstep(vec2(0.0), vec2(1.0), cell - base);
    let i = vec2<u32>(base);
    let a = hash(vec3(i, layer));
    let b = hash(vec3(i + vec2(1u, 0u), layer));
    let c = hash(vec3(i + vec2(0u, 1u), layer));
    let d = hash(vec3(i + vec2(1u, 1u), layer));
    return mix(mix(a, b, t.x), mix(c, d, t.x), t.y) - 0.5;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(scene_texture, scene_sampler, in.uv);
    let intensity = grain.x;
    if (intensity <= 0.0) {
        return color;
    }

    let size = grain.y;
    let layer = globals.frame_count * 3u;
    let gray = noise(in.position.xy, size, layer);
    var amount = vec3(gray);
    if (grain.z > 0.5) {
        amount = vec3(
            gray,
            noise(in.position.xy, size, layer + 1u),
            noise(in.position.xy, size, layer + 2u),
        );
    }

    let perceptual = pow(color.rgb, vec3(1.0 / GAMMA));
    let luma = dot(perceptual, vec3(0.2126, 0.7152, 0.0722));
    let midtones = 1.0 - pow(2.0 * luma - 1.0, 2.0);
    let response = mix(0.25, 1.0, midtones);
    let grained = max(perceptual + amount * (2.0 * intensity * response), vec3(0.0));
    return vec4(pow(grained, vec3(GAMMA)), color.a);
}
