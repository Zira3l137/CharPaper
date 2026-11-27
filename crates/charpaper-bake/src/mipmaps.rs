//! Mipmaps for textures that come without them. PNG and JPEG images in a glTF have none, and
//! without them a texture seen small shimmers and cutout hair flickers into specks.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    /// 1 to 4 bytes a texel. With 4, the last is alpha.
    pub channels: usize,
    /// The color channels are sRGB-encoded. Alpha never is.
    pub srgb: bool,
}

pub fn level_count(width: u32, height: u32) -> u32 {
    width.max(height).max(1).ilog2() + 1
}

/// `level0` followed by every smaller level down to 1×1, each half the one before, rounded
/// down. Each level is a box filter of the one before, in linear light. With a cutoff, the
/// alpha of every level is scaled so that as much of it passes the cutoff as in `level0`:
/// a cutout that is averaged instead thins out with distance until it vanishes.
pub fn with_mipmaps(level0: Vec<u8>, layout: Layout, alpha_cutoff: Option<f32>) -> Vec<u8> {
    let Layout { mut width, mut height, channels, srgb } = layout;
    debug_assert_eq!(level0.len(), width as usize * height as usize * channels);
    let alpha = (channels == 4).then_some(3);
    let is_color = |c: usize| srgb && Some(c) != alpha;

    let to_linear = to_linear();
    let decode = |i: usize, v: u8| {
        if is_color(i % channels) { to_linear[v as usize] } else { u16::from(v) * 257 }
    };
    let cutout = alpha.zip(alpha_cutoff).map(|(a, cutoff)| {
        let threshold = (cutoff.clamp(0.0, 1.0) * 65535.0).round() as u32;
        let passing =
            level0.chunks_exact(channels).filter(|t| u32::from(t[a]) * 257 >= threshold).count();
        (threshold, passing as f64 / (level0.len() / channels) as f64)
    });
    let mut level: Vec<u16> = Vec::new();
    let mut out = level0;
    out.reserve(out.len() / 3 + channels);
    let to_srgb = to_srgb();
    while width > 1 || height > 1 {
        let (w, h) = ((width / 2).max(1), (height / 2).max(1));
        // The first step reads the bytes as they are, saving a 16-bit copy of the full size.
        level = if level.is_empty() {
            downsample(|i| decode(i, out[i]), (width, height), (w, h), channels)
        } else {
            downsample(|i| level[i], (width, height), (w, h), channels)
        };
        (width, height) = (w, h);
        let scale = match (alpha, cutout) {
            (Some(a), Some((threshold, coverage))) => {
                coverage_scale(&level, channels, a, threshold, coverage)
            }
            _ => 1.0,
        };
        out.extend(level.iter().enumerate().map(|(i, &v)| {
            let c = i % channels;
            if Some(c) == alpha {
                let v = (f64::from(v) * scale).min(65535.0) as u32;
                ((v + 128) / 257) as u8
            } else if is_color(c) {
                to_srgb[v as usize]
            } else {
                ((u32::from(v) + 128) / 257) as u8
            }
        }));
    }
    out
}

// Each target texel averages the source texels under it: 2×2 for even sizes, 3 wide or tall
// where an odd size leaves one over, so no row or column is dropped.
fn downsample(
    source: impl Fn(usize) -> u16,
    (sw, sh): (u32, u32),
    (w, h): (u32, u32),
    channels: usize,
) -> Vec<u16> {
    let span = |i: u32, from: u32, to: u32| {
        let start = i * from / to;
        start..((i + 1) * from / to).max(start + 1)
    };
    let mut target = Vec::with_capacity(w as usize * h as usize * channels);
    let mut sums = vec![0u32; channels];
    for y in 0..h {
        let rows = span(y, sh, h);
        for x in 0..w {
            let columns = span(x, sw, w);
            sums.fill(0);
            for sy in rows.clone() {
                let row = sy as usize * sw as usize;
                for sx in columns.clone() {
                    let texel = (row + sx as usize) * channels;
                    for (c, sum) in sums.iter_mut().enumerate() {
                        *sum += u32::from(source(texel + c));
                    }
                }
            }
            let count = rows.len() as u32 * columns.len() as u32;
            target.extend(sums.iter().map(|&s| ((s + count / 2) / count) as u16));
        }
    }
    target
}

// The scale that brings this level's share of alpha at or above the threshold closest to the
// full-size share (Castaño, "Computing Alpha Mipmaps"). Read off a histogram rather than
// searched for, so it costs one pass over the level.
fn coverage_scale(
    level: &[u16],
    channels: usize,
    alpha: usize,
    threshold: u32,
    coverage: f64,
) -> f64 {
    const BINS: usize = 1024;
    const BIN_WIDTH: u32 = 65536 / BINS as u32;
    if coverage <= 0.0 || threshold == 0 {
        return 1.0;
    }
    let mut histogram = [0u32; BINS];
    for texel in level.chunks_exact(channels) {
        histogram[usize::from(texel[alpha]) / BIN_WIDTH as usize] += 1;
    }
    let wanted = coverage * (level.len() / channels) as f64;
    let mut above = 0u32;
    let (mut best, mut best_bin) = (f64::MAX, BINS - 1);
    for bin in (1..BINS).rev() {
        above += histogram[bin];
        let miss = (f64::from(above) - wanted).abs();
        // On a tie, the larger share: hair that thickens a little reads better than hair that
        // thins away.
        if miss <= best {
            (best, best_bin) = (miss, bin);
        }
        if f64::from(above) >= wanted {
            break;
        }
    }
    f64::from(threshold) / f64::from(best_bin as u32 * BIN_WIDTH)
}

