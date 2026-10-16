// Lays half-resolution fog over the full-resolution scene. Each fog pixel was marched for the
// top-left screen pixel of the 2×2 it covers. A screen pixel blends the nearest ones, but
// leans on those whose surface is as far away as its own, so fog behind the character
// doesn't bleed over its outline, and fog in front of a wall doesn't bleed around it.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var fog_texture: texture_2d<f32>;

#ifdef MULTISAMPLED
@group(0) @binding(1) var depth_texture: texture_depth_multisampled_2d;
#else
@group(0) @binding(1) var depth_texture: texture_depth_2d;
#endif

// How quickly a fog pixel stops counting as its surface's distance differs from this pixel's.
// At 40, 2% farther still counts for half.
const SHARPNESS: f32 = 40.0;

fn depth_at(pixel: vec2<i32>) -> f32 {
    return textureLoad(depth_texture, pixel, 0);
}

// 0 for the same distance, up to 1 when one is endlessly farther than the other. Depth is the
// near plane over the distance, so the two depths' ratio is the inverse of their distances'.
// The sky's depth is 0, as far as can be.
fn depth_gap(a: f32, b: f32) -> f32 {
    let nearer = max(a, b);
    if (nearer <= 0.0) {
        return 0.0;
    }
    return 1.0 - min(a, b) / nearer;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let screen = vec2<i32>(textureDimensions(depth_texture));
    let fog_size = vec2<i32>(textureDimensions(fog_texture));
    let depth = depth_at(pixel);

    // A pixel at even coordinates is exactly where a fog pixel was marched; one at odd
    // coordinates lies halfway to the next.
    let base = pixel / 2;
    let between = vec2<f32>(pixel - base * 2) * 0.5;

    var fog = vec4(0.0);
    var total = 0.0;
    for (var y = 0; y < 2; y += 1) {
        for (var x = 0; x < 2; x += 1) {
            let texel = min(base + vec2(x, y), fog_size - 1);
            let marched_at = min(texel * 2, screen - 1);
            let bilinear = select(1.0 - between.x, between.x, x == 1) *
                select(1.0 - between.y, between.y, y == 1);
            // Never quite zero, so that a pixel unlike all of its neighbors still takes the
            // one most like it.
            let alike = max(exp(-depth_gap(depth, depth_at(marched_at)) * SHARPNESS), 1e-4);
            let weight = bilinear * alike;
            fog += textureLoad(fog_texture, texel, 0) * weight;
            total += weight;
        }
    }
    return fog / max(total, 1e-12);
}
