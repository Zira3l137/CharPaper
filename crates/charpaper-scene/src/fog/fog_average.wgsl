// Folds each frame's march into a running average of the fog, so that Jitter's noise averages
// away while the camera and the fog stay still, with or without TAA. Only the fog pixels
// marched this frame bring anything new; the others keep their average as it was.
//
// What changes without starting the average over, the character moving through the fog or the
// shadows it casts into it, would linger in the average. So an average is first kept within
// what the fog pixels around it hold now, as TAA does with its history.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct FogAverage {
    // Which fog pixel of each 2×2 was marched this frame, from 0 to 3.
    phase: u32,
    // This frame's share of the average. 1 starts it over, with every fog pixel marched afresh.
    blend: f32,
}

@group(0) @binding(0) var<uniform> average: FogAverage;
@group(0) @binding(1) var marched: texture_2d<f32>;
@group(0) @binding(2) var previous: texture_2d<f32>;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(floor(in.position.xy));
    let fresh = textureLoad(marched, texel, 0);
    if (average.blend >= 1.0) {
        return fresh;
    }
    let old = textureLoad(previous, texel, 0);
    let turn = u32(texel.x & 1) | (u32(texel.y & 1) << 1u);
    if (turn != average.phase) {
        return old;
    }

    let last = vec2<i32>(textureDimensions(marched)) - 1;
    var low = fresh;
    var high = fresh;
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            let around = textureLoad(marched, clamp(texel + vec2(x, y), vec2(0), last), 0);
            low = min(low, around);
            high = max(high, around);
        }
    }
    return mix(clamp(old, low, high), fresh, average.blend);
}
