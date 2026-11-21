use std::collections::BTreeMap;

use glam::Quat;
use glam::Vec3;
use gltf::animation::util::ReadOutputs;
use serde::Deserialize;

use crate::clip::Clip;
use crate::clip::LightTracks;
use crate::clip::NodeTracks;
use crate::track::Interpolation;
use crate::track::Sample;
use crate::track::Track;

/// The version of the `.json` this crate reads.
pub const FORMAT: u32 = 1;

/// The `.json` the exporter writes beside a clip's `.glb`, or on its own for a clip that keys
/// no bones or objects. Curves hold one value per frame, from the clip's first frame to its
/// last inclusive.
///
/// The `.glb` carries what glTF does well, node transforms in the app's axes. Shape keys come
/// from here, by name, because glTF animates a mesh's shape keys as one block and needs the
/// mesh in the file. `nodes` lists what the action keys, since the exporter bakes every bone.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClipJson {
    pub format: u32,
    pub fps: f32,
    pub frames: usize,
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub shape_keys: BTreeMap<String, Vec<f32>>,
    #[serde(default)]
    pub light: Option<JsonLight>,
}

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JsonLight {
    pub strength: Option<Vec<f32>>,
    pub color: Option<Vec<[f32; 3]>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClipError {
    #[error("unreadable .json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the .json is format {0}; this app reads format {FORMAT}")]
    Format(u32),
    #[error("fps must be above 0, not {0}")]
    Fps(f32),
    #[error("the .json has no frames")]
    NoFrames,
    #[error("{name} has {samples} samples for {frames} frames")]
    Samples { name: String, samples: usize, frames: usize },
    #[error("it keys bones or objects but has no .glb")]
    NoGlb,
    #[error("unreadable .glb: {0}")]
    Glb(#[from] gltf::Error),
    #[error("the .glb holds no animation")]
    NoAnimation,
    #[error("it keys {0}, which the .glb doesn't animate")]
    Missing(String),
    #[error("the .glb's keys for {0} are unreadable")]
    Unreadable(String),
}

impl ClipJson {
    pub fn read(json: &[u8]) -> Result<Self, ClipError> {
        Ok(serde_json::from_slice(json)?)
    }
}

impl Clip {
    pub fn load(json: &[u8], glb: Option<&[u8]>) -> Result<Self, ClipError> {
        let json = ClipJson::read(json)?;
        if json.format != FORMAT {
            return Err(ClipError::Format(json.format));
        }
        if !(json.fps.is_finite() && json.fps > 0.0) {
            return Err(ClipError::Fps(json.fps));
        }
        if json.frames == 0 {
            return Err(ClipError::NoFrames);
        }
        let mut shape_keys = Vec::with_capacity(json.shape_keys.len());
        for (name, values) in json.shape_keys.clone() {
            let track = frames(&name, values, &json)?;
            shape_keys.push((name, track));
        }
        let light = json.light.clone().unwrap_or_default();
        let light = LightTracks {
            strength: (light.strength)
                .map(|values| frames("light strength", values, &json))
                .transpose()?,
            color: (light.color)
                .map(|values| values.into_iter().map(Vec3::from).collect())
                .map(|values| frames("light color", values, &json))
                .transpose()?,
        };
        let nodes = match (json.nodes.is_empty(), glb) {
            (true, _) => Vec::new(),
            (false, None) => return Err(ClipError::NoGlb),
            (false, Some(glb)) => read_nodes(glb, &json.nodes)?,
        };
        let duration = json.frames as f32 / json.fps;
        Ok(Self { duration, nodes, shape_keys, light })
    }
}

fn frames<T: Sample>(name: &str, values: Vec<T>, json: &ClipJson) -> Result<Track<T>, ClipError> {
    let samples = values.len();
    let wrong = || ClipError::Samples { name: name.to_string(), samples, frames: json.frames };
    if samples != json.frames {
        return Err(wrong());
    }
    Track::frames(values, json.fps).ok_or_else(wrong)
}

fn read_nodes(glb: &[u8], keyed: &[String]) -> Result<Vec<NodeTracks>, ClipError> {
    let gltf = gltf::Gltf::from_slice(glb)?;
    let blob = gltf.blob.as_deref();
    let animation = gltf.animations().next().ok_or(ClipError::NoAnimation)?;
    let mut found: BTreeMap<&str, NodeTracks> = keyed
        .iter()
        .map(|name| (name.as_str(), NodeTracks { name: name.clone(), ..Default::default() }))
        .collect();

    for channel in animation.channels() {
        let Some(tracks) = channel.target().node().name().and_then(|name| found.get_mut(name))
        else {
            continue;
        };
        let unreadable = || ClipError::Unreadable(tracks.name.clone());
        let reader = channel.reader(|buffer| match buffer.source() {
            gltf::buffer::Source::Bin => blob,
            gltf::buffer::Source::Uri(_) => None,
        });
        let times: Vec<f32> = reader.read_inputs().ok_or_else(unreadable)?.collect();
        let how = channel.sampler().interpolation();
        match reader.read_outputs().ok_or_else(unreadable)? {
            ReadOutputs::Translations(values) => {
                let values = values.map(Vec3::from).collect();
                tracks.translation = Some(track(times, values, how).ok_or_else(unreadable)?);
            }
            ReadOutputs::Rotations(values) => {
                let values = values.into_f32().map(|q| Quat::from_array(q).normalize()).collect();
                tracks.rotation = Some(track(times, values, how).ok_or_else(unreadable)?);
            }
            ReadOutputs::Scales(values) => {
                let values = values.map(Vec3::from).collect();
                tracks.scale = Some(track(times, values, how).ok_or_else(unreadable)?);
            }
            ReadOutputs::MorphTargetWeights(_) => {}
        }
    }

    let mut nodes = Vec::with_capacity(found.len());
    for (name, tracks) in found {
        if tracks.translation.is_none() && tracks.rotation.is_none() && tracks.scale.is_none() {
            return Err(ClipError::Missing(name.to_string()));
        }
        nodes.push(tracks);
    }
    // The clip starts at its earliest key, whatever frame the exporter counted from.
    let start = nodes
        .iter()
        .flat_map(|n| {
            let t = n.translation.as_ref().map(Track::first_time);
            let r = n.rotation.as_ref().map(Track::first_time);
            let s = n.scale.as_ref().map(Track::first_time);
            [t, r, s].into_iter().flatten()
        })
        .fold(f32::INFINITY, f32::min);
    for node in &mut nodes {
        node.translation.iter_mut().for_each(|t| t.shift(start));
        node.rotation.iter_mut().for_each(|t| t.shift(start));
        node.scale.iter_mut().for_each(|t| t.shift(start));
    }
    Ok(nodes)
}

fn track<T: Sample>(
    times: Vec<f32>,
    values: Vec<T>,
    how: gltf::animation::Interpolation,
) -> Option<Track<T>> {
    match how {
        gltf::animation::Interpolation::Linear => Track::new(times, values, Interpolation::Linear),
        gltf::animation::Interpolation::Step => Track::new(times, values, Interpolation::Step),
        // In-tangent, value, out-tangent for each key. The exporter samples every frame, so
        // the values alone, joined linearly, are what Blender showed.
        gltf::animation::Interpolation::CubicSpline => {
            let values = values.chunks_exact(3).map(|key| key[1]).collect();
            Track::new(times, values, Interpolation::Linear)
        }
    }
}
