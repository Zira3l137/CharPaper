use std::fs::File;
use std::io::Read;
use std::path::Path;

use gltf::Document;

// Reads only the JSON, never the binary chunk: that is where the megabytes are.
pub(crate) fn read_document(path: &Path) -> Result<Document, String> {
    let json = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb")) {
        read_glb_json(path)?
    } else {
        std::fs::read(path).map_err(|e| e.to_string())?
    };
    let root = gltf::json::Root::from_slice(&json).map_err(|e| {
        // Bevy can't load this extension at all, and the raw parse error wouldn't tell the artist
        // which export setting to change.
        let pointer = b"KHR_animation_pointer";
        match json.windows(pointer.len()).any(|w| w == pointer) {
            true => "animates properties through KHR_animation_pointer (e.g. focal length), \
                     which Bevy cannot load; turn off animation pointer export in Blender's \
                     glTF exporter"
                .to_string(),
            false => format!("invalid glTF JSON: {e}"),
        }
    })?;
    Document::from_json(root).map_err(|e| e.to_string())
}

// A .glb is a 12-byte header, then the JSON chunk, then usually one binary chunk.
fn read_glb_json(path: &Path) -> Result<Vec<u8>, String> {
    const MAGIC: &[u8; 4] = b"glTF";
    const JSON_CHUNK: u32 = 0x4E4F_534A;

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut head = [0u8; 20];
    file.read_exact(&mut head).map_err(|_| "file is too short to be a .glb".to_string())?;

    let word = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().unwrap());
    if &head[0..4] != MAGIC {
        return Err("not a .glb file (bad magic)".to_string());
    }
    if word(4) != 2 {
        return Err(format!("glTF container version {} is not supported", word(4)));
    }
    if word(16) != JSON_CHUNK {
        return Err("first .glb chunk is not JSON".to_string());
    }

    let mut json = vec![0u8; word(12) as usize];
    file.read_exact(&mut json).map_err(|_| "JSON chunk is truncated".to_string())?;
    Ok(json)
}
