//! Picks which character suite to show and reads its description.
//!
//! Reading a suite never touches its glTF files, so this runs synchronously at
//! startup. A missing or broken suite is logged, not fatal: a wallpaper with
//! nothing on it beats one that never starts.

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use bevy::asset::AssetPath;
use bevy::gltf::GltfLoaderSettings;
use bevy::prelude::*;
use charpaper_suite::Severity;
use charpaper_suite::Suite;

use crate::CHARACTERS_SOURCE;
use crate::Picks;
use crate::SceneConfig;

/// The suite being shown. Absent when no suite could be loaded.
///
/// A newtype because `Suite` lives in a Bevy-free crate and cannot derive
/// `Resource` itself.
#[derive(Resource, Deref)]
pub struct ActiveSuite(pub Suite);

impl ActiveSuite {
    /// The suite's folder name: how `--suite` names it, and how its
    /// remembered choices are keyed.
    pub fn folder(&self) -> String {
        self.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub(crate) fn remembered(&self, config: &SceneConfig) -> Picks {
        config.remembered.get(&self.folder()).cloned().unwrap_or_default()
    }
}

pub(crate) fn select_suite(mut commands: Commands, config: Res<SceneConfig>) {
    let Some(root) = choose(&config.characters_dir, config.suite.as_deref()) else {
        return;
    };

    let suite = match Suite::load(&root) {
        Ok(suite) => suite,
        Err(err) => {
            error!("cannot load suite at {}: {}", root.display(), chain(&err));
            return;
        }
    };

    for finding in charpaper_suite::inspect(&suite).findings {
        let file = finding.file.map(|f| format!("{}: ", f.display())).unwrap_or_default();
        match finding.severity {
            Severity::Error => error!("suite {:?}: {file}{}", suite.name, finding.message),
            Severity::Warning => warn!("suite {:?}: {file}{}", suite.name, finding.message),
        }
    }

    info!("showing suite {:?} from {}", suite.name, root.display());
    commands.insert_resource(ActiveSuite(suite));
}

/// `characters://<suite folder>/<file>`, for a path relative to the suite.
pub(crate) fn asset_path(suite: &Suite, file: &Path) -> AssetPath<'static> {
    let folder = suite.root.file_name().unwrap_or_default();
    AssetPath::from_path_buf(Path::new(folder).join(file)).with_source(CHARACTERS_SOURCE)
}

/// Scene 0 rather than the file's default scene: Bevy has no label for the
/// default one, and Blender always exports the active scene as scene 0.
///
/// Cameras are never loaded: exported cameras have their own folder. Lights
/// are only wanted from the environment. Lighting otherwise belongs to
/// `suite.toml`, and a stray lamp exported with a skin would light the scene
/// only while that skin is loaded.
pub(crate) fn load_scene(
    assets: &AssetServer,
    suite: &Suite,
    file: &Path,
    lights: bool,
) -> Handle<WorldAsset> {
    assets
        .load_builder()
        .with_settings(move |s: &mut GltfLoaderSettings| {
            s.load_cameras = false;
            s.load_lights = lights;
        })
        .load(GltfAssetLabel::Scene(0).from_asset(asset_path(suite, file)))
}

fn choose(dir: &Path, wanted: Option<&str>) -> Option<PathBuf> {
    let found = match charpaper_suite::discover(dir) {
        Ok(found) => found,
        Err(err) => {
            error!("cannot list character suites: {}", chain(&err));
            return None;
        }
    };

    let names: Vec<String> =
        found.iter().filter_map(|p| p.file_name()).map(|n| n.to_string_lossy().into()).collect();
    debug!("character suites in {}: {names:?}", dir.display());

    match wanted {
        Some(name) => {
            let chosen = found.into_iter().find(|p| p.file_name().is_some_and(|n| n == name));
            if chosen.is_none() {
                error!("no suite named {name:?} in {} (found: {names:?})", dir.display());
            }
            chosen
        }
        None => {
            let first = found.into_iter().next();
            match &first {
                Some(path) => info!("no suite chosen; using the first, {}", path.display()),
                None => warn!("no character suites in {}", dir.display()),
            }
            first
        }
    }
}

/// `SuiteError` keeps details such as the TOML line and column in its source
/// chain, which `Display` alone would drop.
fn chain(err: &dyn Error) -> String {
    let mut text = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        text.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    text
}
