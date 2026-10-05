use crate::manifest::Grading;
use crate::manifest::GradingSection;

// Bevy balances white by moving the white point away from D65 in CIE 1931 xy: temperature
// along x alone (red against cyan) and tint along y. Warmth instead follows the curve of
// black-body colors, orange against blue, the way a photographer's white balance does.

const D65_KELVIN: f64 = 6504.0;
// How far warmth ±1 moves along the curve, in mireds (a million over kelvins): about the
// difference between daylight and a strong warming filter.
const MIREDS_PER_WARMTH: f64 = 100.0;
// How far tint ±1 moves y.
const TINT_Y: f64 = 0.05;

impl Grading {
    pub fn warmth(&self) -> f32 {
        self.warmth.unwrap_or(0.0).clamp(-1.0, 1.0)
    }

    pub fn tint(&self) -> f32 {
        self.tint.unwrap_or(0.0).clamp(-1.0, 1.0)
    }

    pub fn hue_deg(&self) -> f32 {
        self.hue_deg.unwrap_or(0.0)
    }

    pub fn saturation(&self) -> f32 {
        self.saturation.unwrap_or(1.0).max(0.0)
    }

    pub fn contrast(&self) -> f32 {
        self.contrast.unwrap_or(1.0).max(0.0)
    }

    pub fn midtones_range(&self) -> [f32; 2] {
        let [start, end] = self.midtones_range.unwrap_or([0.2, 0.7]);
        [start.clamp(0.0, 1.0), end.clamp(start.clamp(0.0, 1.0), 1.0)]
    }

    /// Warmth and tint as Bevy's `ColorGradingGlobal` temperature and tint.
    pub fn white_balance(&self) -> [f32; 2] {
        // A warmer picture comes from taking a bluer light for white.
        let mireds = 1e6 / D65_KELVIN - MIREDS_PER_WARMTH * self.warmth() as f64;
        let [x, y] = black_body_xy(1e6 / mireds);
        let [x0, y0] = black_body_xy(D65_KELVIN);
        [(x0 - x) as f32, (y - y0 + TINT_Y * self.tint() as f64) as f32]
    }
}

impl GradingSection {
    pub fn saturation(&self) -> f32 {
        self.saturation.unwrap_or(1.0).max(0.0)
    }

    pub fn contrast(&self) -> f32 {
        self.contrast.unwrap_or(1.0).max(0.0)
    }

    pub fn gamma(&self) -> f32 {
        self.gamma.unwrap_or(1.0).max(0.01)
    }

    pub fn gain(&self) -> f32 {
        self.gain.unwrap_or(1.0).max(0.0)
    }

    pub fn lift(&self) -> f32 {
        self.lift.unwrap_or(0.0)
    }
}

// Kim et al. (2002): the black-body color at `kelvin`, valid from 1667 K to 25000 K. Warmth
// stays between about 3900 K and 18600 K.
fn black_body_xy(kelvin: f64) -> [f64; 2] {
    let t = 1e3 / kelvin;
    let x = if kelvin <= 4000.0 {
        -0.2661239 * t.powi(3) - 0.2343589 * t.powi(2) + 0.8776956 * t + 0.179910
    } else {
        -3.0258469 * t.powi(3) + 2.1070379 * t.powi(2) + 0.2226347 * t + 0.240390
    };
    let y = if kelvin <= 2222.0 {
        -1.1063814 * x.powi(3) - 1.34811020 * x.powi(2) + 2.18555832 * x - 0.20219683
    } else if kelvin <= 4000.0 {
        -0.9549476 * x.powi(3) - 1.37418593 * x.powi(2) + 2.09137015 * x - 0.16748867
    } else {
        3.0817580 * x.powi(3) - 5.87338670 * x.powi(2) + 3.75112997 * x - 0.37001483
    };
    [x, y]
}
