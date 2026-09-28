use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use gltf::animation::Property;

use crate::expressions::is_expression;
use crate::inspect::Report;
use crate::inspect::Severity;
use crate::inspect::describe_clips;
use crate::inspect::examples;
use crate::inspect::open;
use crate::inspect::tree::Gltf;
use crate::layout::ClipSet;
use crate::layout::Skin;
use crate::layout::Suite;

// Animation curves find their bone by the chain of node names from the scene root, so an
// animation file must repeat the model's hierarchy exactly, armature object included.

pub(super) fn check_animations(
    suite: &Suite,
    model: &Gltf,
    skins: &[(&Skin, Gltf)],
    report: &mut Report,
) {
    let model_paths: HashSet<&str> = model.paths.values().map(String::as_str).collect();
    let mut clip_names: BTreeMap<String, PathBuf> = BTreeMap::new();

    for file in &suite.animations {
        let Some(gltf) = open(suite, &file.path, report) else {
            continue;
        };
        let path = file.path.as_path();

        check_correctives(path, &gltf, skins, report);

        let available: Vec<Option<&str>> = gltf.doc.animations().map(|a| a.name()).collect();
        let mut register = |name: &str, report: &mut Report| {
            if let Some(first) = clip_names.insert(name.to_string(), file.path.clone()) {
                let message =
                    format!("animation name {name:?} is already used by {}", first.display());
                report.push(Severity::Error, Some(path), message);
            }
        };

        match &file.clips {
            ClipSet::All => {
                if available.is_empty() {
                    report.push(Severity::Warning, Some(path), "holds no animations".to_string());
                }
                for (i, name) in available.iter().enumerate() {
                    match name {
                        Some(name) => register(name, report),
                        None => {
                            let message =
                                format!("clip #{i} has no name; name it or list it in suite.toml");
                            report.push(Severity::Warning, Some(path), message);
                        }
                    }
                }
            }
            ClipSet::Listed(bindings) => {
                for binding in bindings {
                    let found = match &binding.clip {
                        Some(clip) => available.contains(&Some(clip.as_str())),
                        None => available.len() == 1,
                    };
                    if !found {
                        let message = match &binding.clip {
                            Some(clip) => format!(
                                "animation {:?}: no clip named {clip:?} (found: {})",
                                binding.name,
                                describe_clips(&available)
                            ),
                            None => format!(
                                "animation {:?}: the file holds {} clips, so `clip` must name one (found: {})",
                                binding.name,
                                available.len(),
                                describe_clips(&available)
                            ),
                        };
                        report.push(Severity::Error, Some(path), message);
                    }
                    register(&binding.name, report);
                }
            }
        }

        let unmatched: BTreeSet<&str> = gltf
            .doc
            .animations()
            .flat_map(|a| a.channels())
            .filter(|c| c.target().property() != Property::MorphTargetWeights)
            .filter_map(|c| gltf.paths.get(&c.target().node().index()))
            .map(String::as_str)
            .filter(|p| !model_paths.contains(p))
            .collect();
        if !unmatched.is_empty() {
            let unmatched: Vec<&str> = unmatched.into_iter().collect();
            let message = format!(
                "{} animated node(s) are not at the same place in the model, so they will not \
                 move (e.g. {}); names from the scene root down must match exactly, the \
                 armature object's own name included",
                unmatched.len(),
                examples(&unmatched)
            );
            report.push(Severity::Warning, Some(path), message);
        }
    }

    if let Some(default) = &suite.default_animation {
        if !clip_names.contains_key(default) {
            let mut message =
                format!("default animation {default:?} is not one of the suite's clips\n");
            message.push_str(
                format!(
                    "available clips: {:#?}",
                    clip_names
                        .iter()
                        .map(|(k, v)| format!("{k} - {}", v.display()))
                        .collect::<Vec<_>>()
                )
                .as_str(),
            );
            report.push(Severity::Error, None, message);
        }
    }
}

// glTF keys shape keys only on a mesh in the same file, so an animation file carries the
// meshes its correctives key. The app never loads them from there.
fn check_correctives(path: &Path, gltf: &Gltf, skins: &[(&Skin, Gltf)], report: &mut Report) {
    let path = Some(path);
    let keyed: BTreeSet<usize> = gltf
        .doc
        .animations()
        .flat_map(|a| a.channels())
        .filter(|c| c.target().property() == Property::MorphTargetWeights)
        .map(|c| c.target().node().index())
        .collect();

    let unused: Vec<&str> = gltf
        .doc
        .nodes()
        .filter(|n| n.mesh().is_some() && !keyed.contains(&n.index()))
        .map(|n| gltf.names[n.index()].as_str())
        .collect();
    if !unused.is_empty() {
        let message = format!(
            "{} mesh(es) have no shape keys any clip keys, so they only take up space; export \
             the armature and just the meshes the animations key: {}",
            unused.len(),
            examples(&unused)
        );
        report.push(Severity::Warning, path, message);
    }

    for node in keyed {
        let (Some(at), name) = (gltf.paths.get(&node), gltf.names[node].as_str()) else {
            continue;
        };
        let keys = shape_keys(gltf, node);
        let mut found = false;
        for (skin, skin_gltf) in skins {
            let Some((&twin, _)) = skin_gltf.paths.iter().find(|(_, p)| *p == at) else {
                continue;
            };
            found = true;
            let theirs = shape_keys(skin_gltf, twin);
            if theirs != keys {
                let message = format!(
                    "mesh {name:?} has {keys} shape key(s) here but {theirs} in skin {:?}; \
                     they must be the same keys in the same order",
                    skin.name
                );
                report.push(Severity::Warning, path, message);
            }
            let owned = skin_gltf
                .doc
                .animations()
                .filter(is_expression)
                .flat_map(|a| a.channels())
                .any(|c| c.target().node().index() == twin);
            if owned {
                let message = format!(
                    "mesh {name:?} is keyed by skin {:?}'s expressions, which own its shape \
                     keys there; this file's keys on it are ignored with that skin",
                    skin.name
                );
                report.push(Severity::Warning, path, message);
            }
        }
        if !found {
            let message = format!(
                "keys the shape keys of {at:?}, which no skin has at that place, so they move \
                 nothing"
            );
            report.push(Severity::Warning, path, message);
        }
    }
}

fn shape_keys(gltf: &Gltf, node: usize) -> usize {
    let node = gltf.doc.nodes().nth(node);
    node.and_then(|n| n.mesh())
        .map_or(0, |m| m.primitives().map(|p| p.morph_targets().len()).max().unwrap_or(0))
}
