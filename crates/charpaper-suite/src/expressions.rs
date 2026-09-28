//! Which clips of a skin are its expressions.
//!
//! A clip in a skin's file is an expression when it keys shape keys and
//! nothing else: the app loops the chosen one on that skin, and stopping it
//! returns the keys to 0. A clip that also moves bones is a body animation
//! that came along with the skin's export; it is ignored there, since body
//! animations, correctives included, are played from `animations/`.

use std::collections::BTreeSet;
use std::path::Path;

use gltf::Animation;
use gltf::animation::Property;

use crate::inspect::read_document;

/// The most shape keys Bevy supports on one mesh.
pub const MAX_SHAPE_KEYS: usize = 256;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkinClips {
    /// Expression names, in name order.
    pub expressions: Vec<String>,
    /// The mesh objects any expression keys, by name. Expressions own them:
    /// body animations never key their shape keys.
    pub expression_meshes: BTreeSet<String>,
}

/// Reads only the file's JSON.
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

pub(crate) fn is_expression(animation: &Animation) -> bool {
    animation.channels().all(|c| c.target().property() == Property::MorphTargetWeights)
}
