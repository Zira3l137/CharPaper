//! Which character suite is shown, and swapping it for another while the app
//! runs.
//!
//! Reading a suite never touches its glTF files, so it happens synchronously
//! on the frame the suite is picked. A missing or broken suite is logged, not
//! fatal: a wallpaper with nothing on it beats one that never starts.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use bevy::asset::AssetPath;
use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::gltf::GltfLoaderSettings;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::prelude::*;
use charpaper_suite::Severity;
use charpaper_suite::Suite;

use crate::CHARACTERS_SOURCE;
use crate::CharacterClips;
use crate::OrbitCamera;
use crate::Picks;
use crate::SceneConfig;
use crate::animation::PendingClips;
use crate::cameras::ShownRig;
use crate::character::Character;
use crate::character::CharacterState;
use crate::character::ShownSkin;
use crate::environment::Baking;
use crate::environment::ShownEnvironment;
use crate::look::ActiveLook;

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

    pub(crate) fn remembered(&self, remembered: &Remembered) -> Picks {
        remembered.0.get(&self.folder()).cloned().unwrap_or_default()
    }
}

/// Every suite in the characters folder, by folder name, in name order.
#[derive(Resource, Default, Debug)]
pub struct AvailableSuites(pub Vec<String>);

/// The viewer's choices per suite while the app runs: loaded from the saved
/// state at startup, and updated whenever the viewer leaves a suite, so
/// coming back to it picks up where they left off.
#[derive(Resource, Default)]
pub(crate) struct Remembered(pub BTreeMap<String, Picks>);

/// The suite whose loading has been attempted, as opposed to
/// [`CharacterState::suite`], the one asked for. Set even when loading
/// failed, so a broken suite is tried once rather than every frame.
#[derive(Resource, Default)]
pub(crate) struct LoadedSuite(Option<String>);

pub(crate) fn discover_suites(
    config: Res<SceneConfig>,
    mut available: ResMut<AvailableSuites>,
    mut state: ResMut<CharacterState>,
) {
    let dir = &config.characters_dir;
    let found = match charpaper_suite::discover(dir) {
        Ok(found) => found,
        Err(err) => {
            error!("cannot list character suites: {}", chain(&err));
            return;
        }
    };
    available.0 =
        found.iter().filter_map(|p| p.file_name()).map(|n| n.to_string_lossy().into()).collect();
    debug!("character suites in {}: {:?}", dir.display(), available.0);

    let first = available.0.first().cloned();
    state.suite = match &config.suite {
        Some(name) if available.0.contains(name) => Some(name.clone()),
        Some(name) => {
            error!("no suite named {name:?} in {}; using the first instead", dir.display());
            first
        }
        None => first,
    };
    if state.suite.is_none() {
        warn!("no character suites in {}", dir.display());
    }
}

/// Swaps the whole suite: everything the old one spawned or loaded goes, which
/// frees it, and the new one is read from disk. The systems that fill a suite
/// in (the character, its clips, camera and environment) then run because
/// [`ActiveSuite`] has been added anew.
pub(crate) fn switch_suite(
    mut commands: Commands,
    config: Res<SceneConfig>,
    mut loaded: ResMut<LoadedSuite>,
    mut state: ResMut<CharacterState>,
    active: Option<Res<ActiveSuite>>,
    mut remembered: ResMut<Remembered>,
    mut teardown: Teardown,
) {
    if loaded.0 == state.suite {
        return;
    }

    if let Some(old) = active {
        remembered.0.entry(old.folder()).or_default().merge(Picks::from_state(&state));
        teardown.run(&mut commands);
        info!("left suite {:?}", old.name);
    }
    *state = CharacterState { suite: state.suite.clone(), ..default() };
    loaded.0 = state.suite.clone();

    let Some(name) = &state.suite else {
        return;
    };
    let root = config.characters_dir.join(name);
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
    commands.insert_resource(ActiveLook(suite.look()));
    commands.insert_resource(ActiveSuite(suite));
}

/// Everything a suite leaves behind, gathered so [`switch_suite`] stays
/// readable.
#[derive(SystemParam)]
pub(crate) struct Teardown<'w, 's> {
    characters: Query<'w, 's, Entity, With<Character>>,
    camera: Query<'w, 's, Entity, With<OrbitCamera>>,
    skin: ResMut<'w, ShownSkin>,
    rig: ResMut<'w, ShownRig>,
    environment: ResMut<'w, ShownEnvironment>,
    baking: ResMut<'w, Baking>,
}

impl Teardown<'_, '_> {
    /// Despawning the character takes the armature, the skin and the pieces
    /// the skin moved onto the armature with it. Dropping the pending bakes
    /// cancels them. Removing the resources drops the last handles to the
    /// clips and anything else the suite loaded.
    fn run(&mut self, commands: &mut Commands) {
        for entity in &self.characters {
            commands.entity(entity).despawn();
        }
        for root in [self.rig.root.take(), self.environment.root.take()].into_iter().flatten() {
            commands.entity(root).despawn();
        }
        for camera in &self.camera {
            commands.entity(camera).remove::<(Skybox, EnvironmentMapLight)>();
        }
        *self.skin = default();
        *self.rig = default();
        *self.environment = default();
        self.baking.0.clear();
        commands.remove_resource::<ActiveSuite>();
        commands.remove_resource::<ActiveLook>();
        commands.remove_resource::<CharacterClips>();
        commands.remove_resource::<PendingClips>();
    }
}

/// `characters://<suite folder>/<file>`, for a path relative to the suite.
pub(crate) fn asset_path(suite: &Suite, file: &Path) -> AssetPath<'static> {
    let folder = suite.root.file_name().unwrap_or_default();
    AssetPath::from_path_buf(Path::new(folder).join(file)).with_source(CHARACTERS_SOURCE)
}

/// Meshes and textures are kept on the GPU only: once uploaded, Bevy drops
/// the copy in RAM, often the larger half of a model's footprint. Nothing here
/// reads them back. Two consequences: a mesh without a CPU copy gets no
/// bounding box, so it is never culled (harmless for one character on
/// screen), and skinned meshes keep the joint bounds the loader computed.
pub(crate) const GPU_ONLY: RenderAssetUsages = RenderAssetUsages::RENDER_WORLD;

/// A character file's scene: the model or a skin. Scene 0 rather than the
/// file's default scene, since Bevy has no label for the default one and
/// Blender always exports the active scene as scene 0.
///
/// Cameras and lights are skipped. Cameras have their own folder, and all
/// lighting comes from the environment; a stray lamp exported with a skin
/// would otherwise light the scene only while that skin is worn.
pub(crate) fn load_scene(assets: &AssetServer, suite: &Suite, file: &Path) -> Handle<WorldAsset> {
    assets
        .load_builder()
        .with_settings(|s: &mut GltfLoaderSettings| {
            s.load_cameras = false;
            s.load_lights = false;
            s.load_meshes = GPU_ONLY;
            s.load_materials = GPU_ONLY;
        })
        .load(GltfAssetLabel::Scene(0).from_asset(asset_path(suite, file)))
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
