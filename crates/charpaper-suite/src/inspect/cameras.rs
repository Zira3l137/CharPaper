use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

use gltf::camera::Projection;

use crate::inspect::Report;
use crate::inspect::Severity;
use crate::inspect::describe_clips;
use crate::inspect::examples;
use crate::inspect::tree::Gltf;
use crate::layout::ExportedCamera;
use crate::layout::Skin;
use crate::manifest::Camera;
use crate::manifest::MANIFEST_FILE;

// A lens setting that can't work leaves the image sharp or focused elsewhere, never broken,
// so only values the lens can't use at all are errors.
pub(super) fn check_lens(
    camera: &ExportedCamera,
    file: Option<&Gltf>,
    bones: &HashMap<&str, usize>,
    skins: &[(&Skin, Gltf)],
    report: &mut Report,
) {
    let manifest = Some(Path::new(MANIFEST_FILE));
    let name = &camera.name;
    let lens = &camera.settings;
    for (key, value) in [
        ("f_stop", lens.f_stop),
        ("focus_distance", lens.focus_distance),
        ("sensor_height_mm", lens.sensor_height_mm),
    ] {
        if value.is_some_and(|v| !(v > 0.0 && v.is_finite())) {
            let message = format!("camera {name:?}: `{key}` must be above 0");
            report.push(Severity::Error, manifest, message);
        }
    }

    if lens.f_stop.is_none() {
        if lens.focus_object.is_some()
            || lens.focus_distance.is_some()
            || lens.sensor_height_mm.is_some()
        {
            let message = format!(
                "camera {name:?}: depth of field is off without `f_stop`, so its other lens \
                 settings do nothing"
            );
            report.push(Severity::Warning, manifest, message);
        }
        return;
    }

    let Some(file) = file else {
        return;
    };
    let lens_node = file.doc.nodes().find(|n| {
        n.camera().is_some()
            && camera.node.as_deref().is_none_or(|node| file.names[n.index()] == node)
    });
    let orthographic = lens_node
        .and_then(|n| n.camera())
        .is_some_and(|c| matches!(c.projection(), Projection::Orthographic(_)));
    if orthographic {
        let message = format!(
            "camera {name:?} is orthographic, and depth of field needs a perspective camera, \
             so it stays off"
        );
        report.push(Severity::Warning, Some(&camera.file), message);
    }

    if let Some(object) = &lens.focus_object {
        let found = file.names.iter().any(|n| n == object)
            || bones.contains_key(object.as_str())
            || skins.iter().any(|(_, skin)| skin.names.iter().any(|n| n == object));
        if !found {
            let message = format!(
                "camera {name:?}: `focus_object` {object:?} is neither in {} nor on the \
                 character, so it focuses {} m away instead",
                camera.file.display(),
                lens.focus_distance()
            );
            report.push(Severity::Warning, manifest, message);
        }
    }
}

pub(super) fn check_orbit_lens(orbit: &Camera, report: &mut Report) {
    if orbit.f_stop.is_some_and(|v| !(v > 0.0 && v.is_finite())) {
        let message = "camera: `f_stop` must be above 0".to_string();
        report.push(Severity::Error, Some(Path::new(MANIFEST_FILE)), message);
    }
}

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
