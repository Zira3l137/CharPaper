use crate::manifest::Fog;

// Bevy's fog in Blender's terms. Blender's Principled Volume works per color channel:
//   scattering = color × density
//   absorption = (1 − color) × (1 − √absorption_color) × density
// Bevy has one number for each, so the channels are averaged. Bevy tints scattered light by
// fog color × scattering, which makes the fog color Blender's color over its average.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Medium {
    pub color: [f32; 3],
    pub density: f32,
    pub scattering: f32,
    pub absorption: f32,
    pub anisotropy: f32,
}

impl Fog {
    pub fn medium(&self) -> Medium {
        let color = self.color.unwrap_or([0.5; 3]).map(|c| c.max(0.0));
        let passed = self.absorption_color.unwrap_or([0.0; 3]).map(|c| c.max(0.0).sqrt());
        let scattering = mean(color);
        let absorption =
            mean(std::array::from_fn(|i| (1.0 - color[i]).max(0.0) * (1.0 - passed[i]).max(0.0)));
        Medium {
            color: if scattering > 0.0 { color.map(|c| c / scattering) } else { [0.0; 3] },
            density: self.density.unwrap_or(1.0).max(0.0),
            scattering,
            absorption,
            // At ±1 Bevy's phase function divides by zero.
            anisotropy: self.anisotropy.unwrap_or(0.0).clamp(-0.99, 0.99),
        }
    }
}

fn mean(values: [f32; 3]) -> f32 {
    values.iter().sum::<f32>() / 3.0
}
