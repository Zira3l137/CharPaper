use serde::Deserialize;

/// A driver's curve, as the exporter sampled it with Blender's own evaluation, handles and
/// modifiers included, between its first and last key. Beyond them it runs on as Blender
/// extrapolates it: flat, or along the slope it ends with.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Curve {
    pub from: f32,
    pub to: f32,
    /// Evenly spaced from `from` to `to`; one value when they are the same.
    pub values: Vec<f32>,
    pub slope_before: f32,
    pub slope_after: f32,
}

impl Curve {
    pub fn sample(&self, x: f32) -> f32 {
        let (Some(&first), Some(&last)) = (self.values.first(), self.values.last()) else {
            return x;
        };
        if x <= self.from {
            return first + (x - self.from) * self.slope_before;
        }
        if x >= self.to || self.values.len() < 2 {
            return last + (x - self.to) * self.slope_after;
        }
        let at = (x - self.from) / (self.to - self.from) * (self.values.len() - 1) as f32;
        let index = (at as usize).min(self.values.len() - 2);
        let t = at - index as f32;
        self.values[index] + (self.values[index + 1] - self.values[index]) * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> Curve {
        Curve {
            from: 0.0,
            to: 2.0,
            values: vec![0.0, 1.0, 4.0],
            slope_before: 0.0,
            slope_after: 2.0,
        }
    }

    #[test]
    fn between_samples() {
        assert!((curve().sample(0.5) - 0.5).abs() < 1e-6);
        assert!((curve().sample(1.5) - 2.5).abs() < 1e-6);
    }

    #[test]
    fn beyond_the_ends() {
        assert_eq!(curve().sample(-3.0), 0.0);
        assert!((curve().sample(3.0) - 6.0).abs() < 1e-6);
    }

    #[test]
    fn a_single_key() {
        let one =
            Curve { from: 1.0, to: 1.0, values: vec![0.5], slope_before: 0.0, slope_after: 0.0 };
        assert_eq!(one.sample(-5.0), 0.5);
        assert_eq!(one.sample(5.0), 0.5);
    }
}
