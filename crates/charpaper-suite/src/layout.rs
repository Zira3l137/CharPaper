//! Turns a folder plus its manifest into a [`Suite`]: every path checked and
//! every blank the folder layout can fill, filled.
//!
//! Paths in a [`Suite`] stay relative to [`Suite::root`], because that is the
//! form an asset source wants later.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use tracing::debug;

use crate::error::SuiteError;
use crate::manifest::Camera;
use crate::manifest::Lighting;
use crate::manifest::MANIFEST_FILE;
use crate::manifest::Manifest;
use crate::manifest::PlayMode;
use crate::manifest::Post;
use crate::manifest::SCHEMA_VERSION;

const ANIMATIONS_DIR: &str = "animations";
const SKINS_DIR: &str = "skins";
const CAMERAS_DIR: &str = "cameras";

/// The camera the app always has, which the user drags around the character.
/// An exported camera cannot take this name.
pub const ORBIT_CAMERA: &str = "orbit";
const ENVIRONMENT_DIR: &str = "environment";

#[derive(Debug, Clone)]
pub struct Suite {
    pub root: PathBuf,
    pub name: String,
    pub model: PathBuf,
    /// Sorted by path.
    pub animations: Vec<AnimationFile>,
    /// Sorted by name.
    pub skins: Vec<Skin>,
    pub default_skin: Option<String>,
    pub default_animation: Option<String>,
    /// Sorted by name.
    pub cameras: Vec<ExportedCamera>,
    /// `None` is the orbit camera.
    pub default_camera: Option<String>,
    pub environment: Environment,
    pub lighting: Lighting,
    pub post: Post,
    pub camera: Camera,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationFile {
    pub path: PathBuf,
    pub clips: ClipSet,
}

/// A file is either taken whole or described entry by entry; never both. Once
/// the manifest mentions a file, only the clips it lists are used, which is
/// how you hide helper clips that happen to live in the same file.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipSet {
    /// Every named clip, under its own name, looping.
    All,
    Listed(Vec<ClipBinding>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipBinding {
    pub name: String,
    pub clip: Option<String>,
    pub mode: PlayMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    pub name: String,
    pub file: PathBuf,
}

/// A camera made in Blender, along with at most one clip moving it.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportedCamera {
    pub name: String,
    pub file: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    pub scene: Option<PathBuf>,
    pub skybox: Option<PathBuf>,
    pub skybox_brightness: Option<f32>,
}

impl Suite {
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

        let model = match &manifest.character.model {
            Some(path) => existing(root, path)?,
            None => sole(root, "", "the suite folder", is_gltf)?.ok_or(SuiteError::NoModel)?,
        };

        let animations = resolve_animations(root, &manifest)?;
        let skins: Vec<Skin> = named_files(root, SKINS_DIR, "skin")?
            .into_iter()
            .map(|(name, file)| Skin { name, file })
            .collect();

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

        let cameras: Vec<ExportedCamera> = named_files(root, CAMERAS_DIR, "camera")?
            .into_iter()
            .map(|(name, file)| ExportedCamera { name, file })
            .collect();
        if cameras.iter().any(|c| c.name == ORBIT_CAMERA) {
            return Err(SuiteError::ReservedName { kind: "camera", name: ORBIT_CAMERA.into() });
        }

        let default_camera = match manifest.camera.default.as_deref() {
            None | Some(ORBIT_CAMERA) => None,
            Some(name) if !cameras.iter().any(|c| c.name == name) => {
                return Err(SuiteError::UnknownDefault { kind: "camera", name: name.into() });
            }
            Some(name) => Some(name.to_string()),
        };

        let environment = Environment {
            scene: match &manifest.environment.scene {
                Some(path) => Some(existing(root, path)?),
                None => sole(root, ENVIRONMENT_DIR, "`environment/`", is_gltf)?,
            },
            skybox: match &manifest.environment.skybox {
                Some(path) => Some(existing(root, path)?),
                None => {
                    sole(root, ENVIRONMENT_DIR, "`environment/`", |p| has_extension(p, &["ktx2"]))?
                }
            },
            skybox_brightness: manifest.environment.skybox_brightness,
        };

        Ok(Self {
            root: root.to_path_buf(),
            name,
            model,
            animations,
            skins,
            default_skin,
            default_animation: manifest.character.default_animation,
            cameras,
            default_camera,
            environment,
            lighting: manifest.lighting,
            post: manifest.post,
            camera: manifest.camera,
        })
    }

    pub fn absolute(&self, relative: &Path) -> PathBuf {
        self.root.join(relative)
    }
}

/// Folders directly inside `dir` that contain a `suite.toml`, sorted.
///
/// Whether each one actually loads is left to [`Suite::load`], so one broken
/// suite cannot hide the others from a picker.
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
        let binding =
            ClipBinding { name: name.clone(), clip: entry.clip.clone(), mode: entry.mode };
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

/// Every .glb/.gltf directly in `dir`, named after its file and sorted by
/// name. Sub-folders are left alone, so a `.gltf` can keep its `.bin` and
/// textures in one.
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

/// Rejects anything that could reach outside the suite folder. The asset
/// server would refuse it too, but only at runtime and with a vaguer message.
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

/// Files directly inside `root/dir`, relative to `root`, sorted. A missing
/// folder is simply empty: every sub-folder of a suite is optional.
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
