// Where a sky's sun is, for the scene's sun light to shine from. The skybox is read as Bevy
// loads it: RGB9E5 texels, the six faces of its full-size level first, each laid out as
// cube.rs renders it.

use crate::cube::world;
use crate::ktx2::unpack_rgb9e5;

// How many times brighter than the sky on average its brightest spot must be to count as a
// sun. A clear sky's sun is thousands of times so; a window or a bright cloud, a few.
const SUN_OVER_SKY: f32 = 25.0;
// The first search reads about this many texels across a face.
const SEARCH_ACROSS: u32 = 256;
// Within this angle of the brightest texel, those at least half as bright make up the sun.
const SUN_RADIUS_DEG: f32 = 5.0;
// A sun this far below the horizon, as a share of straight down, is still taken as setting.
const HORIZON: f32 = 0.02;

/// The direction a sky's sun lies in, Y up and before the sky is turned, from a cubemap of
/// `size` texels a face whose full-size level starts `data`. None when no spot stands out
/// from the rest of the sky, as under clouds, or when the brightest is below the horizon.
pub fn sun_direction(size: u32, data: &[u8]) -> Option<[f32; 3]> {
    let per_face = (size as usize).pow(2);
    if size == 0 || data.len() < per_face * 6 * 4 {
        return None;
    }
    let luminance = |face: usize, x: u32, y: u32| {
        let i = (face * per_face + (y * size + x) as usize) * 4;
        let [r, g, b] =
            unpack_rgb9e5(u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]));
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let texel = |at: u32| 2.0 * (at as f32 + 0.5) / size as f32 - 1.0;
    let direction = |face: usize, x: u32, y: u32| world(face, texel(x), texel(y));

    // Every few texels at first, close enough not to step over a sun a few texels wide.
    let stride = (size / SEARCH_ACROSS).max(1);
    let (mut total, mut count) = (0.0_f64, 0_u64);
    let (mut peak, mut found) = (0.0_f32, (0, 0, 0));
    for face in 0..6 {
        for y in (0..size).step_by(stride as usize) {
            for x in (0..size).step_by(stride as usize) {
                let bright = luminance(face, x, y);
                total += f64::from(bright);
                count += 1;
                if bright > peak {
                    (peak, found) = (bright, (face, x, y));
                }
            }
        }
    }
    let mean = (total / count as f64) as f32;
    if peak <= 0.0 || peak < SUN_OVER_SKY * mean {
        return None;
    }

    // Then every texel around what was found, on every face it reaches, as a sun across an
    // edge does: the sun is where it is brightest, weighed by how bright.
    let found = direction(found.0, found.1, found.2);
    let reach = (size / 16).max(2 * stride);
    let around = |c: u32| c.saturating_sub(reach)..(c + reach + 1).min(size);
    let texels = || {
        (0..6).filter_map(move |face| on_face(face, found, size).map(|at| (face, at))).flat_map(
            move |(face, (cx, cy))| {
                around(cy).flat_map(move |y| around(cx).map(move |x| (face, x, y)))
            },
        )
    };
    let (mut brightest, mut centre) = (0.0_f32, found);
    for (face, x, y) in texels() {
        let bright = luminance(face, x, y);
        if bright > brightest {
            (brightest, centre) = (bright, direction(face, x, y));
        }
    }
    let within = SUN_RADIUS_DEG.to_radians().cos();
    let mut sum = [0.0_f64; 3];
    for (face, x, y) in texels() {
        let bright = luminance(face, x, y);
        let towards = direction(face, x, y);
        if bright >= 0.5 * brightest && dot(towards, centre) >= within {
            for (s, d) in sum.iter_mut().zip(towards) {
                *s += f64::from(d * bright);
            }
        }
    }
    let length = sum.iter().map(|s| s * s).sum::<f64>().sqrt();
    if length <= 0.0 {
        return None;
    }
    let sun = sum.map(|s| (s / length) as f32);
    (sun[1] >= -HORIZON).then_some(sun)
}

