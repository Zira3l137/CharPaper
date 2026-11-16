// Blender's Perlin noise (Cycles, kernel/svm/noise.h), baked into a 3D texture that the fog
// samples instead of working the noise out at every step. The lattice wraps every PERIOD
// units, so the texture repeats seamlessly; within the first period it is Blender's own noise.

pub(super) const PERIOD: u32 = 16;
pub(super) const TEXELS_PER_UNIT: u32 = 8;
pub(super) const SIZE: u32 = PERIOD * TEXELS_PER_UNIT;

// One signed byte per texel, x fastest, then y, then z: Blender's noise from -1 to 1, which
// filtering then blends between texels at full precision.
pub(super) fn perlin_texels() -> Vec<u8> {
    // The gradient hash of every lattice point, and the fade weights of every texel's offset
    // within its cell, which repeat from cell to cell.
    let hashes: Vec<u32> = (0..PERIOD.pow(3))
        .map(|i| hash(i % PERIOD, i / PERIOD % PERIOD, i / PERIOD / PERIOD))
        .collect();
    let corner = |x: u32, y: u32, z: u32| {
        hashes[((z % PERIOD * PERIOD + y % PERIOD) * PERIOD + x % PERIOD) as usize]
    };
    let offsets: Vec<f32> =
        (0..TEXELS_PER_UNIT).map(|i| (i as f32 + 0.5) / TEXELS_PER_UNIT as f32).collect();

    let mut texels = Vec::with_capacity(SIZE.pow(3) as usize);
    for z in 0..SIZE {
        let (cz, fz) = (z / TEXELS_PER_UNIT, offsets[(z % TEXELS_PER_UNIT) as usize]);
        for y in 0..SIZE {
            let (cy, fy) = (y / TEXELS_PER_UNIT, offsets[(y % TEXELS_PER_UNIT) as usize]);
            for x in 0..SIZE {
                let (cx, fx) = (x / TEXELS_PER_UNIT, offsets[(x % TEXELS_PER_UNIT) as usize]);
                let g = |i: u32, j: u32, k: u32| {
                    let h = corner(cx + i, cy + j, cz + k);
                    grad(h, fx - i as f32, fy - j as f32, fz - k as f32)
                };
                let (u, v, w) = (fade(fx), fade(fy), fade(fz));
                let near =
                    lerp(v, lerp(u, g(0, 0, 0), g(1, 0, 0)), lerp(u, g(0, 1, 0), g(1, 1, 0)));
                let far = lerp(v, lerp(u, g(0, 0, 1), g(1, 0, 1)), lerp(u, g(0, 1, 1), g(1, 1, 1)));
                let value = 0.9820 * lerp(w, near, far);
                texels.push((value.clamp(-1.0, 1.0) * 127.0).round() as i8 as u8);
            }
        }
    }
    texels
}

// Bob Jenkins' lookup3 final mix of three integers, as Blender's hash_uint3.
fn hash(x: u32, y: u32, z: u32) -> u32 {
    let start = 0xdeadbeef_u32.wrapping_add((3 << 2) + 13);
    let (mut a, mut b, mut c) =
        (start.wrapping_add(x), start.wrapping_add(y), start.wrapping_add(z));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(14));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(11));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(25));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(16));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(4));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(14));
    c ^= b;
    c.wrapping_sub(b.rotate_left(24))
}

// One of Perlin's twelve edge gradients, picked by the hash, dotted with the offset.
fn grad(hash: u32, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    (1.0 - t) * a + t * b
}
