//! The body's own surface around a bone, for chains to keep off when capsules are too rough.
//!
//! Read from a skin's glTF file: each triangle goes to the bones its vertices are mostly
//! weighted to, in that bone's space, and moves rigidly with it. That bends a little wrong
//! right at a joint, where skinning blends two bones, and is far cheaper than skinning the
//! mesh on the CPU every frame.

use std::collections::BTreeMap;
use std::path::Path;

use glam::Mat4;
use glam::Vec3;

/// Fewer vertices than this around a bone say nothing about the body there.
const FEWEST_VERTICES: usize = 8;

/// Triangles in a bone's space, which a point can be pushed out of.
#[derive(Debug, Clone, Default)]
pub struct Surface {
    // In the order the tree's leaves hold them.
    triangles: Vec<Triangle>,
    // A tree of boxes around them, the root first, so a point only looks at the ones near it.
    nodes: Vec<Node>,
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    min: Vec3,
    max: Vec3,
    // A leaf's triangles, or for a branch the index of its first child, the second right after
    // the first's subtree.
    first: usize,
    count: usize,
}

const LEAF: usize = 4;

impl Surface {
    /// Triangles wound counter-clockwise seen from outside, as glTF has them. Ones with no area
    /// are left out.
    pub fn new(triangles: impl IntoIterator<Item = [Vec3; 3]>) -> Self {
        let mut triangles: Vec<Triangle> = triangles
            .into_iter()
            .filter_map(|[a, b, c]| {
                let normal = (b - a).cross(c - a).try_normalize()?;
                Some(Triangle { a, b, c, normal })
            })
            .collect();
        let mut nodes = Vec::new();
        if !triangles.is_empty() {
            build(&mut triangles, 0, &mut nodes);
        }
        Self { triangles, nodes }
    }

    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// `point` moved to `margin` off the surface, all in its own space. Inside or outside goes
    /// by the nearest triangle's side, so a point that went through comes back out the way it
    /// went in only while it is nearer that side than the other.
    pub fn push(&self, point: Vec3, margin: f32) -> Vec3 {
        let Some(root) = self.nodes.first() else {
            return point;
        };
        if point.cmplt(root.min - margin).any() || point.cmpgt(root.max + margin).any() {
            return point;
        }
        let mut best: Option<Nearest> = None;
        let mut stack = vec![0];
        while let Some(at) = stack.pop() {
            let node = &self.nodes[at];
            if best.is_some_and(|b| box_distance(node, point) > b.distance) {
                continue;
            }
            if node.count > 0 {
                for triangle in &self.triangles[node.first..node.first + node.count] {
                    nearer(&mut best, triangle, point);
                }
                continue;
            }
            let (left, right) = (at + 1, node.first);
            let (near, far) = match box_distance(&self.nodes[left], point)
                <= box_distance(&self.nodes[right], point)
            {
                true => (left, right),
                false => (right, left),
            };
            stack.push(far);
            stack.push(near);
        }
        let Some(Nearest { distance, closest, normal }) = best else {
            return point;
        };
        let away = point - closest;
        let inside = away.dot(normal) < 0.0;
        if !inside && distance >= margin * margin {
            return point;
        }
        let out = if inside { normal } else { away.try_normalize().unwrap_or(normal) };
        closest + out * margin
    }
}

#[derive(Debug, Clone, Copy)]
struct Nearest {
    // Squared.
    distance: f32,
    closest: Vec3,
    normal: Vec3,
}

// Where two triangles meet at an edge both are as near; the one the point is more squarely in
// front of or behind says its side better.
fn nearer(best: &mut Option<Nearest>, triangle: &Triangle, point: Vec3) {
    let closest = closest_on(triangle, point);
    let distance = point.distance_squared(closest);
    let better = match *best {
        None => true,
        Some(b) => {
            let tie = (distance - b.distance).abs() <= 1e-9 * (1.0 + b.distance);
            (distance < b.distance && !tie)
                || (tie
                    && (point - closest).dot(triangle.normal).abs()
                        > (point - b.closest).dot(b.normal).abs())
        }
    };
    if better {
        *best = Some(Nearest { distance, closest, normal: triangle.normal });
    }
}