// The texel of a face a direction points through, or the nearest on its edge, if the
// direction faces that way at all: cube.rs's face table, the other way round.
fn on_face(face: usize, [x, y, z]: [f32; 3], size: u32) -> Option<(u32, u32)> {
    let z = -z;
    let (towards, s, t) = match face {
        0 => (x, -z, -y),
        1 => (-x, z, -y),
        2 => (y, x, z),
        3 => (-y, x, -z),
        4 => (z, x, -y),
        _ => (-z, -x, -y),
    };
    if towards <= 0.0 {
        return None;
    }
    let texel = |c: f32| {
        let at = ((c / towards).clamp(-1.0, 1.0) + 1.0) * 0.5 * size as f32;
        (at as u32).min(size - 1)
    };
    Some((texel(s), texel(t)))
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube;
    use crate::ktx2::pack_rgb9e5;
    use crate::panorama::Panorama;
    use crate::panorama::direction;
    use crate::panorama::equirect;

    // A sky of 0.3 with a sun of 5000 one degree wide, baked into a skybox as the app bakes
    // one, and laid out as Bevy loads it.
    fn skybox(sun: Option<[f32; 3]>, size: u32) -> Vec<u8> {
        let (width, height) = (1024, 512);
        let pixels = (0..width * height)
            .map(|i| {
                let (u, v) = (
                    ((i % width) as f32 + 0.5) / width as f32,
                    ((i / width) as f32 + 0.5) / height as f32,
                );
                let here = direction(u, v);
                let lit = sun.is_some_and(|sun| dot(here, sun) > 1.0_f32.to_radians().cos());
                if lit { [5000.0; 3] } else { [0.3, 0.4, 0.5] }
            })
            .collect();
        let panorama = Panorama::from_pixels(width, height, pixels);
        let level = cube::render(size, |dir| panorama.sharp(dir, size));
        level.texels.iter().flat_map(|&t| pack_rgb9e5(t).to_le_bytes()).collect()
    }

    fn degrees_apart(a: [f32; 3], b: [f32; 3]) -> f32 {
        dot(a, b).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn finds_the_sun_where_the_panorama_has_it() {
        // High in the middle of a face, low near the horizon, at the corner of three faces, and on
        // the edge of two.
        for (u, v) in [(0.3, 0.2), (0.62, 0.47), (0.625, 0.304), (0.5, 0.25)] {
            let sun = direction(u, v);
            let found = sun_direction(256, &skybox(Some(sun), 256)).expect("a sun");
            let apart = degrees_apart(found, sun);
            assert!(apart < 0.5, "sun at {:?} found {apart}° off", equirect(sun));
        }
    }

    #[test]
    fn a_large_skybox_is_searched_coarsely_first_and_still_finds_it() {
        let sun = direction(0.81, 0.33);
        let found = sun_direction(1024, &skybox(Some(sun), 1024)).expect("a sun");
        assert!(degrees_apart(found, sun) < 0.5);
    }

    #[test]
    fn a_sky_without_a_sun_or_with_it_set_has_none() {
        assert_eq!(sun_direction(128, &skybox(None, 128)), None);
        assert_eq!(sun_direction(128, &skybox(Some(direction(0.4, 0.7)), 128)), None);
        assert_eq!(sun_direction(128, &[]), None);
    }
}

#[cfg(test)]
mod faces {
    use super::*;

    #[test]
    fn each_texel_is_found_on_its_own_face() {
        let size = 64;
        for face in 0..6 {
            for (x, y) in [(0, 0), (63, 0), (17, 40), (63, 63), (31, 32)] {
                let s = 2.0 * (x as f32 + 0.5) / size as f32 - 1.0;
                let t = 2.0 * (y as f32 + 0.5) / size as f32 - 1.0;
                assert_eq!(on_face(face, world(face, s, t), size), Some((x, y)), "face {face}");
            }
        }
    }
}
