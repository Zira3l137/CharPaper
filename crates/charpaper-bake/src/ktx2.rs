//! Writes cubemaps as KTX2 in RGB9E5: three 9-bit mantissas sharing one 5-bit
//! exponent, 4 bytes a texel. HDR values up to 65408 survive, at a quarter of
//! the size of four half floats, and Bevy samples it natively.

use std::fs;
use std::io;
use std::path::Path;

use crate::cube::CubeLevel;
use crate::panorama::Rgb;

const IDENTIFIER: [u8; 12] =
    [0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n'];
const VK_FORMAT_E5B9G9R9_UFLOAT_PACK32: u32 = 123;
const HEADER_LENGTH: usize = 80;
const LEVEL_INDEX_ENTRY: usize = 24;

/// `levels` from the largest down. Written to a temporary file and renamed
/// into place, so a bake killed half way leaves no truncated map behind for
/// the next start to mistake for a finished one.
pub(crate) fn write(path: &Path, levels: &[CubeLevel]) -> io::Result<()> {
    let bytes = encode(levels);
    let temporary = path.with_extension("ktx2.tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)
}

pub(crate) fn encode(levels: &[CubeLevel]) -> Vec<u8> {
    let size = levels[0].size;
    let dfd = data_format_descriptor();
    let dfd_offset = HEADER_LENGTH + LEVEL_INDEX_ENTRY * levels.len();
    let data_start = (dfd_offset + dfd.len()).next_multiple_of(4);

    // The format stores levels smallest first, while the index lists them
    // largest first.
    let mut offsets = vec![0u64; levels.len()];
    let mut data = Vec::new();
    for (index, level) in levels.iter().enumerate().rev() {
        offsets[index] = (data_start + data.len()) as u64;
        data.extend(level.texels.iter().flat_map(|&t| pack_rgb9e5(t).to_le_bytes()));
    }

    let mut out = Vec::with_capacity(data_start + data.len());
    out.extend(IDENTIFIER);
    for word in [VK_FORMAT_E5B9G9R9_UFLOAT_PACK32, 4, size, size, 0, 0, 6, levels.len() as u32, 0] {
        out.extend(word.to_le_bytes());
    }
    for word in [dfd_offset as u32, dfd.len() as u32, 0, 0] {
        out.extend(word.to_le_bytes());
    }
    out.extend(0u64.to_le_bytes());
    out.extend(0u64.to_le_bytes());
    for (level, offset) in levels.iter().zip(&offsets) {
        let length = (level.texels.len() * 4) as u64;
        out.extend(offset.to_le_bytes());
        out.extend(length.to_le_bytes());
        out.extend(length.to_le_bytes());
    }
    out.extend(dfd);
    out.resize(data_start, 0);
    out.extend(data);
    out
}

/// The smallest valid descriptor: one basic block, linear RGB, 4 bytes a
/// texel, and no per-channel samples. Bevy reads the format from the header's
/// `vkFormat` and only falls back to this block when that is unset, but the
/// format requires one to be present.
fn data_format_descriptor() -> Vec<u8> {
    const BLOCK_SIZE: u32 = 24;
    const RGBSDA: u8 = 1;
    const BT709: u8 = 1;
    const LINEAR: u8 = 1;
    let mut out = Vec::new();
    out.extend((4 + BLOCK_SIZE).to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend(((BLOCK_SIZE << 16) | 2).to_le_bytes());
    out.extend([RGBSDA, BT709, LINEAR, 0]);
    out.extend([0u8; 4]);
    out.extend([4u8, 0, 0, 0, 0, 0, 0, 0]);
    out
}

/// The packing from the `EXT_texture_shared_exponent` specification.
pub(crate) fn pack_rgb9e5(color: Rgb) -> u32 {
    const MANTISSA_BITS: i32 = 9;
    const BIAS: i32 = 15;
    const MAX: f32 = 65408.0;

    let [r, g, b] = color.map(|c| if c.is_finite() { c.clamp(0.0, MAX) } else { 0.0 });
    let brightest = r.max(g).max(b);
    if brightest <= 0.0 {
        return 0;
    }
    let mut exponent = brightest.log2().floor().max((-BIAS - 1) as f32) as i32 + 1 + BIAS;
    let mut scale = 2f32.powi(exponent - BIAS - MANTISSA_BITS);
    if (brightest / scale + 0.5).floor() as u32 == 1 << MANTISSA_BITS {
        exponent += 1;
        scale *= 2.0;
    }
    let mantissa = |c: f32| ((c / scale + 0.5).floor() as u32).min(511);
    mantissa(r) | (mantissa(g) << 9) | (mantissa(b) << 18) | ((exponent as u32) << 27)
}

#[cfg(test)]
pub(crate) fn unpack_rgb9e5(packed: u32) -> Rgb {
    let exponent = (packed >> 27) as i32;
    let scale = 2f32.powi(exponent - 15 - 9);
    [packed & 511, (packed >> 9) & 511, (packed >> 18) & 511].map(|m| m as f32 * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb9e5_round_trips_within_its_precision() {
        for color in [[0.0, 0.0, 0.0], [1.0, 0.5, 0.25], [1234.5, 3.0, 0.001], [60000.0, 1.0, 1.0]]
        {
            let back = unpack_rgb9e5(pack_rgb9e5(color));
            let brightest = color.iter().cloned().fold(0.0f32, f32::max);
            for k in 0..3 {
                // Channels share the brightest one's exponent, so their error
                // is relative to it, not to themselves.
                assert!(
                    (back[k] - color[k]).abs() <= brightest / 256.0 + 1e-6,
                    "{color:?} -> {back:?}"
                );
            }
        }
    }
}
