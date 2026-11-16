use crate::manifest::LightAnimation;

/// A light's strength and linear color at a moment of its animation, as multiples of those
/// it was loaded with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightSample {
    pub strength: f32,
    pub color: [f32; 3],
}

impl LightAnimation {
    /// Where the loop is `seconds` in, from its start. Missing or empty samples leave the
    /// light as loaded.
    pub fn at(&self, seconds: f32) -> LightSample {
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        LightSample {
            strength: self
                .strength
                .as_deref()
                .and_then(|s| looped(s, self.duration, seconds, lerp))
                .unwrap_or(1.0),
            color: self
                .color
                .as_deref()
                .and_then(|c| {
                    looped(c, self.duration, seconds, |a, b, t| {
                        std::array::from_fn(|i| lerp(a[i], b[i], t))
                    })
                })
                .unwrap_or([1.0; 3]),
        }
    }
}

// Samples evenly spaced over the loop, the last leading back into the first.
fn looped<T: Copy>(
    samples: &[T],
    duration: f32,
    seconds: f32,
    lerp: impl Fn(T, T, f32) -> T,
) -> Option<T> {
    let (&first, count) = (samples.first()?, samples.len());
    if count == 1 || duration.is_nan() || duration <= 0.0 || !seconds.is_finite() {
        return Some(first);
    }
    let at = (seconds / duration).rem_euclid(1.0) * count as f32;
    let i = (at as usize).min(count - 1);
    Some(lerp(samples[i], samples[(i + 1) % count], at - i as f32))
}
