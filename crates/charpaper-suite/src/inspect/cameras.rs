use std::collections::BTreeSet;
use std::collections::HashSet;
use std::path::Path;

use crate::inspect::Report;
use crate::inspect::Severity;
use crate::inspect::describe_clips;
use crate::inspect::examples;
use crate::inspect::tree::Gltf;

pub(super) fn check_camera_file(path: &Path, gltf: &Gltf, report: &mut Report) {
    let path = Some(path);
    let cameras: Vec<usize> =
        gltf.doc.nodes().filter(|n| n.camera().is_some()).map(|n| n.index()).collect();
    if cameras.is_empty() {
        report.push(Severity::Error, path, "holds no camera".to_string());
        return;
    }

    for &camera in &cameras {
        if !gltf.paths.contains_key(&camera) {
            let message = format!(
                "camera {:?} is outside the file's scene, so it would never be spawned",
                gltf.names[camera]
            );
            report.push(Severity::Error, path, message);
        }
    }

    if gltf.doc.meshes().len() > 0 {
        let message = "contains meshes that will be loaded and never shown; export the cameras \
                       and the empties they hang from only";
        report.push(Severity::Warning, path, message.to_string());
    }

    let clips: Vec<Option<&str>> = gltf.doc.animations().map(|a| a.name()).collect();
    if cameras.len() == 1 && clips.len() > 1 {
        let message = format!(
            "holds {} clips ({}); a camera plays exactly one, looping",
            clips.len(),
            describe_clips(&clips)
        );
        report.push(Severity::Error, path, message);
    }

    let carriers = |camera: usize| -> HashSet<usize> {
        std::iter::once(camera).chain(gltf.ancestors(camera)).collect()
    };
    if cameras.len() > 1 {
        for &camera in &cameras {
            let carrying = carriers(camera);
            let moving: Vec<Option<&str>> = gltf
                .doc
                .animations()
                .filter(|a| a.channels().any(|c| carrying.contains(&c.target().node().index())))
                .map(|a| a.name())
                .collect();
            if moving.len() > 1 {
                let message = format!(
                    "camera {:?} is moved by {} clips ({}); only the first plays",
                    gltf.names[camera],
                    moving.len(),
                    describe_clips(&moving)
                );
                report.push(Severity::Warning, path, message);
            }
        }
    }

    let moving: HashSet<usize> = cameras.iter().flat_map(|&c| carriers(c)).collect();
    let idle: BTreeSet<&str> = gltf
        .doc
        .animations()
        .flat_map(|a| a.channels())
        .map(|c| c.target().node().index())
        .filter(|node| !moving.contains(node))
        .map(|node| gltf.names[node].as_str())
        .collect();
    if !idle.is_empty() {
        let idle: Vec<&str> = idle.into_iter().collect();
        let message = format!(
            "{} animated node(s) do not carry a camera, so animating them changes nothing: {}",
            idle.len(),
            examples(&idle)
        );
        report.push(Severity::Warning, path, message);
    }
}
