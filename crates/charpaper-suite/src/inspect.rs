//! Opens the suite's glTF files and checks that the pieces fit together.
//!
//! Only each file's JSON is read, never its binary chunk: the JSON describes
//! the whole node tree, and models run to tens of megabytes of vertex data we
//! have no use for here.
//!
//! The checks mirror how the scene will use the files:
//! - Animation curves find their bone by the chain of node names from the
//!   scene root down, so an animation file must repeat the model's hierarchy
//!   exactly, armature object name included.
//! - Skins are re-pointed at the model's bones by bone name, so their names
//!   must match and their rest poses must agree. The hierarchy above the bones
//!   does not matter for skins.
//! - The model is only the armature. Any mesh in it would stay visible under
//!   every skin, which is almost never what the artist meant.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use gltf::Document;

use crate::layout::ClipSet;
use crate::layout::Suite;

/// Past these, a skin's rest pose no longer matches the model's. The limits
/// are loose enough to absorb exporter float noise and tight enough that a
/// real mismatch is visible on screen long before it reaches them.
const REST_TRANSLATION_EPSILON: f32 = 1e-3;
const REST_ROTATION_DOT_EPSILON: f32 = 1e-6;
const REST_SCALE_EPSILON: f32 = 1e-3;

const EXAMPLES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    /// Relative to the suite root; `None` for the suite as a whole.
    pub file: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.count(Severity::Error)
    }

    pub fn warnings(&self) -> usize {
        self.count(Severity::Warning)
    }

    fn count(&self, severity: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == severity).count()
    }

    fn push(&mut self, severity: Severity, file: Option<&Path>, message: String) {
        self.findings.push(Finding { severity, file: file.map(Path::to_path_buf), message });
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        match &self.file {
            Some(file) => write!(f, "{label:<7} {}: {}", file.display(), self.message),
            None => write!(f, "{label:<7} {}", self.message),
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for finding in &self.findings {
            writeln!(f, "{finding}")?;
        }
        write!(f, "{} error(s), {} warning(s)", self.errors(), self.warnings())
    }
}

pub fn inspect(suite: &Suite) -> Report {
    let mut report = Report::default();

    let Some(model) = open(suite, &suite.model, &mut report) else {
        return report;
    };
    let bones = model_bones(&model, &suite.model, &mut report);

    let meshes = model.doc.nodes().filter(|n| n.mesh().is_some()).count();
    if meshes == 0 && suite.skins.is_empty() {
        let message = "nothing to show: the model has no meshes and the suite has no skins";
        report.push(Severity::Error, None, message.to_string());
    } else if meshes > 0 && !suite.skins.is_empty() {
        let message = format!(
            "{meshes} mesh(es) are shown under every skin; move them into the skins that \
             should own them"
        );
        report.push(Severity::Warning, Some(&suite.model), message);
    }

    let unnamed = model.doc.nodes().filter(|n| n.name().is_none()).count();
    if unnamed > 0 {
        let message = format!(
            "{unnamed} node(s) have no name; Bevy names them by index, which other files \
             cannot match"
        );
        report.push(Severity::Warning, Some(&suite.model), message);
    }

    check_animations(suite, &model, &mut report);
    for skin in &suite.skins {
        if let Some(gltf) = open(suite, &skin.file, &mut report) {
            check_skin_file(&skin.file, &gltf, &model, &bones, &mut report);
        }
    }

    if let Some(scene) = &suite.environment.scene {
        open(suite, scene, &mut report);
    }

    report.findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    report
}

struct Gltf {
    doc: Document,
    /// Per node index, the name Bevy will give the node's entity.
    names: Vec<String>,
    parents: Vec<Option<usize>>,
    /// Per node index, its local transform as (translation, rotation, scale).
    rest: Vec<Decomposed>,
    /// Per node index, the `/`-joined name chain from its scene root. Nodes
    /// outside the scene Bevy spawns have none.
    paths: HashMap<usize, String>,
}

impl Gltf {
    fn ancestors(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.parents[node], |&n| self.parents[n])
    }
}

fn open(suite: &Suite, file: &Path, report: &mut Report) -> Option<Gltf> {
    match read_document(&suite.absolute(file)) {
        Ok(doc) => Some(index(doc)),
        Err(message) => {
            report.push(Severity::Error, Some(file), message);
            None
        }
    }
}

fn read_document(path: &Path) -> Result<Document, String> {
    let json = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb")) {
        read_glb_json(path)?
    } else {
        std::fs::read(path).map_err(|e| e.to_string())?
    };
    let root =
        gltf::json::Root::from_slice(&json).map_err(|e| format!("invalid glTF JSON: {e}"))?;
    Document::from_json(root).map_err(|e| e.to_string())
}

/// A .glb is a 12-byte header, then a JSON chunk that the spec requires to
/// come first, then (usually) one binary chunk. Each chunk starts with its
/// byte length and a type tag, all little-endian `u32`s.
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

fn index(doc: Document) -> Gltf {
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

/// Node name to node index, for every node in the model that holds no mesh.
///
/// Every such node counts as a bone rather than only the joints of the file's
/// glTF skins: a glTF skin exists to bind a mesh, so an armature exported on
/// its own usually has none. Mesh nodes are left out so a mesh named after a
/// bone ("Head") does not look like a duplicate bone.
fn model_bones<'a>(model: &'a Gltf, file: &Path, report: &mut Report) -> HashMap<&'a str, usize> {
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

fn check_animations(suite: &Suite, model: &Gltf, report: &mut Report) {
    let model_paths: HashSet<&str> = model.paths.values().map(String::as_str).collect();
    let mut clip_names: BTreeMap<String, PathBuf> = BTreeMap::new();

    for file in &suite.animations {
        let Some(gltf) = open(suite, &file.path, report) else {
            continue;
        };
        let path = file.path.as_path();

        if gltf.doc.meshes().len() > 0 {
            let message = "contains meshes that will be loaded and never shown; \
                           export animation files with the armature only";
            report.push(Severity::Warning, Some(path), message.to_string());
        }

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
                                "animation {:?}: the file holds {} clips, so `clip` must name one",
                                binding.name,
                                available.len()
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

fn check_skin_file(
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

type Decomposed = ([f32; 3], [f32; 4], [f32; 3]);

fn same_rest((t1, r1, s1): Decomposed, (t2, r2, s2): Decomposed) -> bool {
    let close =
        |a: [f32; 3], b: [f32; 3], eps: f32| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= eps);
    // q and -q are the same rotation, hence the absolute value.
    let dot: f32 = r1.iter().zip(r2).map(|(a, b)| a * b).sum();
    close(t1, t2, REST_TRANSLATION_EPSILON)
        && close(s1, s2, REST_SCALE_EPSILON)
        && dot.abs() >= 1.0 - REST_ROTATION_DOT_EPSILON
}

fn describe_clips(clips: &[Option<&str>]) -> String {
    if clips.is_empty() {
        return "none".to_string();
    }
    let names: Vec<&str> = clips.iter().map(|c| c.unwrap_or("<unnamed>")).collect();
    names.join(", ")
}

fn examples(items: &[&str]) -> String {
    let shown = items.iter().take(EXAMPLES).copied().collect::<Vec<_>>().join(", ");
    match items.len().saturating_sub(EXAMPLES) {
        0 => shown,
        more => format!("{shown} and {more} more"),
    }
}
