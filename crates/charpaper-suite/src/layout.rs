use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use gltf::Document;
use tracing::debug;
use tracing::warn;

use crate::document::read_document;
use crate::error::SuiteError;
use crate::manifest::Camera;
use crate::manifest::CameraEntry;
use crate::manifest::ClipEntry;
use crate::manifest::ClipMode;
use crate::manifest::Cue;
use crate::manifest::EnvironmentEntry;
use crate::manifest::ExpressionEntry;
use crate::manifest::Gaze;
use crate::manifest::Hotspot;
use crate::manifest::MANIFEST_FILE;
use crate::manifest::Manifest;
use crate::manifest::NO_LUT;
use crate::manifest::Physics;
use crate::manifest::Post;
use crate::manifest::SCHEMA_VERSION;
use crate::manifest::Timeline;
use crate::manifest::Trigger;
use crate::manifest::Wind;

const CLIPS_DIR: &str = "clips";
const EXPRESSIONS_DIR: &str = "expressions";
const SKINS_DIR: &str = "skins";
const CAMERAS_DIR: &str = "cameras";
const LUTS_DIR: &str = "luts";
const SOUNDS_DIR: &str = "sounds";

pub const ORBIT_CAMERA: &str = "orbit";
const ENVIRONMENT_DIR: &str = "environment";
const MUSIC_DIR: &str = "music";
const AMBIENCE_FILE: &str = "ambience.ogg";
const SOUND_EXTENSIONS: &[&str] = &["ogg"];
const ANIMATED_DIR: &str = "animated";

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
    pub timeline: Timeline,
    pub clips: Vec<ClipFile>,
    pub default_clip: Option<String>,
    pub expressions: Vec<ExpressionFile>,
    pub skins: Vec<Skin>,
    pub default_skin: Option<String>,
    pub cameras: Vec<ExportedCamera>,
    pub default_camera: Option<String>,
    pub environments: Vec<Environment>,
    pub default_environment: Option<String>,
    pub luts: Vec<LutFile>,
    pub post: Post,
    pub camera: Camera,
    pub gaze: Option<Gaze>,
    pub hotspots: BTreeMap<String, Hotspot>,
    pub physics: Physics,
}

// A clip is its .json, which every clip has, and the .glb beside it when it keys bones or
// objects.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipFile {
    pub name: String,
    pub json: PathBuf,
    pub glb: Option<PathBuf>,
    pub mode: ClipMode,
    pub label: Option<String>,
    pub gaze: bool,
    pub reaction_only: bool,
    pub cues: Vec<ClipCue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionFile {
    pub name: String,
    pub json: PathBuf,
    pub label: Option<String>,
}

