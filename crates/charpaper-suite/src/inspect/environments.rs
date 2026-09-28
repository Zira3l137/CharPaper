use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::inspect::Report;
use crate::inspect::Severity;
use crate::inspect::examples;
use crate::inspect::open;
use crate::layout::Suite;

// An environment is lit only by what it ships: the lights in its scene and its maps. With
// neither, the character renders black in it.

pub(super) fn check_environments(suite: &Suite, report: &mut Report) {
    if suite.environments.is_empty() {
        let message = "no environments, so nothing lights the character and it will render \
                       black; add a scene with lights or reflection maps to `environment/`";
        report.push(Severity::Warning, None, message.to_string());
    }

    for environment in &suite.environments {
        let mut lights = 0;
        if let Some(scene) = &environment.scene {
            if let Some(gltf) = open(suite, scene, report) {
                lights = gltf.doc.nodes().filter(|n| n.light().is_some()).count();
                let cameras: Vec<&str> = gltf
                    .doc
                    .nodes()
                    .filter(|n| n.camera().is_some())
                    .map(|n| gltf.names[n.index()].as_str())
                    .collect();
                if !cameras.is_empty() {
                    let message = format!(
                        "{} camera(s) will be ignored; cameras belong in `cameras/`: {}",
                        cameras.len(),
                        examples(&cameras)
                    );
                    report.push(Severity::Warning, Some(scene), message);
                }
            }
        }

        let maps = [&environment.skybox, &environment.diffuse, &environment.specular];
        for path in maps.into_iter().flatten() {
            match read_cubemap(&suite.absolute(path)) {
                Err(message) => report.push(Severity::Error, Some(path), message),
                Ok(levels) if levels <= 1 && Some(path) == environment.specular.as_ref() => {
                    let message = "has a single level, so rough surfaces will reflect as \
                                   sharply as a mirror; export it with its blur levels (mipmaps)";
                    report.push(Severity::Warning, Some(path), message.to_string());
                }
                Ok(_) => {}
            }
        }

        let baked_later = environment.panorama.is_some();
        if let (Some(path), None) | (None, Some(path)) =
            (&environment.diffuse, &environment.specular)
            && !baked_later
        {
            let message = "reflections need both diffuse.ktx2 and specular.ktx2, so this one \
                           is not used for lighting";
            report.push(Severity::Warning, Some(path), message.to_string());
        }

        if lights == 0 && environment.reflections().is_none() && !baked_later {
            let message = format!(
                "environment {:?} has no lights and no reflection maps, so the character will \
                 render black in it",
                environment.name
            );
            report.push(Severity::Warning, None, message);
        }
    }
}

// The accepted compression schemes mirror the Bevy features the workspace enables.
fn read_cubemap(path: &Path) -> Result<u32, String> {
    const IDENTIFIER: [u8; 12] =
        [0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n'];

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut head = [0u8; 48];
    file.read_exact(&mut head).map_err(|_| "file is too short to be a KTX2 texture".to_string())?;
    if head[..12] != IDENTIFIER {
        return Err("not a KTX2 texture; environment maps must be .ktx2 cubemaps".to_string());
    }

    let word = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().unwrap());
    let faces = word(36);
    if faces != 6 {
        return Err(format!(
            "has {faces} face(s); environment maps must be cubemaps with 6 (an \
             equirectangular panorama has to be converted first)"
        ));
    }
    match word(44) {
        0 | 2 => Ok(word(40)),
        1 => Err("uses BasisLZ supercompression, which this build cannot decode; re-encode \
                  with Zstandard or no supercompression"
            .to_string()),
        scheme => Err(format!(
            "uses supercompression scheme {scheme}, which this build cannot decode; re-encode \
             with Zstandard or no supercompression"
        )),
    }
}
