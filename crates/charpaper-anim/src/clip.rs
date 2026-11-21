use glam::Quat;
use glam::Vec3;

use crate::track::Track;

/// One node's keyed channels, by the node's name: a bone, or an object. A channel left out is
/// not keyed, so it shows what the slots below give.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeTracks {
    pub name: String,
    pub translation: Option<Track<Vec3>>,
    pub rotation: Option<Track<Quat>>,
    pub scale: Option<Track<Vec3>>,
}

/// A light's strength and linear color as multiples of those it was loaded with.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LightTracks {
    pub strength: Option<Track<f32>>,
    pub color: Option<Track<Vec3>>,
}

/// One action, as the exporter wrote it: only what the action keys is here.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Clip {
    /// How long it plays: its frames at its frame rate, the last one shown for a frame before
    /// it ends or starts over, as Blender plays it.
    pub duration: f32,
    pub nodes: Vec<NodeTracks>,
    pub shape_keys: Vec<(String, Track<f32>)>,
    pub light: LightTracks,
}
