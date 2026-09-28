use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;

use crate::expressions::MAX_SHAPE_KEYS;
use crate::expressions::is_expression;
use crate::inspect::Report;
use crate::inspect::Severity;
use crate::inspect::examples;
use crate::inspect::tree::Decomposed;
use crate::inspect::tree::Gltf;

// Skins are re-pointed at the model's bones by name, so names must match and rest poses
// agree. The hierarchy above the bones doesn't matter.

// Loose enough for exporter float noise, tight enough to catch a mismatch you would see.
const REST_TRANSLATION_EPSILON: f32 = 1e-3;

const REST_ROTATION_DOT_EPSILON: f32 = 1e-6;

const REST_SCALE_EPSILON: f32 = 1e-3;

// Every node without a mesh counts as a bone, not only glTF skin joints: an armature
// exported on its own usually has no glTF skin.
pub(super) fn model_bones<'a>(
    model: &'a Gltf,
    file: &Path,
    report: &mut Report,
) -> HashMap<&'a str, usize> {
    let mut bones = HashMap::new();
    let mut duplicates = BTreeSet::new();
    for node in model.doc.nodes().filter(|n| n.mesh().is_none()) {
        let name = model.names[node.index()].as_str();
        if bones.insert(name, node.index()).is_some() {
            duplicates.insert(name);
        }
    }
    if !duplicates.is_empty() {
        let names: Vec<&str> = duplicates.into_iter().collect();
        let message = format!(
            "several bones share a name, so skins cannot tell them apart: {}",
            examples(&names)
        );
        report.push(Severity::Error, Some(file), message);
    }
    bones
}

pub(super) fn check_skin_file(
    path: &Path,
    skin: &Gltf,
    model: &Gltf,
    bones: &HashMap<&str, usize>,
    report: &mut Report,
) {
    let path = Some(path);
    if skin.doc.meshes().len() == 0 {
        report.push(Severity::Error, path, "holds no meshes".to_string());
        return;
    }

    let skin_joints: BTreeSet<usize> =
        skin.doc.skins().flat_map(|s| s.joints()).map(|j| j.index()).collect();

    let mut moved = Vec::new();
    let mut grafted = Vec::new();
    let mut orphans = Vec::new();
    for &joint in &skin_joints {
        let name = skin.names[joint].as_str();
        match bones.get(name) {
            Some(&base) => {
                if !same_rest(skin.rest[joint], model.rest[base]) {
                    moved.push(name);
                }
            }
            None => {
                match skin.ancestors(joint).find(|&a| bones.contains_key(skin.names[a].as_str())) {
                    Some(anchor) => grafted.push(format!("{name} -> {}", skin.names[anchor])),
                    None => orphans.push(name),
                }
            }
        }
    }

    if !orphans.is_empty() {
        let message = format!(
            "{} bone(s) are neither in the model's skeleton nor below a bone that is: {}",
            orphans.len(),
            examples(&orphans)
        );
        report.push(Severity::Error, path, message);
    }
    if !grafted.is_empty() {
        let grafted: Vec<&str> = grafted.iter().map(String::as_str).collect();
        let message = format!(
            "{} extra bone(s) will be attached to the model's skeleton, unanimated: {}",
            grafted.len(),
            examples(&grafted)
        );
        report.push(Severity::Warning, path, message);
    }
    if !moved.is_empty() {
        let message = format!(
            "rest pose of {} bone(s) differs from the model's, so the mesh will deform wrongly \
             (e.g. {}); bind the outfit to the same armature as the model",
            moved.len(),
            examples(&moved)
        );
        report.push(Severity::Warning, path, message);
    }

    let loose: Vec<&str> = skin
        .doc
        .nodes()
        .filter(|n| n.mesh().is_some() && n.skin().is_none())
        .filter(|n| {
            let mut chain = skin.ancestors(n.index());
            !chain.any(|a| bones.contains_key(skin.names[a].as_str()))
        })
        .map(|n| skin.names[n.index()].as_str())
        .collect();
    if !loose.is_empty() {
        let message = format!(
            "{} mesh(es) are neither skinned nor parented to a bone, so they will not follow \
             the character: {}",
            loose.len(),
            examples(&loose)
        );
        report.push(Severity::Warning, path, message);
    }
}

pub(super) fn check_skin_expressions(path: &Path, skin: &Gltf, report: &mut Report) {
    let path = Some(path);
    for mesh in skin.doc.meshes() {
        let keys = mesh.primitives().map(|p| p.morph_targets().len()).max().unwrap_or(0);
        if keys > MAX_SHAPE_KEYS {
            let name = mesh.name().unwrap_or("unnamed");
            let message = format!(
                "mesh {name:?} has {keys} shape keys; Bevy supports at most {MAX_SHAPE_KEYS}"
            );
            report.push(Severity::Error, path, message);
        }
    }

    for animation in skin.doc.animations().filter(is_expression) {
        if animation.name().is_none() {
            let message = format!(
                "clip #{} has no name, so it cannot be offered as an expression",
                animation.index()
            );
            report.push(Severity::Warning, path, message);
        }
    }
}

fn same_rest((t1, r1, s1): Decomposed, (t2, r2, s2): Decomposed) -> bool {
    let close =
        |a: [f32; 3], b: [f32; 3], eps: f32| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= eps);
    // q and -q are the same rotation.
    let dot: f32 = r1.iter().zip(r2).map(|(a, b)| a * b).sum();
    close(t1, t2, REST_TRANSLATION_EPSILON)
        && close(s1, s2, REST_SCALE_EPSILON)
        && dot.abs() >= 1.0 - REST_ROTATION_DOT_EPSILON
}
