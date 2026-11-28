//! A Blender mesh as glTF primitives, as the glTF exporter's PrimitiveCreator makes them.
//!
//! glTF keeps everything per vertex, Blender normals, UVs and tangents per corner. So every
//! corner becomes a "dot" holding its vertex and its corner's values, and each primitive (one
//! per material slot) keeps one glTF vertex per distinct dot.

use std::collections::HashMap;

use glam::DVec3;
use glam::Mat3;
use glam::Mat4;
use glam::Vec2;
use glam::Vec3;

/// A mesh as the add-on reads it out of Blender: per-vertex values by vertex, per-corner ones by
/// corner, in Blender's axes.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub name: String,
    /// The Basis, or the vertices themselves without shape keys.
    pub positions: Vec<Vec3>,
    pub corner_verts: Vec<u32>,
    /// The Basis's corner normals with shape keys.
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec<Vec2>>,
    /// Corner tangents and bitangent signs, when the mesh has them.
    pub tangents: Option<(Vec<Vec3>, Vec<f32>)>,
    /// Triangles as corners, and each one's material slot.
    pub triangles: Vec<[u32; 3]>,
    pub triangle_slots: Vec<u32>,
    pub shape_keys: Vec<ShapeKey>,
    pub skin: Option<SkinData>,
}

#[derive(Debug, Clone, Default)]
pub struct ShapeKey {
    pub name: String,
    pub weight: f32,
    pub positions: Vec<Vec3>,
    /// Corner normals, when shape key normals are exported.
    pub normals: Option<Vec<Vec3>>,
}

#[derive(Debug, Clone, Default)]
pub struct SkinData {
    /// Each vertex's (joint, weight) pairs, the weights above Blender's 0.0001 only.
    pub influences: Vec<Vec<(u16, f32)>>,
    pub object_world: Mat4,
    /// What turns the normals and tangents into the armature's space, made as the exporter
    /// makes them in Blender's float32 math, with `apply` the armature's inverse world matrix
    /// times the object's: `armature_world.to_3x3() @ apply.to_3x3().inverted_safe()
    /// .transposed()`, and `apply.to_quaternion().to_matrix()`. Rebuilt in other math, they
    /// come out a hair apart, and a value on a rounding edge then rounds the other way.
    pub normal_matrix: Mat3,
    pub tangent_matrix: Mat3,
    /// Whether the exporter's NumPy math on the moved values runs in float32, as it does
    /// where NumPy reads a Blender matrix as float32 (Blender 5.3 does), not float64 (4.2).
    pub float32: bool,
}