// Squared.
fn box_distance(node: &Node, point: Vec3) -> f32 {
    (point.clamp(node.min, node.max) - point).length_squared()
}

// Splits the triangles in half along the longest side of the box around their middles, until
// a few are left in each leaf.
fn build(triangles: &mut [Triangle], offset: usize, nodes: &mut Vec<Node>) {
    let corners = triangles.iter().flat_map(|t| [t.a, t.b, t.c]);
    let min = corners.clone().fold(Vec3::INFINITY, Vec3::min);
    let max = corners.fold(Vec3::NEG_INFINITY, Vec3::max);
    let at = nodes.len();
    nodes.push(Node { min, max, first: offset, count: triangles.len() });
    if triangles.len() <= LEAF {
        return;
    }
    let middle = |t: &Triangle| t.a + t.b + t.c;
    let spread = triangles
        .iter()
        .map(middle)
        .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), m| (lo.min(m), hi.max(m)));
    let axis = (spread.1 - spread.0).max_position();
    let half = triangles.len() / 2;
    triangles.select_nth_unstable_by(half, |a, b| middle(a)[axis].total_cmp(&middle(b)[axis]));
    let (left, right) = triangles.split_at_mut(half);
    build(left, offset, nodes);
    nodes[at].count = 0;
    nodes[at].first = nodes.len();
    build(right, offset + half, nodes);
}

// From Ericson's Real-Time Collision Detection, 5.1.5.
fn closest_on(t: &Triangle, p: Vec3) -> Vec3 {
    let (ab, ac, ap) = (t.b - t.a, t.c - t.a, p - t.a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return t.a;
    }
    let bp = p - t.b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return t.b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return t.a + ab * (d1 / (d1 - d3));
    }
    let cp = p - t.c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return t.c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return t.a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return t.b + (t.c - t.b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1.0 / (va + vb + vc);
    t.a + ab * (vb * denominator) + ac * (vc * denominator)
}

/// The part of one mesh object around one bone.
#[derive(Debug, Clone)]
pub struct Part {
    /// The object's name, as the skin's objects are hidden by.
    pub object: String,
    pub bone: String,
    /// How far the vertices mostly weighted to the bone are from it, the median of them: the
    /// body is the thinnest object around a bone, not a coat over it.
    pub thickness: f32,
    pub surface: Surface,
}

