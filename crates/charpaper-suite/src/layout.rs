use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use gltf::Document;
use tracing::debug;

use crate::document::read_document;
use crate::error::SuiteError;
use crate::manifest::Camera;
use crate::manifest::CameraEntry;
use crate::manifest::EnvironmentEntry;
use crate::manifest::Gaze;
use crate::manifest::MANIFEST_FILE;
use crate::manifest::Manifest;
use crate::manifest::NO_LUT;
use crate::manifest::PlayMode;
use crate::manifest::Post;
use crate::manifest::SCHEMA_VERSION;

const ANIMATIONS_DIR: &str = "animations";
const SKINS_DIR: &str = "skins";
const CAMERAS_DIR: &str = "cameras";
const LUTS_DIR: &str = "luts";

pub const ORBIT_CAMERA: &str = "orbit";
const ENVIRONMENT_DIR: &str = "environment";

pub const SKYBOX_MAP: &str = "skybox.ktx2";
pub const DIFFUSE_MAP: &str = "diffuse.ktx2";
pub const SPECULAR_MAP: &str = "specular.ktx2";

#[derive(Debug, Clone)]
pub struct Suite {
    pub root: PathBuf,
    pub name: String,
    pub model: PathBuf,
    // No model file of its own, so `model` is the default skin's file, whose meshes are that
    // skin's and must not show twice.
    pub model_is_skin: bool,
    pub animations: Vec<AnimationFile>,
    pub skins: Vec<Skin>,
    pub default_skin: Option<String>,
    pub default_animation: Option<String>,
    pub cameras: Vec<ExportedCamera>,
    pub default_camera: Option<String>,
    pub environments: Vec<Environment>,
    pub default_environment: Option<String>,
    pub luts: Vec<LutFile>,
    pub post: Post,
    pub camera: Camera,
    pub gaze: Option<Gaze>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationFile {
    pub path: PathBuf,
    pub clips: ClipSet,
}

// Once suite.toml mentions a file, only the clips it lists are used.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipSet {
    All,
    Listed(Vec<ClipBinding>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipBinding {
    pub name: String,
    pub clip: Option<String>,
    pub mode: PlayMode,
    pub gaze: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    pub name: String,
    pub file: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LutFile {
    pub name: String,
    pub file: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportedCamera {
    pub name: String,
    pub file: PathBuf,
    pub node: Option<String>,
    pub clip: Option<String>,
    pub settings: CameraEntry,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    pub name: String,
    pub scene: Option<PathBuf>,
    pub panorama: Option<PathBuf>,
    pub skybox: Option<PathBuf>,
    pub diffuse: Option<PathBuf>,
    pub specular: Option<PathBuf>,
    pub settings: EnvironmentEntry,
}

impl Environment {
    pub fn folder(&self) -> PathBuf {
        Path::new(ENVIRONMENT_DIR).join(&self.name)
    }

    pub fn find_maps(&mut self, root: &Path) {
        let folder = self.folder();
        let map = |file: &str| {
            let path = folder.join(file);
            root.join(&path).is_file().then_some(path)
        };
        (self.skybox, self.diffuse, self.specular) =
            (map(SKYBOX_MAP), map(DIFFUSE_MAP), map(SPECULAR_MAP));
    }

    pub fn needs_baking(&self) -> bool {
        self.panorama.is_some()
            && (self.skybox.is_none() || self.diffuse.is_none() || self.specular.is_none())
    }

    pub fn reflections(&self) -> Option<(&Path, &Path)> {
        Some((self.diffuse.as_deref()?, self.specular.as_deref()?))
    }

    pub fn sky(&self) -> Option<&Path> {
        self.skybox.as_deref().or(self.specular.as_deref())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Look {
    pub post: Post,
    pub environments: BTreeMap<String, EnvironmentEntry>,
}

impl Look {
    pub fn environment(&self, name: &str) -> EnvironmentEntry {
        self.environments.get(name).cloned().unwrap_or_default()
    }
}

impl Suite {
    pub fn look(&self) -> Look {
        Look {
            post: self.post.clone(),
            environments: self
                .environments
                .iter()
                .filter(|e| e.settings != EnvironmentEntry::default())
                .map(|e| (e.name.clone(), e.settings.clone()))
                .collect(),
        }
    }

    pub fn load(root: impl AsRef<Path>) -> Result<Self, SuiteError> {
        let root = root.as_ref();
        let path = root.join(MANIFEST_FILE);
        let text = fs::read_to_string(&path)
            .map_err(|source| SuiteError::Io { path: path.clone(), source })?;
        let manifest =
            toml::from_str(&text).map_err(|source| SuiteError::Manifest { path, source })?;
        Self::resolve(root, manifest)
    }

    pub fn resolve(root: impl AsRef<Path>, manifest: Manifest) -> Result<Self, SuiteError> {
        let root = root.as_ref();
        if manifest.schema == 0 || manifest.schema > SCHEMA_VERSION {
            return Err(SuiteError::Schema { found: manifest.schema, supported: SCHEMA_VERSION });
        }

        let name = match &manifest.name {
            Some(name) => name.clone(),
            None => root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        };

        let animations = resolve_animations(root, &manifest)?;
        let skins: Vec<Skin> = named_files(root, SKINS_DIR, "skin")?
            .into_iter()
            .map(|(name, file)| Skin { name, file })
            .collect();
        let environments = resolve_environments(root, &manifest)?;
        let cameras = resolve_cameras(root, &manifest)?;
        if cameras.iter().any(|c| c.name == ORBIT_CAMERA) {
            return Err(SuiteError::ReservedName { kind: "camera", name: ORBIT_CAMERA.into() });
        }

        let default_skin = match manifest.character.default_skin {
            Some(name) if !skins.iter().any(|s| s.name == name) => {
                return Err(SuiteError::UnknownDefault { kind: "skin", name });
            }
            Some(name) => Some(name),
            None => {
                let first = skins.first().map(|s| s.name.clone());
                if let Some(name) = &first {
                    debug!("no default skin set; using {name:?}");
                }
                first
            }
        };

        let own_model = match &manifest.character.model {
            Some(path) => Some(existing(root, path)?),
            None => sole(root, "", "the suite folder", is_gltf)?,
        };
        let skeleton_skin = default_skin.as_ref().and_then(|d| skins.iter().find(|s| &s.name == d));
        let (model, model_is_skin) = match (own_model, skeleton_skin) {
            (Some(model), _) => (model, false),
            (None, Some(skin)) => {
                debug!("no model file; taking the armature from skin {:?}", skin.name);
                (skin.file.clone(), true)
            }
            (None, None) => return Err(SuiteError::NoModel),
        };

        let default_camera = match manifest.camera.default.as_deref() {
            None | Some(ORBIT_CAMERA) => None,
            Some(name) if !cameras.iter().any(|c| c.name == name) => {
                return Err(SuiteError::UnknownDefault { kind: "camera", name: name.into() });
            }
            Some(name) => Some(name.to_string()),
        };

        let luts: Vec<LutFile> = files_in(root, LUTS_DIR)?
            .into_iter()
            .filter(|p| has_extension(p, &["cube"]))
            .map(|file| {
                let name = file.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                LutFile { name, file }
            })
            .collect();
        if luts.iter().any(|l| l.name == NO_LUT) {
            return Err(SuiteError::ReservedName { kind: "LUT", name: NO_LUT.into() });
        }

        let default_environment = match &manifest.environment.default {
            Some(name) if !environments.iter().any(|e| &e.name == name) => {
                return Err(SuiteError::UnknownDefault { kind: "environment", name: name.clone() });
            }
            Some(name) => Some(name.clone()),
            None => environments.first().map(|e| e.name.clone()),
        };

        Ok(Self {
            root: root.to_path_buf(),
            name,
            model,
            model_is_skin,
            animations,
            skins,
            default_skin,
            default_animation: manifest.character.default_animation,
            cameras,
            default_camera,
            environments,
            default_environment,
            luts,
            post: manifest.post,
            camera: manifest.camera,
            gaze: manifest.gaze,
        })
    }

    pub fn absolute(&self, relative: &Path) -> PathBuf {
        self.root.join(relative)
    }
}

// Whether each one loads is left to Suite::load, so one broken suite can't hide the others.
pub fn discover(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>, SuiteError> {
    let dir = dir.as_ref();
    let mut found: Vec<PathBuf> =
        read_dir(dir)?.into_iter().filter(|path| path.join(MANIFEST_FILE).is_file()).collect();
    found.sort();
    Ok(found)
}

fn resolve_animations(root: &Path, manifest: &Manifest) -> Result<Vec<AnimationFile>, SuiteError> {
    let mut listed: BTreeMap<PathBuf, Vec<ClipBinding>> = BTreeMap::new();
    for (name, entry) in &manifest.animations {
        let binding = ClipBinding {
            name: name.clone(),
            clip: entry.clip.clone(),
            mode: entry.mode,
            gaze: entry.gaze.unwrap_or(true),
        };
        listed.entry(existing(root, &entry.file)?).or_default().push(binding);
    }

    let mut files: Vec<AnimationFile> = listed
        .into_iter()
        .map(|(path, bindings)| AnimationFile { path, clips: ClipSet::Listed(bindings) })
        .collect();

    for path in files_in(root, ANIMATIONS_DIR)?.into_iter().filter(|p| is_gltf(p)) {
        if !files.iter().any(|f| f.path == path) {
            debug!("discovered animation file {}", path.display());
            files.push(AnimationFile { path, clips: ClipSet::All });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn resolve_cameras(root: &Path, manifest: &Manifest) -> Result<Vec<ExportedCamera>, SuiteError> {
    let mut cameras: Vec<ExportedCamera> = Vec::new();
    for (name, file) in named_files(root, CAMERAS_DIR, "camera")? {
        // An unreadable file stays one camera here; `inspect` reports what is wrong with it.
        let found = read_document(&root.join(&file)).map(|d| camera_nodes(&d)).unwrap_or_default();
        if found.len() <= 1 {
            cameras.push(ExportedCamera {
                name,
                file,
                node: None,
                clip: None,
                settings: CameraEntry::default(),
            });
            continue;
        }
        for (node, clip) in found {
            let camera = ExportedCamera {
                name: node.clone(),
                file: file.clone(),
                node: Some(node),
                clip,
                settings: CameraEntry::default(),
            };
            cameras.push(camera);
        }
    }

    cameras.sort_by(|a, b| a.name.cmp(&b.name));
    if let Some(pair) = cameras.windows(2).find(|pair| pair[0].name == pair[1].name) {
        return Err(SuiteError::DuplicateCamera {
            name: pair[0].name.clone(),
            files: format!("{}, {}", pair[0].file.display(), pair[1].file.display()),
        });
    }

    for (name, settings) in &manifest.cameras {
        let Some(camera) = cameras.iter_mut().find(|c| &c.name == name) else {
            return Err(SuiteError::UnknownEntry { kind: "camera", name: name.clone() });
        };
        camera.settings = settings.clone();
    }
    Ok(cameras)
}

fn camera_nodes(doc: &Document) -> Vec<(String, Option<String>)> {
    let mut parents = vec![None; doc.nodes().len()];
    for node in doc.nodes() {
        for child in node.children() {
            parents[child.index()] = Some(node.index());
        }
    }
    doc.nodes()
        .filter(|node| node.camera().is_some())
        .map(|node| {
            let carriers: HashSet<usize> =
                std::iter::successors(Some(node.index()), |&i| parents[i]).collect();
            let clip = doc
                .animations()
                .find(|a| a.channels().any(|c| carriers.contains(&c.target().node().index())))
                .and_then(|a| a.name().map(str::to_string));
            let name =
                node.name().map_or_else(|| format!("GltfNode{}", node.index()), str::to_string);
            (name, clip)
        })
        .collect()
}

// A folder only counts if it holds a map or a panorama, or shares a scene file's name. A
// .gltf's own texture folder is neither.
fn resolve_environments(root: &Path, manifest: &Manifest) -> Result<Vec<Environment>, SuiteError> {
    let mut found: BTreeMap<String, Environment> =
        named_files(root, ENVIRONMENT_DIR, "environment")?
            .into_iter()
            .map(|(name, scene)| {
                let environment =
                    Environment { name: name.clone(), scene: Some(scene), ..Default::default() };
                (name, environment)
            })
            .collect();

    for folder in read_dir(&root.join(ENVIRONMENT_DIR))?.into_iter().filter(|p| p.is_dir()) {
        let Some(name) = folder.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let mut candidate = Environment { name: name.clone(), ..Default::default() };
        candidate.find_maps(root);
        let relative = candidate.folder();
        let mut panoramas: Vec<PathBuf> = files_in(root, &relative.to_string_lossy())?
            .into_iter()
            .filter(|p| has_extension(p, &["hdr", "exr"]))
            .collect();
        if panoramas.len() > 1 {
            let names: Vec<String> = panoramas.iter().map(|p| p.display().to_string()).collect();
            return Err(SuiteError::SeveralPanoramas { folder: relative, found: names.join(", ") });
        }
        candidate.panorama = panoramas.pop();
        let has_content = candidate.panorama.is_some()
            || candidate.skybox.is_some()
            || candidate.diffuse.is_some()
            || candidate.specular.is_some();
        if !has_content && !found.contains_key(&name) {
            continue;
        }
        let environment = found.entry(name.clone()).or_insert_with(|| candidate.clone());
        environment.panorama = candidate.panorama;
        environment.skybox = candidate.skybox;
        environment.diffuse = candidate.diffuse;
        environment.specular = candidate.specular;
        debug!("environment {name:?}: folder found");
    }

    for (name, settings) in &manifest.environments {
        let Some(environment) = found.get_mut(name) else {
            return Err(SuiteError::UnknownEntry { kind: "environment", name: name.clone() });
        };
        environment.settings = settings.clone();
    }

    Ok(found.into_values().collect())
}

fn named_files(
    root: &Path,
    dir: &str,
    kind: &'static str,
) -> Result<Vec<(String, PathBuf)>, SuiteError> {
    let mut found: BTreeMap<String, PathBuf> = BTreeMap::new();
    for file in files_in(root, dir)?.into_iter().filter(|p| is_gltf(p)) {
        let name = file.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        if found.contains_key(&name) {
            return Err(SuiteError::DuplicateName { kind, name });
        }
        debug!("discovered {kind} {name:?}");
        found.insert(name, file);
    }
    Ok(found.into_iter().collect())
}

// Rejects anything that could reach outside the suite folder.
fn relative(path: &Path) -> Result<PathBuf, SuiteError> {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => return Err(SuiteError::UnsafePath(path.to_path_buf())),
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(SuiteError::UnsafePath(path.to_path_buf()));
    }
    Ok(clean)
}

fn existing(root: &Path, path: &Path) -> Result<PathBuf, SuiteError> {
    let clean = relative(path)?;
    if root.join(&clean).is_file() { Ok(clean) } else { Err(SuiteError::Missing(clean)) }
}

fn sole(
    root: &Path,
    dir: &str,
    place: &'static str,
    wanted: impl Fn(&Path) -> bool,
) -> Result<Option<PathBuf>, SuiteError> {
    let mut found: Vec<PathBuf> = files_in(root, dir)?.into_iter().filter(|p| wanted(p)).collect();
    match found.len() {
        0 | 1 => Ok(found.pop()),
        _ => {
            let names: Vec<String> = found.iter().map(|p| p.display().to_string()).collect();
            Err(SuiteError::Ambiguous { place, found: names.join(", ") })
        }
    }
}

// A missing folder is just empty: every sub-folder of a suite is optional.
fn files_in(root: &Path, dir: &str) -> Result<Vec<PathBuf>, SuiteError> {
    let mut files: Vec<PathBuf> = read_dir(&root.join(dir))?
        .into_iter()
        .filter(|p| p.is_file())
        .filter_map(|p| p.file_name().map(|name| Path::new(dir).join(name)))
        .collect();
    files.sort();
    Ok(files)
}

fn read_dir(dir: &Path) -> Result<Vec<PathBuf>, SuiteError> {
    let io_error = |source| SuiteError::Io { path: dir.to_path_buf(), source };
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_error(err)),
    };
    entries.map(|entry| entry.map(|e| e.path()).map_err(io_error)).collect()
}

fn is_gltf(path: &Path) -> bool {
    has_extension(path, &["glb", "gltf"])
}

fn has_extension(path: &Path, wanted: &[&str]) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    wanted.iter().any(|w| ext.eq_ignore_ascii_case(w))
}
