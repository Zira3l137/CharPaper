// The four passes of the depth of field, picked by a shader def: DOWNSAMPLE, TILES, GATHER
// and COMPOSITE. pass.rs says what each reads and writes.
//
// A blur radius ("circle of confusion", CoC) is signed: negative in front of the focus,
// positive behind it, in pixels of the texture it is stored with.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct Params {
    // A view-space z from a depth-buffer value d: (x·d + y) / (z·d + w).
    unproject: vec4<f32>,
    focal_distance: f32,
    focal_length: f32,
    // focal_length² / (sensor_height · f_stop), times the screen's height in pixels.
    coc_scale: f32,
    // In screen pixels.
    max_radius: f32,
    max_depth: f32,
    samples: u32,
    bokeh: u32,
    foreground: u32,
    highlights: f32,
}

@group(0) @binding(0) var<uniform> params: Params;

// Half-resolution texels in a tile of the near-blur map, across and down.
const TILE: i32 = 16;
const GOLDEN_ANGLE: f32 = 2.39996323;
const EPSILON: f32 = 1.19209290e-07;

fn view_depth(depth: f32) -> f32 {
    let u = params.unproject;
    let w = u.z * depth + u.w;
    // The far plane of an infinite projection, where the sky is.
    if abs(w) < EPSILON {
        return params.max_depth;
    }
    return min(-(u.x * depth + u.y) / w, params.max_depth);
}

// In screen pixels. Without the foreground blur, everything in front stays sharp.
fn coc_radius(depth: f32) -> f32 {
    let focus = params.focal_distance;
    if depth < focus && params.foreground == 0u {
        return 0.0;
    }
    let diameter = params.coc_scale * abs(depth - focus)
        / (depth * max(focus - params.focal_length, EPSILON));
    let radius = min(0.5 * diameter, params.max_radius);
    return select(radius, -radius, depth < focus);
}

// The i-th of n points spread evenly over a disk of radius 1.
fn spiral(i: u32, n: u32) -> vec2<f32> {
    let r = sqrt((f32(i) + 0.5) / f32(n));
    let a = f32(i) * GOLDEN_ANGLE;
    return r * vec2(cos(a), sin(a));
}

// How much of a sample blurred over `radius` reaches a pixel `distance` away. Bokeh is a hard
// disk; the plain blur fades out over its outer half, which reads as a softer blur.
fn coverage(radius: f32, distance: f32) -> f32 {
    if params.bokeh != 0u {
        return saturate(radius - distance + 0.5);
    }
    return saturate((radius - distance) / max(0.5 * radius, 1.0) + 0.5);
}

// Bokeh lets bright spots outweigh the rest, so lights turn into round highlights.
fn brightness_weight(color: vec3<f32>) -> f32 {
    if params.bokeh != 0u {
        return 1.0 + params.highlights * dot(color, vec3(0.2126, 0.7152, 0.0722));
    }
    return 1.0;
}

#ifdef DOWNSAMPLE_OR_COMPOSITE
#ifdef MULTISAMPLED
@group(0) @binding(1) var depth_texture: texture_depth_multisampled_2d;
#else
@group(0) @binding(1) var depth_texture: texture_depth_2d;
#endif
@group(0) @binding(2) var color_texture: texture_2d<f32>;

fn screen_depth(pixel: vec2<i32>) -> f32 {
    return view_depth(textureLoad(depth_texture, pixel, 0));
}
#endif

// Half resolution: the average color of each 2×2 block, with the radius of its nearest pixel,
// so a near object's outline never shrinks here.
#ifdef DOWNSAMPLE
@fragment
fn downsample(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let base = vec2<i32>(floor(in.position.xy)) * 2;
    let last = vec2<i32>(textureDimensions(color_texture)) - 1;
    var color = vec3(0.0);
    var nearest = params.max_depth;
    for (var i = 0; i < 4; i += 1) {
        let pixel = min(base + vec2(i & 1, i >> 1u), last);
        color += textureLoad(color_texture, pixel, 0).rgb;
        nearest = min(nearest, screen_depth(pixel));
    }
    return vec4(color * 0.25, coc_radius(nearest) * 0.5);
}
#endif

// The largest near radius in each tile: how far a near object around there may spread.
#ifdef TILES
@group(0) @binding(1) var half_texture: texture_2d<f32>;

@fragment
fn tiles(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let base = vec2<i32>(floor(in.position.xy)) * TILE;
    let last = vec2<i32>(textureDimensions(half_texture)) - 1;
    var near = 0.0;
    for (var y = 0; y < TILE; y += 1) {
        for (var x = 0; x < TILE; x += 1) {
            near = max(near, -textureLoad(half_texture, min(base + vec2(x, y), last), 0).a);
        }
    }
    return vec4(near, 0.0, 0.0, 0.0);
}
#endif

// Three layers at half resolution, each premultiplied by how much of the pixel it covers, so
// they blend right where they are scaled up.
//
// The far layer blurs what is behind the focus, each pixel by its own radius, from samples
// that themselves blur that far: so a sharp object never smears into the blurred background
// behind it. The near layer is what lies in front of the focus, spread by each sample's own
// radius over whatever is behind it, with how much of the pixel it covers: so a blurred object
// in front blurs past its own outline, as through a lens.
//
// Where a pixel lies on a near object, a lens would also show a little of what is behind the
// object near its edge, which no pixel holds. The behind layer guesses it from the pixels
// around that the near layer doesn't cover.
#ifdef GATHER
@group(0) @binding(1) var half_texture: texture_2d<f32>;
@group(0) @binding(2) var tile_texture: texture_2d<f32>;