#[derive(Debug, thiserror::Error)]
pub enum SurfaceError {
    #[error("cannot read {0}: {1}")]
    Read(String, std::io::Error),
    #[error("{0}")]
    Gltf(#[from] gltf::Error),
}

/// Every object's part around each of `bones`, given as their names and lengths, from a skin's
/// .glb or .gltf.
pub fn read_parts(path: &Path, bones: &[(String, f32)]) -> Result<Vec<Part>, SurfaceError> {
    let read = |path: &Path| {
        std::fs::read(path).map_err(|e| SurfaceError::Read(path.display().to_string(), e))
    };
    let mut gltf = gltf::Gltf::from_slice(&read(path)?)?;
    let blob = gltf.blob.take();
    let base = path.parent().unwrap_or(Path::new(""));
    let external: Vec<Option<Vec<u8>>> = gltf
        .buffers()
        .map(|buffer| match buffer.source() {
            gltf::buffer::Source::Uri(uri) if !uri.starts_with("data:") => {
                read(&base.join(uri)).ok()
            }
            _ => None,
        })
        .collect();
    let data = |buffer: gltf::Buffer| match buffer.source() {
        gltf::buffer::Source::Bin => blob.as_deref(),
        gltf::buffer::Source::Uri(_) => external[buffer.index()].as_deref(),
    };

    let mut parts = Vec::new();
    for node in gltf.nodes() {
        let (Some(mesh), Some(skin)) = (node.mesh(), node.skin()) else {
            continue;
        };
        let object = node.name().unwrap_or_default().to_string();
        // Each joint of the skin: the bone it is, if it is one of `bones`, and how to get from
        // the mesh into its space.
        let wanted: Vec<Option<usize>> = skin
            .joints()
            .map(|joint| bones.iter().position(|(name, _)| Some(name.as_str()) == joint.name()))
            .collect();
        let inverse: Vec<Mat4> = match skin.reader(data).read_inverse_bind_matrices() {
            Some(matrices) => matrices.map(|m| Mat4::from_cols_array_2d(&m)).collect(),
            None => vec![Mat4::IDENTITY; wanted.len()],
        };
        let mut found: BTreeMap<usize, (Vec<f32>, Vec<[Vec3; 3]>)> = BTreeMap::new();
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                continue;
            }
            let reader = primitive.reader(data);
            let (Some(positions), Some(joints), Some(weights)) =
                (reader.read_positions(), reader.read_joints(0), reader.read_weights(0))
            else {
                continue;
            };
            let positions: Vec<Vec3> = positions.map(Vec3::from).collect();
            let weights: Vec<[f32; 4]> = weights.into_f32().collect();
            // The joint each vertex is mostly weighted to.
            let mostly: Vec<usize> = joints
                .into_u16()
                .zip(&weights)
                .map(|(joints, weights)| {
                    let at = (0..4).max_by(|&a, &b| weights[a].total_cmp(&weights[b])).unwrap_or(0);
                    joints[at] as usize
                })
                .collect();
            let indices: Vec<usize> = match reader.read_indices() {
                Some(indices) => indices.into_u32().map(|i| i as usize).collect(),
                None => (0..positions.len()).collect(),
            };
            let local = |joint: usize, vertex: usize| {
                inverse
                    .get(joint)
                    .copied()
                    .unwrap_or(Mat4::IDENTITY)
                    .transform_point3(positions[vertex])
            };
            for (vertex, &joint) in mostly.iter().enumerate() {
                let Some(Some(bone)) = wanted.get(joint) else {
                    continue;
                };
                let length = bones[*bone].1;
                let point = local(joint, vertex);
                let on_bone = Vec3::Y * point.y.clamp(0.0, length);
                found.entry(*bone).or_default().0.push(point.distance(on_bone));
            }
            for corners in indices.as_chunks::<3>().0 {
                if corners.iter().any(|&v| v >= positions.len() || v >= mostly.len()) {
                    continue;
                }
                let mut joints: Vec<usize> = corners.iter().map(|&v| mostly[v]).collect();
                joints.sort_unstable();
                joints.dedup();
                for joint in joints {
                    let Some(Some(bone)) = wanted.get(joint) else {
                        continue;
                    };
                    let triangle = [0, 1, 2].map(|i| local(joint, corners[i]));
                    found.entry(*bone).or_default().1.push(triangle);
                }
            }
        }
        for (bone, (mut distances, triangles)) in found {
            if distances.len() < FEWEST_VERTICES {
                continue;
            }
            distances.sort_by(f32::total_cmp);
            parts.push(Part {
                object: object.clone(),
                bone: bones[bone].0.clone(),
                thickness: distances[distances.len() / 2],
                surface: Surface::new(triangles),
            });
        }
    }
    Ok(parts)
}

/// For each bone, which of `parts` is the body around it: the thinnest whose object isn't
/// hidden.
pub fn body(parts: &[Part], hidden: impl Fn(&str) -> bool) -> BTreeMap<String, usize> {
    let mut thinnest: BTreeMap<String, usize> = BTreeMap::new();
    for (at, part) in parts.iter().enumerate() {
        if hidden(&part.object) || part.surface.is_empty() {
            continue;
        }
        let kept = thinnest.entry(part.bone.clone()).or_insert(at);
        if part.thickness < parts[*kept].thickness {
            *kept = at;
        }
    }
    thinnest
}