// An environment's animated object or armature, by its object's name.
#[derive(Debug, Clone, PartialEq)]
pub struct Actor {
    pub name: String,
    pub clips: Vec<ClipFile>,
    /// The clip it plays by itself, if suite.toml picks one.
    pub default_clip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipCue {
    pub at: f32,
    pub sound: PathBuf,
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
    // The .json beside a .glb camera file that moves, the file being its clip's .glb. In a file
    // holding several cameras, it moves them all.
    pub clip: Option<PathBuf>,
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
    // In file-name order, which is the order they play in.
    pub music: Vec<PathBuf>,
    pub ambience: Option<PathBuf>,
    // GIFs that play on the scene's materials, keyed by the material each is named after.
    pub animated: BTreeMap<String, PathBuf>,
    pub actors: Vec<Actor>,
    // Its `actors` are moved out into `actors`, so these are only how it looks.
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

    fn find_sounds(&mut self, root: &Path) -> Result<(), SuiteError> {
        let folder = self.folder();
        let ambience = folder.join(AMBIENCE_FILE);
        self.ambience = root.join(&ambience).is_file().then_some(ambience);
        self.music = files_in(root, &folder.join(MUSIC_DIR).to_string_lossy())?
            .into_iter()
            .filter(|p| has_extension(p, SOUND_EXTENSIONS))
            .collect();
        Ok(())
    }

    fn find_animated(&mut self, root: &Path) -> Result<(), SuiteError> {
        let folder = self.folder().join(ANIMATED_DIR);
        self.animated = files_in(root, &folder.to_string_lossy())?
            .into_iter()
            .filter(|p| has_extension(p, &["gif"]))
            .map(|p| (p.file_stem().unwrap_or_default().to_string_lossy().into_owned(), p))
            .collect();
        Ok(())
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
        if manifest.schema != SCHEMA_VERSION {
            return Err(SuiteError::Schema { found: manifest.schema, supported: SCHEMA_VERSION });
        }
        let timeline = manifest.timeline;
        if !(timeline.fps.is_finite() && timeline.fps > 0.0 && timeline.end >= timeline.start) {
            return Err(SuiteError::Timeline(timeline));
        }

        let name = match &manifest.name {
            Some(name) => name.clone(),
            None => root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        };

        let clips = clip_files(root, Path::new(CLIPS_DIR), &manifest.character.clips, "")?;
        let expressions = expression_files(root, &manifest.character.expressions)?;
        let default_clip = match &manifest.character.default_clip {
            Some(name) if !clips.iter().any(|c| &c.name == name) => {
                return Err(SuiteError::UnknownDefault { kind: "clip", name: name.clone() });
            }
            default => default.clone(),
        };
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

        let mut camera = manifest.camera;
        above_zero(&mut camera.f_stop, "camera: `f_stop`");

        for (name, hotspot) in &manifest.hotspots {
            check_hotspot(hotspot, &clips, &expressions, &environments)
                .map_err(|problem| SuiteError::Hotspot { hotspot: name.clone(), problem })?;
        }
        check_physics(&manifest.physics)?;

        Ok(Self {
            root: root.to_path_buf(),
            name,
            model,
            model_is_skin,
            timeline,
            clips,
            default_clip,
            expressions,
            skins,
            default_skin,
            cameras,
            default_camera,
            environments,
            default_environment,
            luts,
            post: manifest.post,
            camera,
            gaze: manifest.gaze,
            hotspots: manifest.hotspots,
            physics: manifest.physics,
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

// Settings for a clip that isn't there are refused, so a typo doesn't go unnoticed. `owner`
// prefixes clip names in that error, for an environment's actors.
fn clip_files(
    root: &Path,
    dir: &Path,
    settings: &BTreeMap<String, ClipEntry>,
    owner: &str,
) -> Result<Vec<ClipFile>, SuiteError> {
    let found = files_in(root, &dir.to_string_lossy())?;
    let mut clips = Vec::new();
    for json in found.iter().filter(|p| has_extension(p, &["json"])) {
        let name = stem(json);
        let glb = Some(json.with_extension("glb")).filter(|path| found.contains(path));
        let entry = settings.get(&name).cloned().unwrap_or_default();
        clips.push(ClipFile {
            json: json.clone(),
            glb,
            mode: entry.mode,
            label: entry.label,
            gaze: entry.gaze.unwrap_or(true),
            reaction_only: entry.reaction_only,
            cues: cues(root, &entry.cues)?,
            name,
        });
    }
    for path in found.iter().filter(|p| is_gltf(p)) {
        if !clips.iter().any(|c| c.glb.as_ref() == Some(path)) {
            warn!("{} has no .json beside it, so it is not a clip", path.display());
        }
    }
    if let Some(name) = settings.keys().find(|name| !clips.iter().any(|c| &c.name == *name)) {
        return Err(SuiteError::UnknownEntry { kind: "clip", name: format!("{owner}{name}") });
    }
    Ok(clips)
}

// Bones are looked up on the armature as it loads; what's checked here is what no armature
// could make right.
fn check_physics(physics: &Physics) -> Result<(), SuiteError> {
    let mut taken = BTreeMap::new();
    for (name, chain) in &physics.chains {
        let fail = |problem: String| SuiteError::Physics { name: name.clone(), problem };
        if chain.bones.is_empty() {
            return Err(fail("has no bones".into()));
        }
        // Two chains on one bone would each turn it their own way.
        for bone in &chain.bones {
            if let Some(other) = taken.insert(bone, name) {
                return Err(fail(format!("shares the bone {bone:?} with {other:?}")));
            }
        }
        let fraction = |value: Option<f32>| value.is_some_and(|v| !(0.0..=1.0).contains(&v));
        if fraction(chain.stiffness) || fraction(chain.drag) {
            return Err(fail("`stiffness` and `drag` go from 0 to 1".into()));
        }
        let negative = |value: f32| !(value.is_finite() && value >= 0.0);
        let sizes = [Some(chain.length), chain.radius, chain.gravity];
        if sizes.into_iter().flatten().any(negative) {
            return Err(fail("`length`, `radius` and `gravity` can't be negative".into()));
        }
    }
    for (name, collider) in &physics.colliders {
        if !(collider.length >= 0.0 && collider.radius > 0.0 && collider.length.is_finite()) {
            let problem = "needs a `radius` above 0, and a `length` of 0 or more".into();
            return Err(SuiteError::Physics { name: name.clone(), problem });
        }
    }
    Ok(())
}

// Clips and expressions that aren't there are refused, as for settings: the add-on writes
// them, so one that doesn't match is a suite out of date. An actor only has to be in one
// environment. An area's armature isn't checked: any armature of an environment can carry
// one, clips or not, and only its file knows which it has.
fn check_hotspot(
    hotspot: &Hotspot,
    clips: &[ClipFile],
    expressions: &[ExpressionFile],
    environments: &[Environment],
) -> Result<(), String> {
    for area in &hotspot.areas {
        let finite = area.length.is_finite() && area.radius.is_finite();
        if !finite || area.length < 0.0 || area.radius <= 0.0 {
            return Err(format!(
                "bone {:?} needs a length of 0 or more and a radius above 0",
                area.bone
            ));
        }
    }
    let find = |actor: &Option<String>, clip: &str| -> Result<&ClipFile, String> {
        let found = match actor {
            None => clips.iter().find(|c| c.name == clip),
            Some(name) => {
                let named = environments.iter().flat_map(|e| &e.actors).filter(|a| &a.name == name);
                named.flat_map(|a| &a.clips).find(|c| c.name == clip)
            }
        };
        let owner = actor.as_deref().unwrap_or("the character");
        found.ok_or_else(|| format!("{owner} has no clip {clip:?}"))
    };
    let pointed = matches!(hotspot.trigger, Trigger::Click | Trigger::Hover);
    if !pointed && !hotspot.areas.is_empty() {
        return Err("only `click` and `hover` hotspots have `areas`".into());
    }
    if pointed && hotspot.after.is_some() {
        return Err("only `idle` and `welcome` hotspots have `after`".into());
    }
    if hotspot.after.is_some_and(|after| !(after.is_finite() && after > 0.0)) {
        return Err("`after` must be more than 0 seconds".into());
    }
    for wanted in &hotspot.clips {
        find(&wanted.actor, &wanted.clip)?;
        if wanted.from == Some(0) {
            return Err(format!("clip {:?}: `from` counts from 1", wanted.clip));
        }
    }
    for wanted in &hotspot.during {
        find(&wanted.actor, &wanted.clip)?;
    }
    if hotspot.trigger == Trigger::Welcome && !hotspot.meanwhile.is_empty() {
        return Err("a `welcome` hotspot isn't held, so it has no `while`".into());
    }
    for (field, list) in [("while", &hotspot.meanwhile), ("then", &hotspot.then)] {
        for (at, wanted) in list.iter().enumerate() {
            let clip = find(&wanted.actor, &wanted.clip)?;
            // What the character plays on and on after is the viewer's pick, so it must be one.
            let hidden = field == "then" && wanted.actor.is_none() && clip.reaction_only;
            if clip.mode == ClipMode::Once || hidden {
                return Err(format!(
                    "`{field}` names {:?}, which doesn't play on and on: it has to be a timeline \
                     or hold clip{}",
                    wanted.clip,
                    if field == "then" { ", and not only for hotspots" } else { "" }
                ));
            }
            if list[..at].iter().any(|other| other.actor == wanted.actor) {
                let owner = wanted.actor.as_deref().unwrap_or("the character");
                return Err(format!("`{field}` names two clips for {owner}"));
            }
        }
    }
    if let Some(name) = &hotspot.expression
        && !expressions.iter().any(|e| &e.name == name)
    {
        return Err(format!("the character has no expression {name:?}"));
    }
    if hotspot.cooldown.is_some_and(|c| !(c.is_finite() && c >= 0.0)) {
        return Err("`cooldown` must be 0 or more seconds".into());
    }
    Ok(())
}

fn cues(root: &Path, cues: &[Cue]) -> Result<Vec<ClipCue>, SuiteError> {
    cues.iter()
        .map(|cue| {
            let file = Path::new(SOUNDS_DIR).join(format!("{}.ogg", cue.sound));
            Ok(ClipCue { at: cue.at, sound: existing(root, &file)? })
        })
        .collect()
}

fn expression_files(
    root: &Path,
    settings: &BTreeMap<String, ExpressionEntry>,
) -> Result<Vec<ExpressionFile>, SuiteError> {
    let expressions: Vec<ExpressionFile> = files_in(root, EXPRESSIONS_DIR)?
        .into_iter()
        .filter(|p| has_extension(p, &["json"]))
        .map(|json| {
            let name = stem(&json);
            let label = settings.get(&name).and_then(|entry| entry.label.clone());
            ExpressionFile { name, json, label }
        })
        .collect();
    if let Some(name) = settings.keys().find(|name| !expressions.iter().any(|e| &e.name == *name)) {
        return Err(SuiteError::UnknownEntry { kind: "expression", name: name.clone() });
    }
    Ok(expressions)
}

fn actors(root: &Path, environment: &mut Environment) -> Result<Vec<Actor>, SuiteError> {
    let settings = std::mem::take(&mut environment.settings.actors);
    let dir = environment.folder().join(CLIPS_DIR);
    let mut actors = Vec::new();
    for folder in read_dir(&root.join(&dir))?.into_iter().filter(|p| p.is_dir()) {
        let name = folder.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let entry = settings.get(&name);
        let owner = format!("{}/{name}/", environment.name);
        let empty = BTreeMap::new();
        let clips = entry.map_or(&empty, |actor| &actor.clips);
        let clips = clip_files(root, &dir.join(&name), clips, &owner)?;
        let default_clip = entry.and_then(|actor| actor.default_clip.clone());
        if let Some(wanted) = &default_clip {
            let clip = clips.iter().find(|clip| &clip.name == wanted);
            if clip.is_none_or(|clip| clip.mode == ClipMode::Once) {
                return Err(SuiteError::UnknownDefault {
                    kind: "timeline or hold clip",
                    name: format!("{owner}{wanted}"),
                });
            }
        }
        actors.push(Actor { name, clips, default_clip });
    }
    actors.sort_by(|a, b| a.name.cmp(&b.name));
    if let Some(name) = settings.keys().find(|name| !actors.iter().any(|a| &a.name == *name)) {
        let name = format!("{}/{name}", environment.name);
        return Err(SuiteError::UnknownEntry { kind: "actor", name });
    }
    Ok(actors)
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}

fn resolve_cameras(root: &Path, manifest: &Manifest) -> Result<Vec<ExportedCamera>, SuiteError> {
    let mut cameras: Vec<ExportedCamera> = Vec::new();
    for (name, file) in named_files(root, CAMERAS_DIR, "camera")? {
        // An unreadable file stays one camera here; the app reports it when it loads the file.
        let found = read_document(&root.join(&file)).map(|d| camera_nodes(&d)).unwrap_or_default();
        let json = file.with_extension("json");
        let clip = (has_extension(&file, &["glb"]) && root.join(&json).is_file()).then_some(json);
        if found.len() <= 1 {
            cameras.push(ExportedCamera {
                name,
                file,
                node: None,
                clip,
                settings: CameraEntry::default(),
            });
            continue;
        }
        for node in found {
            let camera = ExportedCamera {
                name: node.clone(),
                file: file.clone(),
                node: Some(node),
                clip: clip.clone(),
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
        let lens = &mut camera.settings;
        for (key, value) in [
            ("f_stop", &mut lens.f_stop),
            ("focus_distance", &mut lens.focus_distance),
            ("sensor_height_mm", &mut lens.sensor_height_mm),
        ] {
            above_zero(value, &format!("camera {name:?}: `{key}`"));
        }
    }
    Ok(cameras)
}

// Depth of field divides by each of these, so a value that isn't above 0 counts as not given:
// no f-stop leaves depth of field off, and the others fall back to their defaults.
fn above_zero(value: &mut Option<f32>, what: &str) {
    if let Some(v) = *value
        && !(v > 0.0 && v.is_finite())
    {
        warn!("{what} is {v}, but must be above 0, so it counts as not given");
        *value = None;
    }
}

fn camera_nodes(doc: &Document) -> Vec<String> {
    doc.nodes()
        .filter(|node| node.camera().is_some())
        .map(|node| node.name().map_or_else(|| format!("GltfNode{}", node.index()), str::to_string))
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
        candidate.find_sounds(root)?;
        candidate.find_animated(root)?;
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
        environment.music = candidate.music;
        environment.ambience = candidate.ambience;
        environment.animated = candidate.animated;
        debug!("environment {name:?}: folder found");
    }

    for (name, settings) in &manifest.environments {
        let Some(environment) = found.get_mut(name) else {
            return Err(SuiteError::UnknownEntry { kind: "environment", name: name.clone() });
        };
        if let Some(wind) = &settings.wind {
            check_wind(wind)
                .map_err(|problem| SuiteError::Wind { environment: name.clone(), problem })?;
        }
        environment.settings = settings.clone();
    }
    for environment in found.values_mut() {
        environment.actors = actors(root, environment)?;
    }

    Ok(found.into_values().collect())
}

fn check_wind(wind: &Wind) -> Result<(), String> {
    let direction = wind.direction;
    if !direction.iter().all(|v| v.is_finite()) || direction.iter().all(|v| *v == 0.0) {
        return Err("needs a `direction` that points somewhere".into());
    }
    if !(wind.strength.is_finite() && wind.strength >= 0.0) {
        return Err("can't have a negative `strength`".into());
    }
    if wind.gusts.is_some_and(|gusts| !(0.0..=1.0).contains(&gusts)) {
        return Err("`gusts` go from 0 to 1".into());
    }
    Ok(())
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
/// What the settings panel calls a music track: its file name, less the `01 - ` the Blender
/// add-on puts in front to keep the playlist's order.
pub fn track_title(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let digits = stem.bytes().take_while(u8::is_ascii_digit).count();
    match stem[digits..].strip_prefix(" - ") {
        Some(title) if digits > 0 && !title.is_empty() => title.to_owned(),
        _ => stem,
    }
}

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
