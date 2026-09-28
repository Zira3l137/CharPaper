use std::collections::BTreeMap;
use std::error::Error;

use bevy::ecs::system::SystemParam;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::prelude::*;
use charpaper_suite::Severity;
use charpaper_suite::Suite;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::camera::ShownRig;
use crate::character::Character;
use crate::character::CharacterClips;
use crate::character::Expressions;
use crate::character::PendingClips;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::config::SceneConfig;
use crate::environment::Baking;
use crate::environment::ShownEnvironment;
use crate::look::ActiveLook;
use crate::state::CharacterState;
use crate::state::Picks;

// Which suite is shown, and swapping it while the app runs. A missing or broken suite is
// logged, not fatal: a wallpaper with nothing on it beats one that never starts.
pub(crate) struct SuitePlugin;

impl Plugin for SuitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AvailableSuites>()
            .init_resource::<LoadedSuite>()
            .add_systems(Startup, discover_suites)
            .add_systems(Update, switch_suite.in_set(SceneSet::Switch));
    }
}

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
    camera: Query<'w, 's, Entity, With<SceneCamera>>,
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