/// One material slot's triangles, as glTF wants them, in glTF's axes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Primitive {
    pub slot: u32,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<Vec<[f32; 2]>>,
    pub tangents: Vec<[f32; 4]>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub targets: Vec<Target>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Target {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

// The exporter rounds corner normals and tangents to this many decimals before telling corners
// apart, so ones that differ only by float noise share a vertex.
const DECIMALS: f64 = 1e4;

impl MeshData {
    pub fn primitives(&self) -> Vec<Primitive> {
        let skin = self.skin.as_ref();

        // A skinned mesh is moved by a matrix, and from there on its values take the matrix's
        // precision in the exporter's NumPy until written. Held as float64 either way, as a
        // float32 value converts exactly.
        let float32 = skin.is_some_and(|s| s.float32);
        let object = skin.map_or(Mat4::IDENTITY, |s| s.object_world);
        let placed = |points: &[Vec3]| -> Vec<DVec3> {
            match float32 {
                true => points.iter().map(|p| object.transform_point3(*p).as_dvec3()).collect(),
                false => {
                    let object = object.as_dmat4();
                    points.iter().map(|p| object.transform_point3(p.as_dvec3())).collect()
                }
            }
        };
        let less = |a: DVec3, b: DVec3| match float32 {
            true => a.as_vec3() - b.as_vec3(),
            false => (a - b).as_vec3(),
        };
        let base = placed(&self.positions);
        let key_positions: Vec<Vec<[f32; 3]>> = self
            .shape_keys
            .iter()
            .map(|key| {
                let moved = placed(&key.positions);
                moved.iter().zip(&base).map(|(k, b)| y_up(finite(less(*k, *b)))).collect()
            })
            .collect();
        let positions: Vec<[f32; 3]> = base.iter().map(|p| y_up(p.as_vec3())).collect();

        let normal_matrix = skin.map(|s| s.normal_matrix);
        let turned = |n: Vec3| match normal_matrix {
            Some(m) if float32 => normalized(m * n).as_dvec3(),
            Some(m) => normalized64(m.as_dmat3() * n.as_dvec3()),
            None => n.as_dvec3(),
        };
        let up = |n: DVec3| if n == DVec3::ZERO { DVec3::Z } else { n };
        let normals: Vec<DVec3> =
            self.normals.iter().map(|n| up(turned(normalized(round(*n))))).collect();
        let key_normals: Vec<Vec<[f32; 3]>> = self
            .shape_keys
            .iter()
            .filter_map(|k| k.normals.as_deref())
            .map(|key| {
                // Rounded, not normalized, before the turn: as the exporter does.
                key.iter()
                    .zip(&normals)
                    .map(|(n, b)| y_up(less(up(turned(round(*n))), *b)))
                    .collect()
            })
            .collect();
        let normals: Vec<[f32; 3]> = normals.iter().map(|n| y_up(n.as_vec3())).collect();

        let tangent_matrix = skin.map(|s| s.tangent_matrix);
        let tangents: Option<Vec<[f32; 4]>> = self.tangents.as_ref().map(|(tangents, signs)| {
            let flip = tangent_matrix.is_some_and(|m| m.determinant() < 0.0);
            tangents
                .iter()
                .zip(signs)
                .map(|(t, &sign)| {
                    let mut t = round(*t);
                    if let Some(m) = tangent_matrix {
                        t = match float32 {
                            true => round(normalized(m * t)),
                            false => round64(normalized64(m.as_dmat3() * t.as_dvec3())).as_vec3(),
                        };
                    }
                    let [x, y, z] = y_up(t);
                    [x, y, z, if flip { -sign } else { sign }]
                })
                .collect()
        });
        let uvs: Vec<Vec<[f32; 2]>> = self
            .uvs
            .iter()
            .map(|layer| layer.iter().map(|uv| [uv.x, 1.0 - uv.y]).collect())
            .collect();

        let skin = self
            .skin
            .as_ref()
            .map(|s| s.influences.iter().map(|i| strongest_four(i)).collect::<Vec<_>>());

        // Each corner's dot as words, side by side: its vertex, then the bits of every value
        // glTF keeps per vertex that Blender keeps per corner.
        let width = 1 + 3 + 2 * uvs.len() + 4 * tangents.is_some() as usize + 3 * key_normals.len();
        let mut dots: Vec<u32> = Vec::with_capacity(width * self.corner_verts.len());
        for (corner, &vertex) in self.corner_verts.iter().enumerate() {
            dots.push(vertex);
            dots.extend(normals[corner].map(f32::to_bits));
            for layer in &uvs {
                dots.extend(layer[corner].map(f32::to_bits));
            }
            if let Some(t) = &tangents {
                dots.extend(t[corner].map(f32::to_bits));
            }
            for key in &key_normals {
                dots.extend(key[corner].map(f32::to_bits));
            }
        }

        let mut slots: Vec<u32> = self.triangle_slots.clone();
        slots.sort_unstable();
        slots.dedup();
        let mut primitives = Vec::with_capacity(slots.len());
        for slot in slots {
            let mut primitive = Primitive { slot, ..Default::default() };
            primitive.uvs = vec![Vec::new(); uvs.len()];
            primitive.targets = vec![Target::default(); key_positions.len()];
            let mut seen: HashMap<&[u32], u32> = HashMap::new();
            for (triangle, _) in
                self.triangles.iter().zip(&self.triangle_slots).filter(|(_, s)| **s == slot)
            {
                for &corner in triangle {
                    let corner = corner as usize;
                    let vertex = self.corner_verts[corner] as usize;
                    let dot = &dots[corner * width..(corner + 1) * width];
                    let next = primitive.positions.len() as u32;
                    let index = *seen.entry(dot).or_insert_with(|| {
                        primitive.positions.push(positions[vertex]);
                        primitive.normals.push(normals[corner]);
                        for (out, layer) in primitive.uvs.iter_mut().zip(&uvs) {
                            out.push(layer[corner]);
                        }
                        if let Some(t) = &tangents {
                            primitive.tangents.push(t[corner]);
                        }
                        if let Some(skin) = &skin {
                            let (joints, weights) = skin[vertex];
                            primitive.joints.push(joints);
                            primitive.weights.push(weights);
                        }
                        for (k, target) in primitive.targets.iter_mut().enumerate() {
                            target.positions.push(key_positions[k][vertex]);
                            if let Some(normals) = key_normals.get(k) {
                                target.normals.push(normals[corner]);
                            }
                        }
                        next
                    });
                    primitive.indices.push(index);
                }
            }
            if !primitive.positions.is_empty() {
                primitives.push(primitive);
            }
        }
        primitives
    }

    /// Whether some vertex has no weight on any joint, which Blender's exporter gives a joint
    /// of its own that this writer doesn't make.
    pub fn needs_neutral_bone(&self) -> bool {
        self.skin.as_ref().is_some_and(|s| s.influences.iter().any(Vec::is_empty))
    }
}

// The four strongest, normalized to sum to 1, as the exporter keeps by default.
fn strongest_four(influences: &[(u16, f32)]) -> ([u16; 4], [f32; 4]) {
    let mut sorted = influences.to_vec();
    // Stable, as Python's sort, so equal weights keep the vertex groups' order.
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (mut joints, mut weights) = ([0u16; 4], [0f32; 4]);
    for (i, (joint, weight)) in sorted.into_iter().take(4).enumerate() {
        joints[i] = joint;
        weights[i] = weight;
    }
    let sum: f32 = weights.iter().sum();
    if sum > 0.0 {
        weights.iter_mut().for_each(|w| *w /= sum);
    }
    (joints, weights)
}

fn y_up(v: Vec3) -> [f32; 3] {
    [v.x, v.z, -v.y]
}

// NumPy rounds halves to even, in the array's own precision: float32 for what is read from
// Blender, float64 once a float64 matrix has turned it. The two disagree on values near a half.
fn round(v: Vec3) -> Vec3 {
    (v * DECIMALS as f32).map(f32::round_ties_even) / DECIMALS as f32
}

// As NumPy's normalize_vecs: the length summed in order, then each value divided by it, a
// zero vector left. glam multiplies by the length's reciprocal, which can differ in the last
// bit, and a tangent rounded after it then lands a step off.
fn normalized(v: Vec3) -> Vec3 {
    let length = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if length == 0.0 { v } else { Vec3::new(v.x / length, v.y / length, v.z / length) }
}

fn normalized64(v: DVec3) -> DVec3 {
    let length = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if length == 0.0 { v } else { DVec3::new(v.x / length, v.y / length, v.z / length) }
}

fn round64(v: DVec3) -> DVec3 {
    (v * DECIMALS).map(f64::round_ties_even) / DECIMALS
}

fn finite(v: Vec3) -> Vec3 {
    v.map(|x| if x.is_finite() { x } else { 0.0 })
}
