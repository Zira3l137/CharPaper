//! Walking a cubemap's texels as directions.

use crate::panorama::Rgb;

/// One mip level: six faces of `size × size` texels, in the layer order
/// `+X, -X, +Y, -Y, +Z, -Z`, each face row by row from the top.
pub(crate) struct CubeLevel {
    pub size: u32,
    pub texels: Vec<Rgb>,
}

/// Fills one level with `shade(direction)`, a face per thread.
pub(crate) fn render(size: u32, shade: impl Fn([f32; 3]) -> Rgb + Sync) -> CubeLevel {
    let per_face = (size * size) as usize;
    let mut texels = vec![[0.0; 3]; per_face * 6];
    std::thread::scope(|scope| {
        for (face, out) in texels.chunks_mut(per_face).enumerate() {
            let shade = &shade;
            scope.spawn(move || {
                for (i, texel) in out.iter_mut().enumerate() {
                    let (x, y) = ((i as u32 % size) as f32, (i as u32 / size) as f32);
                    let s = 2.0 * (x + 0.5) / size as f32 - 1.0;
                    let t = 2.0 * (y + 0.5) / size as f32 - 1.0;
                    *texel = shade(world(face, s, t));
                }
            });
        }
    });
    CubeLevel { size, texels }
}

/// The world direction a face texel shows, from its position `(s, t)` on the
/// face, each in `[-1, 1]` with `t` growing downwards.
///
/// The table is the usual Vulkan and Direct3D cube layout. Bevy reads cubemaps
/// as left-handed, looking up the world direction `(x, y, z)` at cube
/// coordinate `(x, y, -z)`, so the cube direction's `z` is negated back here.
fn world(face: usize, s: f32, t: f32) -> [f32; 3] {
    let [x, y, z] = match face {
        0 => [1.0, -t, -s],
        1 => [-1.0, -t, s],
        2 => [s, 1.0, t],
        3 => [s, -1.0, -t],
        4 => [s, -t, 1.0],
        _ => [-s, -t, -1.0],
    };
    let length = (x * x + y * y + z * z).sqrt();
    [x / length, y / length, -z / length]
}