fn to_linear() -> &'static [u16; 256] {
    static TABLE: OnceLock<[u16; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        std::array::from_fn(|v| {
            let c = v as f64 / 255.0;
            let linear = if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
            (linear * 65535.0).round() as u16
        })
    })
}

fn to_srgb() -> &'static [u8] {
    static TABLE: OnceLock<Vec<u8>> = OnceLock::new();
    TABLE.get_or_init(|| {
        (0..=u16::MAX)
            .map(|v| {
                let l = f64::from(v) / 65535.0;
                let c = if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
                (c * 255.0).round() as u8
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(layout: Layout) -> Vec<(u32, u32)> {
        let (mut w, mut h) = (layout.width, layout.height);
        let mut all = vec![(w, h)];
        while w > 1 || h > 1 {
            (w, h) = ((w / 2).max(1), (h / 2).max(1));
            all.push((w, h));
        }
        all
    }

    fn level(data: &[u8], layout: Layout, index: usize) -> &[u8] {
        let sizes = levels(layout);
        let bytes = |(w, h): (u32, u32)| w as usize * h as usize * layout.channels;
        let start: usize = sizes[..index].iter().map(|&s| bytes(s)).sum();
        &data[start..start + bytes(sizes[index])]
    }

    #[test]
    fn counts_levels_down_to_one_texel() {
        assert_eq!(level_count(1, 1), 1);
        assert_eq!(level_count(2, 2), 2);
        assert_eq!(level_count(1024, 512), 11);
        assert_eq!(level_count(5, 3), 3);
        assert_eq!(level_count(0, 0), 1);
    }

    #[test]
    fn writes_every_level_after_the_first() {
        let layout = Layout { width: 6, height: 3, channels: 4, srgb: true };
        let data = with_mipmaps(vec![200; 6 * 3 * 4], layout, None);
        let expected: usize = levels(layout).iter().map(|(w, h)| (w * h) as usize * 4).sum();
        assert_eq!(levels(layout).len() as u32, level_count(6, 3));
        assert_eq!(data.len(), expected);
        assert!(data.iter().all(|&v| v == 200), "a flat color stays the same at every level");
    }

    #[test]
    fn averages_srgb_in_linear_light() {
        let layout = Layout { width: 2, height: 1, channels: 4, srgb: true };
        let data = with_mipmaps(vec![0, 0, 0, 255, 255, 255, 255, 255], layout, None);
        let small = level(&data, layout, 1);
        // Half of white in linear light is sRGB 188, not 128.
        assert_eq!(&small[..3], &[188, 188, 188]);
        assert_eq!(small[3], 255);
    }

    #[test]
    fn averages_linear_data_as_it_is() {
        let layout = Layout { width: 2, height: 2, channels: 1, srgb: false };
        let data = with_mipmaps(vec![0, 100, 200, 100], layout, None);
        assert_eq!(level(&data, layout, 1), &[100]);
    }

    #[test]
    fn keeps_the_odd_column() {
        let layout = Layout { width: 3, height: 1, channels: 1, srgb: false };
        let data = with_mipmaps(vec![0, 0, 255], layout, None);
        assert_eq!(level(&data, layout, 1), &[85]);
    }

    #[test]
    fn cutouts_keep_their_coverage() {
        // Strands of hair: a few solid texels among soft, mostly see-through ones.
        let (width, height) = (256, 64);
        let layout = Layout { width, height, channels: 4, srgb: true };
        let mut seed = 7u32;
        let texels: Vec<u8> = (0..width * height)
            .flat_map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let v = (seed >> 24) as u8;
                [90, 60, 40, if v < 51 { 255 } else { v / 3 }]
            })
            .collect();
        let texels = texels.into_iter();
        let data = with_mipmaps(texels.clone().collect(), layout, Some(0.5));
        let plain = with_mipmaps(texels.collect(), layout, None);
        let passing = |bytes: &[u8]| {
            let alphas: Vec<u8> = bytes.chunks_exact(4).map(|t| t[3]).collect();
            alphas.iter().filter(|&&a| a >= 128).count() as f64 / alphas.len() as f64
        };
        let full = passing(level(&data, layout, 0));
        for index in 1..5 {
            let kept = passing(level(&data, layout, index));
            assert!(
                (kept - full).abs() < 0.1,
                "level {index} shows {kept} of the area, not {full}"
            );
        }
        assert!(passing(level(&plain, layout, 3)) < full / 2.0, "averaged alpha thins out");
    }

    #[test]
    fn leaves_alpha_alone_without_a_cutoff() {
        let layout = Layout { width: 2, height: 1, channels: 4, srgb: false };
        let data = with_mipmaps(vec![0, 0, 0, 0, 0, 0, 0, 100], layout, None);
        assert_eq!(level(&data, layout, 1)[3], 50);
    }
}
