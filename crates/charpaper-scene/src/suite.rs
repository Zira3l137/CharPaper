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

use crate::ARMATURE_SOURCE;
use crate::CHARACTERS_SOURCE;
use crate::CharacterClips;
use crate::Expressions;
use crate::OrbitCamera;
use crate::Picks;
use crate::SceneConfig;
use crate::animation::PendingClips;
use crate::cameras::ShownRig;
use crate::character::Character;
use crate::character::CharacterState;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::environment::Baking;
use crate::environment::ShownEnvironment;
use crate::look::ActiveLook;

// A newtype because Suite lives in a Bevy-free crate.
#[derive(Resource, Deref)]
pub struct ActiveSuite(pub Suite);

impl ActiveSuite {
    pub fn folder(&self) -> String {
        self.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub(crate) fn remembered(&self, remembered: &Remembered) -> Picks {
        remembered.0.get(&self.folder()).cloned().unwrap_or_default()
    }
}

#[derive(Resource, Default, Debug)]
pub struct AvailableSuites(pub Vec<String>);

// Choices per suite, updated on leaving one, so coming back picks up where it was left.
#[derive(Resource, Default)]
pub(crate) struct Remembered(pub BTreeMap<String, Picks>);

// Set even when loading failed, so a broken suite is tried once, not every frame.
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

#[derive(SystemParam)]
pub(crate) struct Teardown<'w, 's> {
    characters: Query<'w, 's, Entity, With<Character>>,
    camera: Query<'w, 's, Entity, With<OrbitCamera>>,
    skin: ResMut<'w, ShownSkin>,
    rig: ResMut<'w, ShownRig>,
    environment: ResMut<'w, ShownEnvironment>,
    baking: ResMut<'w, Baking>,
    objects: ResMut<'w, SkinObjects>,
    expressions: ResMut<'w, Expressions>,
}

impl Teardown<'_, '_> {
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
        self.objects.0.clear();
        self.expressions.0.clear();
        commands.remove_resource::<ActiveSuite>();
        commands.remove_resource::<ActiveLook>();
        commands.remove_resource::<CharacterClips>();
        commands.remove_resource::<PendingClips>();
    }
}

pub(crate) fn asset_path(suite: &Suite, file: &Path) -> AssetPath<'static> {
    let folder = suite.root.file_name().unwrap_or_default();
    AssetPath::from_path_buf(Path::new(folder).join(file)).with_source(CHARACTERS_SOURCE)
}

// Once uploaded, Bevy drops the RAM copy of meshes and textures. Bounding boxes survive
// but shape key names don't; expressions read those from the file instead.
pub(crate) const GPU_ONLY: RenderAssetUsages = RenderAssetUsages::RENDER_WORLD;

pub(crate) fn load_gltf(assets: &AssetServer, suite: &Suite, file: &Path) -> Handle<Gltf> {
    assets
        .load_builder()
        .with_settings(|s: &mut GltfLoaderSettings| {
            s.load_cameras = false;
            s.load_lights = false;
            s.load_meshes = GPU_ONLY;
            s.load_materials = GPU_ONLY;
        })
        .load(asset_path(suite, file))
}

// Scene 0, not the default scene: Bevy has no label for the default one, and Blender
// exports the active scene as scene 0. An armature borrowed from a skin skips that skin's
// meshes, materials and clips, which the skin shows itself when worn.
pub(crate) fn load_armature(assets: &AssetServer, suite: &Suite) -> Handle<WorldAsset> {
    let borrowed = suite.model_is_skin;
    let path = asset_path(suite, &suite.model).with_source(ARMATURE_SOURCE);
    assets
        .load_builder()
        .with_settings(move |s: &mut GltfLoaderSettings| {
            s.load_cameras = false;
            s.load_lights = false;
            if borrowed {
                s.load_meshes = RenderAssetUsages::empty();
                s.load_materials = RenderAssetUsages::empty();
                s.load_animations = false;
            } else {
                s.load_meshes = GPU_ONLY;
                s.load_materials = GPU_ONLY;
            }
        })
        .load(GltfAssetLabel::Scene(0).from_asset(path))
}

// Display alone drops details like the TOML line and column.
fn chain(err: &dyn Error) -> String {
    let mut text = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        text.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    text
}
