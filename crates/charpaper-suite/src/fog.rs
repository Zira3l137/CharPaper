use crate::manifest::Fog;
use crate::manifest::FogNoise;

/// Octaves past the first that the app draws: each costs a texture read at every step.
pub const MAX_NOISE_DETAIL: f32 = 3.0;
/// Points of the density curve the app keeps, from noise value 0 to 1.
pub const NOISE_CURVE_POINTS: usize = 32;

// How far one octave of Blender's Perlin noise spreads around 0: its standard deviation,
// measured over many points.
const NOISE_SPREAD: f32 = 0.264;

// Bevy's fog in Blender's terms. Blender's Principled Volume works per color channel:
//   scattering = color × density
//   absorption = (1 − color) × (1 − √absorption_color) × density
// Bevy has one number for each, so the channels are averaged. Bevy tints scattered light by
// fog color × scattering, which makes the fog color Blender's color over its average.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Medium {
    pub color: [f32; 3],
    // With noise, the density the fog has on average.
    pub density: f32,
    pub scattering: f32,
    pub absorption: f32,
    pub anisotropy: f32,
    pub noise: Option<NoiseMedium>,
}

// Blender's fBM noise: each octave Perlin noise at lacunarity times the last one's frequency,
// counting roughness times as much. Its value, between 0 and 1 when normalized, picks the
// fog's thickness from a curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseMedium {
    pub transform: [[f32; 4]; 3],
    pub drift: [f32; 3],
    pub detail: f32,
    pub roughness: f32,
    pub lacunarity: f32,
    pub normalize: bool,
    // The density at evenly spaced noise values from 0 to 1, over Medium::density.
    pub thickness: [f32; NOISE_CURVE_POINTS],
}

impl Fog {
    pub fn medium(&self) -> Medium {
        let color = self.color.unwrap_or([0.5; 3]).map(|c| c.max(0.0));
        let passed = self.absorption_color.unwrap_or([0.0; 3]).map(|c| c.max(0.0).sqrt());
        let scattering = mean(color);
        let absorption =
            mean(std::array::from_fn(|i| (1.0 - color[i]).max(0.0) * (1.0 - passed[i]).max(0.0)));
        let density = self.density.unwrap_or(1.0).max(0.0);
        let noise = self.noise.as_ref().and_then(|noise| noise.medium(density));
        Medium {
            color: if scattering > 0.0 { color.map(|c| c / scattering) } else { [0.0; 3] },
            density: noise.map_or(density, |(mean, _)| mean),
            scattering,
            absorption,
            // At ±1 Bevy's phase function divides by zero.
            anisotropy: self.anisotropy.unwrap_or(0.0).clamp(-0.99, 0.99),
            noise: noise.map(|(_, noise)| noise),
        }
    }
}

impl FogNoise {
    // With the fog's average density, which light passing through it is dimmed by.
    fn medium(&self, scale: f32) -> Option<(f32, NoiseMedium)> {
        if self.density.len() < 2 {
            return None;
        }
        let last = (NOISE_CURVE_POINTS - 1) as f32;
        let curve: [f32; NOISE_CURVE_POINTS] =
            std::array::from_fn(|i| sample(&self.density, i as f32 / last).max(0.0) * scale);
        let mut noise = NoiseMedium {
            transform: self.transform,
            drift: self.drift.unwrap_or_default(),
            detail: self.detail.unwrap_or(2.0).clamp(0.0, MAX_NOISE_DETAIL),
            roughness: self.roughness.unwrap_or(0.5).clamp(0.0, 1.0),
            lacunarity: self.lacunarity.unwrap_or(2.0).max(0.0),
            normalize: self.normalize.unwrap_or(true),
            thickness: curve,
        };
        let mean = noise.mean_of(&curve);
        noise.thickness =
            if mean > 0.0 { curve.map(|d| d / mean) } else { [0.0; NOISE_CURVE_POINTS] };
        Some((mean, noise))
    }
}

impl NoiseMedium {
    // A curve's average over the noise's values, taking them as spread like a bell curve:
    // octaves of noise add up to one, so the more of them, the more so.
    fn mean_of(&self, curve: &[f32; NOISE_CURVE_POINTS]) -> f32 {
        let whole = self.detail.floor();
        let spread = |octaves: f32| {
            let amplitudes = (0..octaves as u32).map(|i| self.roughness.powi(i as i32));
            let (sum, squares) = amplitudes.fold((0.0, 0.0), |(s, q), a| (s + a, q + a * a));
            match self.normalize {
                true if sum > 0.0 => 0.5 * NOISE_SPREAD * squares.sqrt() / sum,
                true => 0.0,
                false => NOISE_SPREAD * squares.sqrt(),
            }
        };
        let rest = self.detail - whole;
        let spread = spread(whole + 1.0) * (1.0 - rest) + spread(whole + 2.0) * rest;
        let center = if self.normalize { 0.5 } else { 0.0 };
        if spread <= 0.0 {
            return sample(curve, center);
        }

        const POINTS: i32 = 128;
        let (mut total, mut weights) = (0.0, 0.0);
        for i in 0..=POINTS {
            let z = 8.0 * i as f32 / POINTS as f32 - 4.0;
            let weight = (-0.5 * z * z).exp();
            total += weight * sample(curve, center + z * spread);
            weights += weight;
        }
        total / weights
    }
}

// A curve of evenly spaced points from 0 to 1, read in between and held at its ends.
fn sample(curve: &[f32], at: f32) -> f32 {
    let last = curve.len() - 1;
    let t = at.clamp(0.0, 1.0) * last as f32;
    let i = (t as usize).min(last.saturating_sub(1));
    let next = (i + 1).min(last);
    curve[i] + (curve[next] - curve[i]) * (t - i as f32)
}

fn mean(values: [f32; 3]) -> f32 {
    values.iter().sum::<f32>() / 3.0
}
