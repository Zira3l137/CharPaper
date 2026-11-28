use glam::Quat;
use glam::Vec3;
use gltf::json;
use gltf::json::accessor::Type;
use gltf::json::animation::Interpolation;
use gltf::json::animation::Property;
use gltf::json::validation::Checked::Valid;

use crate::build::Glb;

/// A node's transform relative to its parent, in glTF's axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trs {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Trs {
    pub const IDENTITY: Self =
        Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };
}

/// How the exporter treats one of a node's channels: translation, rotation or scale.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Channel {
    /// Keyed in the clip's action.
    #[default]
    Keyed,
    /// Keyed, its first key Constant: the value jumps from sample to sample.
    Stepped,
    /// Not keyed, moved by something else: a parent, a constraint.
    Baked,
}

/// A node a clip moves, sampled at every step from the clip's first frame.
#[derive(Debug, Clone, Default)]
pub struct ClipNode {
    pub name: String,
    pub samples: Vec<Trs>,
    pub channels: [Channel; 3],
}

/// A clip's .glb: the nodes it moves and one animation moving them, all the app reads of it.
/// The .json beside it names the nodes; what isn't named there needn't be here.
#[derive(Debug, Clone)]
pub struct Clip {
    pub name: String,
    /// Seconds from one sample to the next: Blender's frame step over its frame rate.
    pub step_seconds: f32,
    /// As Blender's *Optimize keys*: a channel that holds still keeps its first and last keys.
    pub optimize: bool,
    /// Leaves out a baked channel that holds still, as Blender does unless told to keep it.
    pub drop_still_baked: bool,
    pub nodes: Vec<ClipNode>,
}

// Blender's fcurve_is_constant.
const STILL: f32 = 1e-4;

impl Clip {
    /// The clip's .glb, of its nodes and nothing else.
    pub fn to_glb(&self) -> Vec<u8> {
        let mut glb = Glb::new();
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .map(|n| glb.root.push(json::Node { name: Some(n.name.clone()), ..Default::default() }))
            .collect();
        let animation = self.animation(&mut glb, &nodes);
        let scene = glb.root.push(json::Scene {
            extensions: None,
            extras: Default::default(),
            name: None,
            nodes,
        });
        glb.root.scene = Some(scene);
        if let Some(animation) = animation {
            glb.root.push(animation);
        }
        glb.into_vec()
    }

    /// The animation moving `nodes`, one for each of the clip's nodes, or None when nothing
    /// in it moves.
    pub(crate) fn animation(
        &self,
        glb: &mut Glb,
        nodes: &[json::Index<json::Node>],
    ) -> Option<json::Animation> {
        let samples = self.nodes.iter().map(|n| n.samples.len()).max().unwrap_or(0);
        let all_times: Vec<f32> = (0..samples).map(|i| i as f32 * self.step_seconds).collect();
        let times = glb.floats(&all_times, Type::Scalar, true);
        let ends = match samples {
            0 | 1 => times,
            _ => glb.floats(&[0.0, all_times[samples - 1]], Type::Scalar, true),
        };

        let mut animation = json::Animation {
            extensions: None,
            extras: Default::default(),
            channels: Vec::new(),
            name: Some(self.name.clone()),
            samplers: Vec::new(),
        };
        for (node, &index) in self.nodes.iter().zip(nodes) {
            let translations: Vec<[f32; 3]> =
                node.samples.iter().map(|s| s.translation.into()).collect();
            let rotations: Vec<[f32; 4]> = continuous(node.samples.iter().map(|s| s.rotation));
            let scales: Vec<[f32; 3]> = node.samples.iter().map(|s| s.scale.into()).collect();
            let channels = [
                (Property::Translation, Type::Vec3, flatten(&translations)),
                (Property::Rotation, Type::Vec4, flatten(&rotations)),
                (Property::Scale, Type::Vec3, flatten(&scales)),
            ];
            for ((property, kind, values), how) in channels.into_iter().zip(node.channels) {
                let width = kind.multiplicity();
                let still = holds_still(&values, width);
                let (input, values, interpolation) = match how {
                    Channel::Baked if self.drop_still_baked && still => continue,
                    _ if self.optimize && samples > 2 && still => {
                        let first_last =
                            [&values[..width], &values[values.len() - width..]].concat();
                        (ends, first_last, Interpolation::Step)
                    }
                    Channel::Stepped => (times, values, Interpolation::Step),
                    _ => (times, values, Interpolation::Linear),
                };
                let output = glb.floats(&values, kind, false);
                let sampler = json::Index::push(
                    &mut animation.samplers,
                    json::animation::Sampler {
                        extensions: None,
                        extras: Default::default(),
                        input,
                        interpolation: Valid(interpolation),
                        output,
                    },
                );
                animation.channels.push(json::animation::Channel {
                    sampler,
                    target: json::animation::Target {
                        extensions: None,
                        extras: Default::default(),
                        node: index,
                        path: Valid(property),
                    },
                    extensions: None,
                    extras: Default::default(),
                });
            }
        }
        (!animation.channels.is_empty()).then_some(animation)
    }
}

fn flatten<const N: usize>(values: &[[f32; N]]) -> Vec<f32> {
    values.iter().flatten().copied().collect()
}

fn holds_still(values: &[f32], width: usize) -> bool {
    (0..width).all(|i| {
        let column = values.iter().skip(i).step_by(width);
        let (min, max) =
            column.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        max - min < STILL
    })
}

// q and -q are the same turn, but a linear blend from one to the other passes through
// nothing, so each key takes the sign nearer the one before.
fn continuous(rotations: impl Iterator<Item = Quat>) -> Vec<[f32; 4]> {
    let mut previous: Option<Quat> = None;
    rotations
        .map(|q| {
            let q = match previous {
                Some(p) if p.dot(q) < 0.0 => -q,
                _ => q,
            };
            previous = Some(q);
            q.to_array()
        })
        .collect()
}
