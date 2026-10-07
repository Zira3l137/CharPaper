use std::fmt;

use gltf::Semantic;
use gltf::khr_lights_punctual::Kind;
use gltf::mesh::Mode;

use crate::cascades::sun_cascades;
use crate::document::read_document;
use crate::inspect::tree::Decomposed;
use crate::inspect::tree::Gltf;
use crate::inspect::tree::index;
use crate::layout::Environment;
use crate::layout::Suite;

// What an environment asks of the GPU every frame, as far as its files can tell: how often its
// lights draw the scene again for their shadows, what its fog has to light, and how much there
// is to draw. Where the camera looks decides the rest, which only `--benchmark` can measure.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cost {
    pub environment: String,
    pub lights: Lights,
    pub shadows: bool,
    // At most: the scene's diagonal stands in for how deep the camera can see into it.
    pub sun_cascades: usize,
    pub fog_boxes: usize,
    pub fog_lights: Lights,
    pub triangles: usize,
    pub objects: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lights {
    pub point: usize,
    pub spot: usize,
    pub sun: usize,
}

impl Cost {
    /// Six for each point light (a cube around it), one per spot light and one per cascade of
    /// each sun.
    pub fn shadow_passes(&self) -> usize {
        if !self.shadows {
            return 0;
        }
        self.lights.point * 6 + self.lights.spot + self.lights.sun * self.sun_cascades
    }
}

impl Lights {
    fn total(&self) -> usize {
        self.point + self.spot + self.sun
    }

    fn add(&mut self, kind: &Kind) {
        match kind {
            Kind::Directional => self.sun += 1,
            Kind::Point => self.point += 1,
            Kind::Spot { .. } => self.spot += 1,
        }
    }
}

impl fmt::Display for Lights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.total();
        if total == 0 {
            return write!(f, "no lights");
        }
        let parts: Vec<String> = [(self.point, "point"), (self.spot, "spot"), (self.sun, "sun")]
            .into_iter()
            .filter(|&(count, _)| count > 0)
            .map(|(count, kind)| format!("{count} {kind}"))
            .collect();
        let noun = if total == 1 { "light" } else { "lights" };
        write!(f, "{total} {noun} ({})", parts.join(", "))
    }
}

impl fmt::Display for Cost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.environment, self.lights)?;
        if self.lights.total() > 0 {
            match self.shadows {
                true => write!(f, ", up to {} shadow passes a frame", self.shadow_passes())?,
                false => write!(f, ", shadows off")?,
            }
        }
        if self.fog_boxes > 0 {
            let noun = if self.fog_boxes == 1 { "box" } else { "boxes" };
            write!(f, "\nfog: {} {noun}, lit by {}", self.fog_boxes, self.fog_lights)?;
        }
        if self.objects > 0 {
            write!(f, "\n{} triangles in {} objects", amount(self.triangles), self.objects)?;
        }
        Ok(())
    }
}

pub fn costs(suite: &Suite) -> Vec<Cost> {
    suite.environments.iter().map(|environment| cost(suite, environment)).collect()
}

fn cost(suite: &Suite, environment: &Environment) -> Cost {
    let settings = &environment.settings;
    let mut cost = Cost {
        environment: environment.name.clone(),
        shadows: settings.shadows.unwrap_or(true),
        sun_cascades: 1,
        ..Cost::default()
    };
    let Some(gltf) = environment
        .scene
        .as_ref()
        .and_then(|scene| read_document(&suite.absolute(scene)).ok())
        .map(index)
    else {
        return cost;
    };

    let mut bounds: Option<([f32; 3], [f32; 3])> = None;
    for node in gltf.doc.nodes().filter(|n| gltf.paths.contains_key(&n.index())) {
        let name = gltf.names[node.index()].as_str();
        if let Some(light) = node.light() {
            let kind = light.kind();
            cost.lights.add(&kind);
            let skipped = |n: &str| settings.no_volume_scatter.iter().any(|s| s == n);
            let scatters = !(skipped(name) || light.name().is_some_and(skipped));
            // A sun lights fog through its shadow map.
            if scatters && (cost.shadows || !matches!(kind, Kind::Directional)) {
                cost.fog_lights.add(&kind);
            }
        }
        let Some(mesh) = node.mesh() else {
            continue;
        };
        if settings.fog.contains_key(name) {
            cost.fog_boxes += 1;
            continue;
        }
        cost.objects += 1;
        let world = world(&gltf, node.index());
        for primitive in mesh.primitives().filter(|p| p.mode() == Mode::Triangles) {
            let positions = primitive.get(&Semantic::Positions);
            let corners = primitive.indices().or(positions.clone()).map_or(0, |a| a.count());
            cost.triangles += corners / 3;
            if let Some((min, max)) = positions.and_then(|a| Some((xyz(a.min()?)?, xyz(a.max()?)?)))
            {
                for corner in 0..8 {
                    let local =
                        [0, 1, 2].map(|i| if (corner >> i) & 1 == 0 { min[i] } else { max[i] });
                    let point = apply(&world, local);
                    let (low, high) = bounds.get_or_insert((point, point));
                    *low = [0, 1, 2].map(|i| low[i].min(point[i]));
                    *high = [0, 1, 2].map(|i| high[i].max(point[i]));
                }
            }
        }
    }

    if let Some((low, high)) = bounds {
        let diagonal = (0..3).map(|i| (high[i] - low[i]).powi(2)).sum::<f32>().sqrt();
        cost.sun_cascades = sun_cascades(diagonal);
    }
    cost
}

fn amount(count: usize) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => format!("{}k", count / 1_000),
        _ => format!("{:.1}M", count as f64 / 1_000_000.0),
    }
}

fn xyz(value: gltf::json::Value) -> Option<[f32; 3]> {
    let values = value.as_array()?;
    let at = |i: usize| values.get(i)?.as_f64().map(|v| v as f32);
    Some([at(0)?, at(1)?, at(2)?])
}

// Columns x, y and z, then the translation.
type Affine = [[f32; 3]; 4];

fn world(gltf: &Gltf, node: usize) -> Affine {
    let mut world = local(&gltf.rest[node]);
    for parent in gltf.ancestors(node) {
        world = compose(&local(&gltf.rest[parent]), &world);
    }
    world
}

fn local(&([tx, ty, tz], [x, y, z, w], [sx, sy, sz]): &Decomposed) -> Affine {
    let columns = [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + z * w), 2.0 * (x * z - y * w)],
        [2.0 * (x * y - z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + x * w)],
        [2.0 * (x * z + y * w), 2.0 * (y * z - x * w), 1.0 - 2.0 * (x * x + y * y)],
    ];
    let scaled = |column: [f32; 3], s: f32| column.map(|v| v * s);
    [scaled(columns[0], sx), scaled(columns[1], sy), scaled(columns[2], sz), [tx, ty, tz]]
}

fn apply(m: &Affine, [x, y, z]: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| m[0][i] * x + m[1][i] * y + m[2][i] * z + m[3][i])
}

fn compose(parent: &Affine, child: &Affine) -> Affine {
    let linear = |v: [f32; 3]| {
        [0, 1, 2].map(|i| parent[0][i] * v[0] + parent[1][i] * v[1] + parent[2][i] * v[2])
    };
    [linear(child[0]), linear(child[1]), linear(child[2]), apply(parent, child[3])]
}
