use std::collections::HashMap;

use gltf::Document;

// A glTF file's node tree, indexed the way the checks need it.
pub(super) struct Gltf {
    pub(super) doc: Document,
    // Per node, the name Bevy gives its entity.
    pub(super) names: Vec<String>,
    pub(super) parents: Vec<Option<usize>>,
    pub(super) rest: Vec<Decomposed>,
    // Per node, its `/`-joined name chain from the scene root. None outside the spawned scene.
    pub(super) paths: HashMap<usize, String>,
}

impl Gltf {
    pub(super) fn ancestors(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.parents[node], |&n| self.parents[n])
    }
}

pub(super) fn index(doc: Document) -> Gltf {
    let count = doc.nodes().len();
    let names = doc
        .nodes()
        .map(|n| n.name().map_or_else(|| format!("GltfNode{}", n.index()), str::to_string))
        .collect::<Vec<_>>();

    let mut parents = vec![None; count];
    for node in doc.nodes() {
        for child in node.children() {
            parents[child.index()] = Some(node.index());
        }
    }

    // Bevy spawns the default scene, falling back to the first one.
    let mut paths = HashMap::new();
    if let Some(scene) = doc.default_scene().or_else(|| doc.scenes().next()) {
        let mut stack: Vec<(gltf::Node, String)> =
            scene.nodes().map(|n| (n.clone(), names[n.index()].clone())).collect();
        while let Some((node, path)) = stack.pop() {
            for child in node.children() {
                stack.push((child.clone(), format!("{path}/{}", names[child.index()])));
            }
            paths.insert(node.index(), path);
        }
    }

    let rest = doc.nodes().map(|n| n.transform().decomposed()).collect();
    Gltf { doc, names, parents, rest, paths }
}

pub(super) type Decomposed = ([f32; 3], [f32; 4], [f32; 3]);
