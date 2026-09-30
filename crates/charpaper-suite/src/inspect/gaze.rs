use std::collections::HashMap;
use std::path::Path;

use crate::inspect::Report;
use crate::inspect::Severity;
use crate::inspect::tree::Gltf;
use crate::layout::Skin;
use crate::manifest::Gaze;
use crate::manifest::MANIFEST_FILE;
use crate::shape_keys::in_document;

// A missing bone turns gaze off for the whole suite, so it is an error. A shape key is
// only a warning, and only when no skin has it: skins are free to leave any out.
pub(super) fn check_gaze(
    gaze: &Gaze,
    model: &Path,
    bones: &HashMap<&str, usize>,
    skins: &[(&Skin, Gltf)],
    report: &mut Report,
) {
    let manifest = Some(Path::new(MANIFEST_FILE));
    if gaze.eyes.is_empty() {
        let message = "gaze: `eyes` lists no bones, so the character never follows the cursor";
        report.push(Severity::Error, manifest, message.to_string());
    }
    for bone in gaze.bones().filter(|b| !bones.contains_key(b)) {
        let message = format!(
            "gaze: {bone:?} is not a bone of the armature in {}, so the character never \
             follows the cursor",
            model.display()
        );
        report.push(Severity::Error, manifest, message);
    }

    let names: Vec<Vec<String>> =
        skins.iter().flat_map(|(_, gltf)| in_document(&gltf.doc)).map(|mesh| mesh.keys).collect();
    for key in gaze.shape_keys.named() {
        if !names.iter().any(|keys| keys.iter().any(|k| k == key)) {
            let message = format!(
                "gaze: no skin has a shape key named {key:?}, so it never moves; check the \
                 spelling against Blender"
            );
            report.push(Severity::Warning, manifest, message);
        }
    }
}
