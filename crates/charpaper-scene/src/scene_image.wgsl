// The scene image as the display camera shows it: graded by the LUT, then with film grain
// added. LUTs from grading apps expect display-ready sRGB values, so that is what they get.
// The grain goes on in a roughly perceptual space and mostly in the midtones, as on film;
// added to linear light it would lift the shadows into digital noise.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput

struct Settings {
    // Intensity, grain size in pixels, 1 for colored grain, unused.
    grain: vec4<f32>,
    // 0 while there is no LUT.
    lut_strength: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var scene_texture: texture_2d<f32>;
@group(1) @binding(1) var scene_sampler: sampler;
@group(1) @binding(2) var<uniform> settings: Settings;
@group(1) @binding(3) var lut: texture_3d<f32>;

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

fn to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3(1.0 / 2.4)) - 0.055;
    return select(high, low, linear <= vec3(0.0031308));
}

fn to_linear(srgb: vec3<f32>) -> vec3<f32> {
    let low = srgb / 12.92;
    let high = pow((srgb + 0.055) / 1.055, vec3(2.4));
    return select(high, low, srgb <= vec3(0.04045));
}

// Trilinear between the eight entries around the color.
fn look_up(color: vec3<f32>) -> vec3<f32> {
    let last = i32(textureDimensions(lut).x) - 1;
    let p = clamp(color, vec3(0.0), vec3(1.0)) * f32(last);
    let low = min(vec3<i32>(floor(p)), vec3(last));
    let high = min(low + 1, vec3(last));
    let t = p - vec3<f32>(low);
    let c000 = textureLoad(lut, vec3(low.x, low.y, low.z), 0).rgb;
    let c100 = textureLoad(lut, vec3(high.x, low.y, low.z), 0).rgb;
    let c010 = textureLoad(lut, vec3(low.x, high.y, low.z), 0).rgb;
    let c110 = textureLoad(lut, vec3(high.x, high.y, low.z), 0).rgb;
    let c001 = textureLoad(lut, vec3(low.x, low.y, high.z), 0).rgb;
    let c101 = textureLoad(lut, vec3(high.x, low.y, high.z), 0).rgb;
    let c011 = textureLoad(lut, vec3(low.x, high.y, high.z), 0).rgb;
    let c111 = textureLoad(lut, vec3(high.x, high.y, high.z), 0).rgb;
    let near = mix(mix(c000, c100, t.x), mix(c010, c110, t.x), t.y);
    let far = mix(mix(c001, c101, t.x), mix(c011, c111, t.x), t.y);
    return mix(near, far, t.z);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(scene_texture, scene_sampler, in.uv);
    if (settings.lut_strength > 0.0) {
        let srgb = to_srgb(color.rgb);
        let graded = mix(srgb, look_up(srgb), settings.lut_strength);
        color = vec4(to_linear(clamp(graded, vec3(0.0), vec3(1.0))), color.a);
    }

    let grain = settings.grain;
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