struct Layers {
    @location(0) far: vec4<f32>,
    @location(1) near: vec4<f32>,
    @location(2) behind: vec4<f32>,
}

fn load_half(pixel: vec2<i32>) -> vec4<f32> {
    let last = vec2<i32>(textureDimensions(half_texture)) - 1;
    return textureLoad(half_texture, clamp(pixel, vec2(0), last), 0);
}

@fragment
fn gather(in: FullscreenVertexOutput) -> Layers {
    let pixel = vec2<i32>(floor(in.position.xy));
    let center = load_half(pixel);
    var layers: Layers;
    layers.far = vec4(0.0);
    layers.near = vec4(0.0);
    layers.behind = vec4(0.0);

    let far_radius = max(center.a, 0.0);
    if far_radius >= 0.5 {
        var sum = center.rgb * brightness_weight(center.rgb);
        var total = brightness_weight(center.rgb);
        for (var i = 0u; i < params.samples; i += 1u) {
            let offset = spiral(i, params.samples) * far_radius;
            let sample = load_half(pixel + vec2<i32>(round(offset)));
            let weight = coverage(max(sample.a, 0.0), length(offset))
                * brightness_weight(sample.rgb);
            sum += sample.rgb * weight;
            total += weight;
        }
        layers.far = vec4(sum / total, 1.0);
    }

    // The near layer's reach: the largest near radius in this tile and in as many tiles around
    // as the widest blur can span.
    let tile = pixel / TILE;
    let last_tile = vec2<i32>(textureDimensions(tile_texture)) - 1;
    let span = i32(ceil(0.5 * params.max_radius / f32(TILE)));
    var reach = 0.0;
    for (var y = -span; y <= span; y += 1) {
        for (var x = -span; x <= span; x += 1) {
            let at = clamp(tile + vec2(x, y), vec2(0), last_tile);
            reach = max(reach, textureLoad(tile_texture, at, 0).r);
        }
    }
    // Under half a texel, it's sharp enough at full resolution.
    if reach >= 0.5 {
        var sum = vec3(0.0);
        var total = 0.0;
        var covered = 0.0;
        var behind = vec3(0.0);
        var behind_count = 0.0;
        for (var i = 0u; i < params.samples; i += 1u) {
            let offset = spiral(i, params.samples) * reach;
            let sample = load_half(pixel + vec2<i32>(round(offset)));
            let radius = max(-sample.a, 0.0);
            // Each texel spreads over a disk of its own radius, so it covers this pixel by the
            // share of that disk it stands for: one of `samples` cells of the reach's disk.
            // Barely-near texels fade in, so what is almost in focus stays sharp.
            let share = reach * reach / (f32(params.samples) * max(radius * radius, 0.25));
            let weight = coverage(radius, length(offset)) * share
                * smoothstep(0.25, 0.75, radius);
            sum += sample.rgb * weight * brightness_weight(sample.rgb);
            total += weight * brightness_weight(sample.rgb);
            covered += weight;
            if radius < 0.25 {
                behind += sample.rgb;
                behind_count += 1.0;
            }
        }
        if total > 0.0 {
            let alpha = saturate(covered);
            layers.near = vec4(sum / total * alpha, alpha);
        }
        if center.a < -0.25 && behind_count > 0.0 {
            layers.behind = vec4(behind / behind_count, 1.0);
        }
    }
    return layers;
}
#endif

// Full resolution: the sharp picture, its blurred background where this pixel is behind the
// focus or the guess of what is behind where it is clearly in front, and the near layer laid
// over it all.
#ifdef COMPOSITE
@group(0) @binding(3) var far_texture: texture_2d<f32>;
@group(0) @binding(4) var near_texture: texture_2d<f32>;
@group(0) @binding(5) var behind_texture: texture_2d<f32>;
@group(0) @binding(6) var layer_sampler: sampler;

// Four bilinear taps around the point: a small tent that smooths the half-resolution layers
// as they are scaled up.
fn upsample(texture: texture_2d<f32>, at: vec2<f32>) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(texture));
    let uv = at / size;
    let d = 0.5 / size;
    return 0.25 * (
        textureSampleLevel(texture, layer_sampler, uv + vec2(-d.x, -d.y), 0.0)
        + textureSampleLevel(texture, layer_sampler, uv + vec2(d.x, -d.y), 0.0)
        + textureSampleLevel(texture, layer_sampler, uv + vec2(-d.x, d.y), 0.0)
        + textureSampleLevel(texture, layer_sampler, uv + vec2(d.x, d.y), 0.0)
    );
}

@fragment
fn composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let sharp = textureLoad(color_texture, pixel, 0);
    let at = in.position.xy * 0.5;

    var color = sharp.rgb;
    let radius = coc_radius(screen_depth(pixel));
    let far = upsample(far_texture, at);
    if far.a > 0.001 {
        color = mix(color, far.rgb / far.a, smoothstep(0.5, 1.5, radius) * far.a);
    }
    // Barely in front keeps its own color: the near layer hardly covers it.
    let behind = upsample(behind_texture, at);
    if behind.a > 0.001 {
        let amount = smoothstep(1.0, 3.0, -radius) * saturate(behind.a);
        color = mix(color, behind.rgb / behind.a, amount);
    }
    let near = upsample(near_texture, at);
    color = color * (1.0 - near.a) + near.rgb;
    return vec4(color, sharp.a);
}
#endif
