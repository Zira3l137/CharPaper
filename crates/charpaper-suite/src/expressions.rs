//! Expressions: the clips stored in a skin's own file.
//!
//! Each named clip in a skin's `.glb` is one of that skin's expressions,
//! named after the clip, which is the Blender action it was exported from.
//! They key shape keys; the app loops the chosen one on the skin's faces, and
//! stopping it returns them to the weights they were exported with.

use std::path::Path;

use crate::inspect::read_document;

/// The most shape keys Bevy supports on one mesh.
pub const MAX_SHAPE_KEYS: usize = 256;

/// The expressions in a skin file, in name order. Reads only the file's JSON.
pub fn expressions(path: &Path) -> Result<Vec<String>, String> {
    let doc = read_document(path)?;
    let mut names: Vec<String> =
        doc.animations().filter_map(|a| a.name().map(str::to_string)).collect();
    names.sort();
    names.dedup();
    Ok(names)
}
