use std::path::Path;

use gltf::Document;

use crate::document::read_document;

// glTF has no place for shape key names; Blender writes them into each mesh's extras as
// `targetNames`. Bevy reads them there too, but drops them with a GPU-only mesh's RAM copy.

#[derive(Debug, Clone, PartialEq)]
pub struct MeshShapeKeys {
    // The node's name, as Bevy names its entity.
    pub node: String,
    // In weight order. Empty for a target the file gives no name.
    pub keys: Vec<String>,
}

pub fn mesh_shape_keys(path: &Path) -> Result<Vec<MeshShapeKeys>, String> {
    read_document(path).map(|doc| in_document(&doc))
}

pub(crate) fn in_document(doc: &Document) -> Vec<MeshShapeKeys> {
    doc.nodes()
        .filter_map(|node| {
            let mesh = node.mesh()?;
            let count = mesh.primitives().map(|p| p.morph_targets().len()).max().unwrap_or(0);
            if count == 0 {
                return None;
            }
            let mut keys = target_names(&mesh);
            keys.resize(count, String::new());
            let name =
                node.name().map_or_else(|| format!("GltfNode{}", node.index()), str::to_string);
            Some(MeshShapeKeys { node: name, keys })
        })
        .collect()
}

fn target_names(mesh: &gltf::Mesh) -> Vec<String> {
    let Some(extras) = mesh.extras() else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(extras.get()) else {
        return Vec::new();
    };
    let Some(names) = value.get("targetNames").and_then(|n| n.as_array()) else {
        return Vec::new();
    };
    names.iter().map(|n| n.as_str().unwrap_or_default().to_string()).collect()
}
