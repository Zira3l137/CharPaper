// The strokes over a scene, as darkening: 1 leaves a pixel as drawn, less darkens it. See
// pass.rs for how that's applied, and mod.rs for the look.
//
// Depth here is Bevy's reversed depth, the near plane over the distance, 0 for the sky. Across
// any flat surface it changes by the same amount from pixel to pixel, however steeply the
// surface is seen, so it only bends where one surface gives way to another. An outline is a
// pixel standing out in front of its neighbors on either side, a crease one whose neighbors
// face another way.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct Strokes {
    darkening: f32,
    width: f32,
    crease: f32,
    wobble: f32,
    seed: f32,
    fade_depth: f32,
}

@group(0) @binding(0) var<uniform> strokes: Strokes;
#ifdef MULTISAMPLED
@group(0) @binding(1) var depth_texture: texture_depth_multisampled_2d;
@group(0) @binding(2) var normal_texture: texture_multisampled_2d<f32>;
#else
@group(0) @binding(1) var depth_texture: texture_depth_2d;
@group(0) @binding(2) var normal_texture: texture_2d<f32>;
#endif

// How far in front of its neighbors' average a pixel must be, as a share of its own depth, to
// start and to finish becoming an outline.
const OUTLINE_FROM: f32 = 0.03;
const OUTLINE_TO: f32 = 0.08;
// How much more bend than the threshold makes a crease fully dark.
const CREASE_RANGE: f32 = 0.15;

fn clamped(pixel: vec2<i32>) -> vec2<i32> {
    return clamp(pixel, vec2(0), vec2<i32>(textureDimensions(depth_texture)) - 1);
}

fn depth_at(pixel: vec2<i32>) -> f32 {
    return textureLoad(depth_texture, clamped(pixel), 0);
}

fn normal_at(pixel: vec2<i32>) -> vec3<f32> {
    return textureLoad(normal_texture, clamped(pixel), 0).xyz * 2.0 - 1.0;
}

// PCG, from "Hash Functions for GPU Rendering" (Jarzynski and Olano, 2020). Integer hashing is
// exact for any input; the usual fract(sin(…)) hash loses precision as its input grows, and
// lines that boiled from an ever-growing seed stopped wobbling after a few minutes.
fn pcg(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash(cell: vec2<i32>) -> f32 {
    return f32(pcg(bitcast<u32>(cell.x) ^ pcg(bitcast<u32>(cell.y)))) / 4294967295.0;
}

// Value noise: neighboring points get nearly the same value, so lines bend rather than fray.
fn smooth_noise(point: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(point));
    let t = smoothstep(vec2(0.0), vec2(1.0), fract(point));
    let top = mix(hash(cell), hash(cell + vec2(1, 0)), t.x);
    let bottom = mix(hash(cell + vec2(0, 1)), hash(cell + vec2(1, 1)), t.x);
    return mix(top, bottom, t.y);
}

// Where in the noise this step of the boil reads, kept within a thousand cells so the points
// stay precise however long the app runs.
fn boil_offset(salt: u32) -> vec2<f32> {
    let boil = u32(strokes.seed) * 2u + salt;
    return vec2(f32(pcg(boil) & 1023u), f32(pcg(boil ^ 0x9e3779b9u) & 1023u));
}

fn outline(center: vec2<i32>, depth: f32, reach: i32) -> f32 {
    let across = vec2(reach, 0);
    let down = vec2(0, reach);
    let ridge_across = 2.0 * depth - depth_at(center - across) - depth_at(center + across);
    let ridge_down = 2.0 * depth - depth_at(center - down) - depth_at(center + down);
    let ridge = max(ridge_across, ridge_down) / depth;
    return smoothstep(OUTLINE_FROM, OUTLINE_TO, ridge);
}

// Neighbors in the sky have no surface to compare with; the outline covers those edges.
fn crease(center: vec2<i32>, reach: i32) -> f32 {
    let normal = normal_at(center);
    var facing = 1.0;
    for (var i = 0; i < 4; i += 1) {
        let offset = select(vec2(0, reach), vec2(reach, 0), i < 2) * select(1, -1, i % 2 == 1);
        let neighbor = center + offset;
        if (depth_at(neighbor) > 0.0) {
            facing = min(facing, dot(normal, normal_at(neighbor)));
        }
    }
    return smoothstep(strokes.crease, strokes.crease + CREASE_RANGE, 1.0 - facing);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    var center = pixel;
    if (strokes.wobble > 0.0) {
        let point = in.position.xy * 0.03 + boil_offset(0u);
        let wander = vec2(smooth_noise(point), smooth_noise(point + vec2(41.0, 23.0))) - 0.5;
        center = pixel + vec2<i32>(round(wander * 2.0 * strokes.wobble));
    }

    // Lines go on the nearer side of an edge only, so the sky never gets one.
    let depth = depth_at(center);
    if (depth <= 0.0) {
        return vec4(1.0);
    }
    let reach = max(i32(round(strokes.width)), 1);
    var line = max(outline(center, depth, reach), crease(center, reach));
    if (strokes.wobble > 0.0) {
        line *= mix(0.6, 1.0, smooth_noise(in.position.xy * 0.01 + boil_offset(1u)));
    }
    if (strokes.fade_depth > 0.0) {
        line *= clamp(depth / strokes.fade_depth, 0.0, 1.0);
    }
    return vec4(vec3(1.0 - strokes.darkening * line), 1.0);
}
