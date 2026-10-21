use std::collections::BTreeMap;
use std::error::Error;

use bevy::ecs::system::SystemParam;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::prelude::*;
use charpaper_settings::Picks;
use charpaper_suite::Suite;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::camera::ShownOrbit;
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
use crate::look::EditedLooks;
use crate::stage::StageOf;
use crate::state::CharacterState;
use crate::state::picks_of;

// Which suite each stage shows, and swapping it while the app runs. A missing or broken suite
// is logged, not fatal: a wallpaper with nothing on it beats one that never starts.
pub(crate) struct SuitePlugin;

impl Plugin for SuitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AvailableSuites>()
            .add_systems(Startup, discover_suites)
            .add_systems(Update, switch_suite.in_set(SceneSet::Switch));
    }
}

// A newtype because Suite lives in a Bevy-free crate.
#[derive(Component, Deref)]
pub struct ActiveSuite(pub Suite);

pub(crate) fn folder_of(suite: &Suite) -> String {
    suite.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

impl ActiveSuite {
    pub fn folder(&self) -> String {
        folder_of(self)
    }

    pub(crate) fn remembered(&self, remembered: &Remembered) -> Picks {
        remembered.0.get(&self.folder()).cloned().unwrap_or_default()
    }
}

#[derive(Resource, Default, Debug)]
pub struct AvailableSuites(pub Vec<String>);

// A stage's choices per suite, updated on leaving one, so coming back picks up where it was
// left.
#[derive(Component, Default)]
pub(crate) struct Remembered(pub BTreeMap<String, Picks>);

// Set even when loading failed, so a broken suite is tried once, not every frame.
#[derive(Component, Default)]
pub(crate) struct LoadedSuite(Option<String>);

pub(crate) fn discover_suites(config: Res<SceneConfig>, mut available: ResMut<AvailableSuites>) {
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
}

// The suite asked for, or the first there is.
pub(crate) fn starting_suite(
    config: &SceneConfig,
    available: &AvailableSuites,
    asked: Option<String>,
) -> Option<String> {
    let dir = &config.characters_dir;
    let first = available.0.first().cloned();
    let suite = match &asked {
        Some(name) if available.0.contains(name) => Some(name.clone()),
        Some(name) => {
            error!("no suite named {name:?} in {}; using the first instead", dir.display());
            first
        }
        None => first,
    };
    if suite.is_none() {
        warn!("no character suites in {}", dir.display());
    }
    suite
}

pub(crate) fn switch_suite(
    mut commands: Commands,
    config: Res<SceneConfig>,
    mut stages: Query<(
        Entity,
        &ShownOrbit,
        &mut LoadedSuite,
        &mut CharacterState,
        Option<&ActiveSuite>,
        Option<&ActiveLook>,
        &mut Remembered,
        &mut EditedLooks,
    )>,
    mut teardown: Teardown,
) {
    for (stage, orbit, mut loaded, mut state, active, look, mut remembered, mut edited) in
        &mut stages
    {
        if loaded.0 == state.suite {
            continue;
        }

        if let Some(old) = active {
            remembered.0.entry(old.folder()).or_default().merge(picks_of(&state, orbit));
            if let Some(look) = look {
                edited.keep(old, look);
            }
            teardown.run(&mut commands, stage);
            info!("left suite {:?}", old.name);
        }
        *state = CharacterState { suite: state.suite.clone(), ..default() };
        loaded.0 = state.suite.clone();

        let Some(name) = &state.suite else {
            continue;
        };
        let root = config.characters_dir.join(name);
        let suite = match Suite::load(&root) {
            Ok(suite) => suite,
            Err(err) => {
                error!("cannot load suite at {}: {}", root.display(), chain(&err));
                continue;
            }
        };

        info!("showing suite {:?} from {}", suite.name, root.display());
        let look = edited.look_for(&suite);
        commands.entity(stage).insert((ActiveLook(look), ActiveSuite(suite)));
    }
}

#[derive(SystemParam)]
pub(crate) struct Teardown<'w, 's> {
    characters: Query<'w, 's, (Entity, &'static ChildOf), With<Character>>,
    cameras: Query<'w, 's, Entity, With<SceneCamera>>,
    stage_of: StageOf<'w, 's>,
    shown: Query<
        'w,
        's,
        (
            &'static mut ShownSkin,
            &'static mut ShownRig,
            &'static mut ShownEnvironment,
            &'static mut Baking,
            &'static mut SkinObjects,
            &'static mut Expressions,
        ),
    >,
}

impl Teardown<'_, '_> {
    fn run(&mut self, commands: &mut Commands, stage: Entity) {
        for (entity, parent) in &self.characters {
            if parent.parent() == stage {
                commands.entity(entity).despawn();
            }
        }
        for camera in &self.cameras {
            if self.stage_of.get(camera) == Some(stage) {
                commands.entity(camera).remove::<(Skybox, EnvironmentMapLight)>();
            }
        }
        let Ok((mut skin, mut rig, mut environment, mut baking, mut objects, mut expressions)) =
            self.shown.get_mut(stage)
        else {
            return;
        };
        for root in [rig.root.take(), environment.root.take()].into_iter().flatten() {
            commands.entity(root).despawn();
        }
        *skin = default();
        *rig = default();
        *environment = default();
        baking.0.clear();
        objects.0.clear();
        expressions.0.clear();
        commands.entity(stage).remove::<(ActiveSuite, ActiveLook, CharacterClips, PendingClips)>();
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
