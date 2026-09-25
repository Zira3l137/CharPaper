//! The source panorama, and looking up the light arriving from a direction.

use std::f32::consts::PI;
use std::f32::consts::TAU;
use std::path::Path;

use crate::BakeError;

pub(crate) type Rgb = [f32; 3];

/// An equirectangular panorama, plus copies halved again and again down to a
/// few pixels. A lookup meant to cover a wide cone reads a small copy: one
/// pixel there already holds the average of many, which is what spares the
/// specular filter from needing thousands of samples.
pub(crate) struct Panorama {
    levels: Vec<Level>,
}

struct Level {
    width: usize,
    height: usize,
    pixels: Vec<Rgb>,
}

impl Panorama {
    pub fn load(path: &Path) -> Result<Self, BakeError> {
        let read = |source| BakeError::Read { path: path.to_path_buf(), source };
        let image = image::open(path).map_err(read)?.into_rgb32f();
        let (width, height) = image.dimensions();
        if width != height * 2 {
            return Err(BakeError::NotEquirectangular { path: path.to_path_buf(), width, height });
        }
        let pixels = image
            .pixels()
            .map(|p| p.0.map(|c| if c.is_finite() { c.max(0.0) } else { 0.0 }))
            .collect();
        Ok(Self::from_pixels(width as usize, height as usize, pixels))
    }

    pub fn from_pixels(width: usize, height: usize, pixels: Vec<Rgb>) -> Self {
        let mut levels = vec![Level { width, height, pixels }];
        while levels.last().is_some_and(|l| l.height > 2) {
            let next = levels.last().unwrap().halved();
            levels.push(next);
        }
        Self { levels }
    }

    /// Solid angle one full-size pixel covers at the equator, in steradians.
    pub fn texel_solid_angle(&self) -> f32 {
        let top = &self.levels[0];
        (TAU / top.width as f32) * (PI / top.height as f32)
    }

    /// The light from `dir` at a detail matching a cube face of `face_size`,
    /// so a huge panorama folded onto a small cube does not shimmer.
    pub fn sharp(&self, dir: [f32; 3], face_size: u32) -> Rgb {
        // A face spans a quarter of the panorama's width.
        let ratio = self.levels[0].width as f32 / (4.0 * face_size as f32);
        self.sample(dir, ratio.max(1.0).log2())
    }

    /// The light from `dir`, blurred to `lod` halvings, blending between the
    /// two nearest copies.
    pub fn sample(&self, dir: [f32; 3], lod: f32) -> Rgb {
        let lod = lod.clamp(0.0, (self.levels.len() - 1) as f32);
        let low = lod.floor() as usize;
        let high = (low + 1).min(self.levels.len() - 1);
        let t = lod - low as f32;
        let (u, v) = equirect(dir);
        let a = self.levels[low].bilinear(u, v);
        let b = self.levels[high].bilinear(u, v);
        [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
    }

    /// Every pixel of a small copy with the direction it faces and the solid
    /// angle it covers, for summing the whole sphere.
    pub fn texels(&self, max_height: usize) -> impl Iterator<Item = ([f32; 3], f32, Rgb)> + '_ {
        let level = self
            .levels
            .iter()
            .find(|l| l.height <= max_height)
            .unwrap_or_else(|| self.levels.last().unwrap());
        let (w, h) = (level.width, level.height);
        (0..h).flat_map(move |y| {
            (0..w).map(move |x| {
                let u = (x as f32 + 0.5) / w as f32;
                let v = (y as f32 + 0.5) / h as f32;
                let dir = direction(u, v);
                let latitude = PI * (0.5 - v);
                let solid_angle = (TAU / w as f32) * (PI / h as f32) * latitude.cos();
                (dir, solid_angle, level.pixels[y * w + x])
            })
        })
    }
}

impl Level {
    fn halved(&self) -> Level {
        let (width, height) = (self.width / 2, self.height / 2);
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let at = |dx: usize, dy: usize| self.pixels[(2 * y + dy) * self.width + 2 * x + dx];
                let (a, b, c, d) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
                pixels.push([0, 1, 2].map(|i| (a[i] + b[i] + c[i] + d[i]) * 0.25));
            }
        }
        Level { width, height, pixels }
    }

    /// Wraps around horizontally, where the panorama's edges meet, and clamps
    /// at the poles.
    fn bilinear(&self, u: f32, v: f32) -> Rgb {
        let x = u * self.width as f32 - 0.5;
        let y = (v * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let (x0, y0) = (x.floor(), y.floor());
        let (tx, ty) = (x - x0, y - y0);
        let column = |x: f32| (x as i64).rem_euclid(self.width as i64) as usize;
        let (c0, c1) = (column(x0), column(x0 + 1.0));
        let (r0, r1) = (y0 as usize, (y0 as usize + 1).min(self.height - 1));
        let p = |c: usize, r: usize| self.pixels[r * self.width + c];
        let (a, b, c, d) = (p(c0, r0), p(c1, r0), p(c0, r1), p(c1, r1));
        [0, 1, 2].map(|i| {
            let top = a[i] + (b[i] - a[i]) * tx;
            let bottom = c[i] + (d[i] - c[i]) * tx;
            top + (bottom - top) * ty
        })
    }
}

// Two conventions meet here, and a mistake in either mirrors or turns the sky.
//
// Directions are in Bevy's world: Y up, and as the glTF exporter maps Blender,
// Bevy (x, y, z) is Blender (x, -z, y).
//
// The panorama is laid out as Blender's Environment Texture reads it: the
// image's centre faces Blender's +X, turning towards -Y as it goes right, with
// the top row straight up. That is Cycles' `direction_to_equirectangular`.

/// Where `dir` lands in the panorama, as `(u, v)` with `v` from the top.
pub(crate) fn equirect([x, y, z]: [f32; 3]) -> (f32, f32) {
    let (bx, by, bz) = (x, -z, y);
    let length = (bx * bx + by * by + bz * bz).sqrt().max(f32::MIN_POSITIVE);
    let u = 0.5 - by.atan2(bx) / TAU;
    let v = 0.5 - (bz / length).clamp(-1.0, 1.0).asin() / PI;
    (u, v)
}

/// The direction the panorama's point `(u, v)` faces; the inverse of
/// [`equirect`].
pub(crate) fn direction(u: f32, v: f32) -> [f32; 3] {
    let azimuth = TAU * (0.5 - u);
    let latitude = PI * (0.5 - v);
    let (bx, by, bz) =
        (latitude.cos() * azimuth.cos(), latitude.cos() * azimuth.sin(), latitude.sin());
    [bx, bz, -by]
}
