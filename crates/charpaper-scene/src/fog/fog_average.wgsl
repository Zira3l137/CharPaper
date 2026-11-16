// Makes the fog that's shown from this frame's march. Interleaved, only one fog pixel in each
// 2×2 was marched this frame and the others kept what they had, which goes stale where
// something moves through the fog without the fog starting over: the character's shadow in a
// light shaft. So the pixels marched now are what every other one is checked against, and a
// pixel not marched now is kept within what those around it hold.
//
// With dithering, each march also brings new noise, which a running average of a pixel's
// marches smooths away while the fog stays still. The average is kept within the noise of the
// pixels marched around it now, as TAA does with its history, and where the mean of those has
// moved by more than their noise explains, the scene changed there and the average starts
// over, so that it never trails behind.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct FogAverage {
    // Which fog pixel of each 2×2 was marched this frame, from 0 to 3.
    phase: u32,
    // This frame's share of the average; 1 without dithering, when nothing is averaged.
    blend: f32,
    // 0 when every fog pixel was marched this frame.
    interleaved: u32,
}

@group(0) @binding(0) var<uniform> average: FogAverage;
@group(0) @binding(1) var marched: texture_2d<f32>;
@group(0) @binding(2) var previous: texture_2d<f32>;

// How far a marched pixel's average may stray from what was marched around it now, in standard
// deviations of those marches: their noise, and how much the fog varies across them.
const STRAY: f32 = 1.0;
// The same for a pixel not marched now, which only a few of those surround. Any tighter and
// their noise would pull its average about.
const STRAY_UNMARCHED: f32 = 2.0;
// How far the mean of what was marched around a pixel may move, in its standard errors, before
// the fog there counts as changed. At 4, noise alone practically never does it.
const CHANGED: f32 = 4.0;

// The fog pixels marched this frame within two of a pixel: up to nine, every other one across
// and down.
struct Fresh {
    low: vec4<f32>,
    high: vec4<f32>,
    mean: vec4<f32>,
    deviation: vec4<f32>,
    // The mean of the previous frame's fog at the same pixels.
    was: vec4<f32>,
    count: f32,
}

fn fresh_around(texel: vec2<i32>) -> Fresh {
    let turn = vec2<i32>(i32(average.phase & 1u), i32(average.phase >> 1u));
    let last = vec2<i32>(textureDimensions(marched)) - 1;
    var fresh = Fresh(vec4(1e30), vec4(-1e30), vec4(0.0), vec4(0.0), vec4(0.0), 0.0);
    var squares = vec4(0.0);
    for (var y = -2; y <= 2; y += 1) {
        for (var x = -2; x <= 2; x += 1) {
            let at = texel + vec2(x, y);
            if (any(at < vec2(0)) || any(at > last) || any((at & vec2(1)) != turn)) {
                continue;
            }
            let value = textureLoad(marched, at, 0);
            fresh.low = min(fresh.low, value);
            fresh.high = max(fresh.high, value);
            fresh.mean += value;
            squares += value * value;
            fresh.was += textureLoad(previous, at, 0);
            fresh.count += 1.0;
        }
    }
    if (fresh.count > 0.0) {
        fresh.mean /= fresh.count;
        fresh.was /= fresh.count;
        fresh.deviation = sqrt(max(squares / fresh.count - fresh.mean * fresh.mean, vec4(0.0)));
    }
    return fresh;
}

fn largest(v: vec4<f32>) -> f32 {
    return max(max(v.x, v.y), max(v.z, v.w));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(floor(in.position.xy));
    let here = textureLoad(marched, texel, 0);
    if (average.interleaved == 0u) {
        return here;
    }
    let fresh = fresh_around(texel);
    if (fresh.count == 0.0) {
        return here;
    }
    let averaging = average.blend < 1.0;
    let turn = u32(texel.x & 1) | (u32(texel.y & 1) << 1u);
    if (turn != average.phase) {
        if (!averaging) {
            return clamp(here, fresh.low, fresh.high);
        }
        let spread = STRAY_UNMARCHED * fresh.deviation;
        return clamp(
            textureLoad(previous, texel, 0),
            min(fresh.low, fresh.mean - spread),
            max(fresh.high, fresh.mean + spread)
        );
    }
    if (!averaging) {
        return here;
    }

    // Never kept from this pixel's own march, so that a peak in the fog isn't worn down.
    let spread = STRAY * fresh.deviation;
    let old = clamp(
        textureLoad(previous, texel, 0),
        min(fresh.mean - spread, here),
        max(fresh.mean + spread, here)
    );
    let error = fresh.deviation / sqrt(fresh.count);
    let moved = abs(fresh.mean - fresh.was) - CHANGED * error;
    let changed = saturate(largest(moved / max(error, vec4(1e-4))));
    return mix(old, here, mix(average.blend, 1.0, changed));
}
