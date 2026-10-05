// The .cube LUT format (Adobe's, which Resolve, Photoshop and most grading apps export).
// Lines are keywords or one `r g b` entry each; `#` starts a comment. A 3D table lists its
// entries with red changing fastest, then green, then blue. A 1D table is three curves.
// Whatever the file holds, the result is a 3D table over inputs from 0 to 1.

pub const MAX_LUT_SIZE: usize = 129;
// A 1D table becomes a 3D one this fine.
const FROM_1D_SIZE: usize = 65;

#[derive(Debug, Clone, PartialEq)]
pub struct CubeLut {
    pub size: usize,
    // size³ entries, red index fastest.
    pub data: Vec<[f32; 3]>,
}

impl CubeLut {
    pub fn at(&self, r: usize, g: usize, b: usize) -> [f32; 3] {
        self.data[r + self.size * (g + self.size * b)]
    }

    // Trilinear, with each input clamped to [0, 1].
    pub fn sample(&self, input: [f32; 3]) -> [f32; 3] {
        let last = (self.size - 1) as f32;
        let mut low = [0; 3];
        let mut high = [0; 3];
        let mut t = [0.0; 3];
        for c in 0..3 {
            let p = input[c].clamp(0.0, 1.0) * last;
            low[c] = p.floor() as usize;
            high[c] = (low[c] + 1).min(self.size - 1);
            t[c] = p - low[c] as f32;
        }
        let corner = |r: bool, g: bool, b: bool| {
            self.at(
                if r { high[0] } else { low[0] },
                if g { high[1] } else { low[1] },
                if b { high[2] } else { low[2] },
            )
        };
        let lerp =
            |a: [f32; 3], b: [f32; 3], t: f32| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
        let g0 = lerp(
            lerp(corner(false, false, false), corner(true, false, false), t[0]),
            lerp(corner(false, true, false), corner(true, true, false), t[0]),
            t[1],
        );
        let g1 = lerp(
            lerp(corner(false, false, true), corner(true, false, true), t[0]),
            lerp(corner(false, true, true), corner(true, true, true), t[0]),
            t[1],
        );
        lerp(g0, g1, t[2])
    }
}

pub fn parse_cube(text: &str) -> Result<CubeLut, String> {
    let mut size_3d = None;
    let mut size_1d = None;
    let mut domain = ([0.0f32; 3], [1.0f32; 3]);
    let mut entries: Vec<[f32; 3]> = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else {
            continue;
        };
        let at = || format!("line {}", index + 1);
        if first.parse::<f32>().is_ok() {
            let values: Vec<f32> = line
                .split_whitespace()
                .map(|w| w.parse::<f32>().map_err(|_| format!("{}: {w:?} is not a number", at())))
                .collect::<Result<_, _>>()?;
            let [r, g, b] = values[..] else {
                return Err(format!("{}: an entry needs exactly three numbers", at()));
            };
            entries.push([r, g, b]);
            continue;
        }
        let rest: Vec<&str> = words.collect();
        let numbers = || -> Result<Vec<f32>, String> {
            rest.iter()
                .map(|w| w.parse::<f32>().map_err(|_| format!("{}: {w:?} is not a number", at())))
                .collect()
        };
        match first {
            "LUT_3D_SIZE" | "LUT_1D_SIZE" => {
                let size = rest
                    .first()
                    .and_then(|w| w.parse::<usize>().ok())
                    .ok_or_else(|| format!("{}: {first} needs a whole number", at()))?;
                if first == "LUT_3D_SIZE" { size_3d = Some(size) } else { size_1d = Some(size) }
            }
            "DOMAIN_MIN" | "DOMAIN_MAX" => {
                let [r, g, b] = numbers()?[..] else {
                    return Err(format!("{}: {first} needs three numbers", at()));
                };
                if first == "DOMAIN_MIN" { domain.0 = [r, g, b] } else { domain.1 = [r, g, b] }
            }
            // Resolve's older way to say the same for all three channels.
            "LUT_3D_INPUT_RANGE" | "LUT_1D_INPUT_RANGE" => {
                let [min, max] = numbers()?[..] else {
                    return Err(format!("{}: {first} needs two numbers", at()));
                };
                domain = ([min; 3], [max; 3]);
            }
            // TITLE and keywords of other apps change nothing about the table.
            _ => {}
        }
    }

    let (lut, domain_size) = match (size_3d, size_1d) {
        (Some(_), Some(_)) => return Err("holds both a 1D and a 3D table; keep one".to_string()),
        (None, None) => return Err("has no LUT_3D_SIZE or LUT_1D_SIZE line".to_string()),
        (Some(size), None) => {
            if !(2..=MAX_LUT_SIZE).contains(&size) {
                return Err(format!(
                    "LUT_3D_SIZE {size} is outside 2 to {MAX_LUT_SIZE}; export at 33 or 65"
                ));
            }
            check_count(entries.len(), size.pow(3))?;
            (CubeLut { size, data: entries }, size)
        }
        (None, Some(size)) => {
            if size < 2 {
                return Err(format!("LUT_1D_SIZE {size} needs to be at least 2"));
            }
            check_count(entries.len(), size)?;
            (from_curves(&entries), FROM_1D_SIZE)
        }
    };

    let (min, max) = domain;
    if (0..3).any(|c| max[c] <= min[c]) {
        return Err("DOMAIN_MAX must be above DOMAIN_MIN in every channel".to_string());
    }
    if min == [0.0; 3] && max == [1.0; 3] {
        return Ok(lut);
    }
    // Resampled so the app can always look it up over 0 to 1.
    let last = (domain_size - 1) as f32;
    let mut data = Vec::with_capacity(domain_size.pow(3));
    for b in 0..domain_size {
        for g in 0..domain_size {
            for r in 0..domain_size {
                let input = [r, g, b].map(|i| i as f32 / last);
                data.push(
                    lut.sample(std::array::from_fn(|c| (input[c] - min[c]) / (max[c] - min[c]))),
                );
            }
        }
    }
    Ok(CubeLut { size: domain_size, data })
}

fn check_count(found: usize, wanted: usize) -> Result<(), String> {
    if found == wanted {
        Ok(())
    } else {
        Err(format!("has {found} entries where its size needs {wanted}"))
    }
}

// Each channel through its own curve, sampled onto a 3D grid.
fn from_curves(curves: &[[f32; 3]]) -> CubeLut {
    let last = (curves.len() - 1) as f32;
    let curve = |channel: usize, x: f32| {
        let p = x * last;
        let low = p.floor() as usize;
        let high = (low + 1).min(curves.len() - 1);
        let t = p - low as f32;
        curves[low][channel] + (curves[high][channel] - curves[low][channel]) * t
    };
    let size = FROM_1D_SIZE;
    let step = 1.0 / (size - 1) as f32;
    let mut data = Vec::with_capacity(size.pow(3));
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                data.push([
                    curve(0, r as f32 * step),
                    curve(1, g as f32 * step),
                    curve(2, b as f32 * step),
                ]);
            }
        }
    }
    CubeLut { size, data }
}
