use std::collections::BTreeSet;
use std::path::Path;

use gltf::Animation;
use gltf::animation::Property;

use crate::inspect::read_document;

pub const MAX_SHAPE_KEYS: usize = 256;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkinClips {
    pub expressions: Vec<String>,
    pub expression_meshes: BTreeSet<String>,
}

pub fn skin_clips(path: &Path) -> Result<SkinClips, String> {
    let doc = read_document(path)?;
    let mut clips = SkinClips::default();
    for animation in doc.animations().filter(is_expression) {
        let Some(name) = animation.name() else {
            continue;
        };
        clips.expressions.push(name.to_string());
        for channel in animation.channels() {
            let node = channel.target().node();
            let name =
                node.name().map_or_else(|| format!("GltfNode{}", node.index()), str::to_string);
            clips.expression_meshes.insert(name);
        }
    }
    clips.expressions.sort();
    clips.expressions.dedup();
    Ok(clips)
}

// A skin clip that keys only shape keys is an expression. One that also moves bones is a
// body animation exported along with the skin, and is ignored.
pub(crate) fn is_expression(animation: &Animation) -> bool {
    animation.channels().all(|c| c.target().property() == Property::MorphTargetWeights)
}
