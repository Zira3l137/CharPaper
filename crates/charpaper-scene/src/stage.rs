use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bevy::app::HierarchyPropagatePlugin;
use bevy::app::Propagate;
use bevy::app::PropagateSet;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::visibility::VisibilitySystems;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::SceneSet;
use crate::camera::ShownRig;
use crate::camera::view_bundle;
use crate::character::CueClock;
use crate::character::Expressions;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::config::SceneConfig;
use crate::environment::Baking;
use crate::environment::ShownEnvironment;
use crate::gaze::CursorPosition;
use crate::gaze::GazeRig;
use crate::gaze::GazeSettings;
use crate::look::LookBackup;
use crate::lut::ShownLut;
use crate::screens::Screen;
use crate::screens::ShowsStage;
use crate::screens::StageId;
use crate::soundscape::Soundscape;
use crate::state::CharacterState;
use crate::state::Picks;
use crate::suite::ActiveSuite;
use crate::suite::AvailableSuites;
use crate::suite::LoadedSuite;
use crate::suite::Remembered;
use crate::suite::starting_suite;

// Stages stand this far apart, so a fog box of one, which every camera draws, is a speck far
// out of the others' sight. Lights and meshes keep to their own stage by render layer.
const STAGE_SPACING: f32 = 5_000.0;

// One scene running on its own: a suite with its character, environment and view, and what
// the viewer picked for them. Everything a stage spawns hangs under it, so despawning it takes
// all of that away. A stage runs while a screen shows it.
#[derive(Component)]
#[require(
    Transform,
    Visibility,
    CharacterState,
    GazeSettings,
    Remembered,
    LoadedSuite,
    LookBackup,
    ShownSkin,
    SkinObjects,
    Expressions,
    CueClock,
    ShownRig,
    ShownEnvironment,
    Baking,
    ShownLut,
    Soundscape,
    GazeRig
)]
pub struct Stage;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageKey(pub StageId);

// The stage the viewer is pointing at: the one under the cursor, which the panels show and
// change.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct FocusedStage(pub Option<Entity>);

// What a stage keeps between runs, and while none of its monitors is connected.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct StageSeed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<String>,
    pub gaze: GazeSettings,
    // The picks per suite, by the suite's folder.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub suites: BTreeMap<String, Picks>,
}

// Every stage's seed, filled from state.toml and brought up to date as a stage goes away.
#[derive(Resource, Default)]
pub struct StageSeeds(pub(crate) BTreeMap<StageId, StageSeed>);

pub(crate) struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FocusedStage>()
            .add_plugins(HierarchyPropagatePlugin::<RenderLayers>::new(PostUpdate))
            .configure_sets(
                PostUpdate,
                PropagateSet::<RenderLayers>::default().before(VisibilitySystems::CheckVisibility),
            )
            .add_observer(remember_stage)
            .add_systems(Update, (sync_stages, follow_cursor).chain().before(SceneSet::Switch));
    }
}

pub(crate) fn sync_stages(
    mut commands: Commands,
    config: Res<SceneConfig>,
    available: Res<AvailableSuites>,
    seeds: Res<StageSeeds>,
    screens: Query<&ShowsStage>,
    stages: Query<(Entity, &StageKey)>,
) {
    let wanted: BTreeSet<StageId> = screens.iter().map(|shows| shows.0).collect();
    for (stage, key) in &stages {
        if !wanted.contains(&key.0) {
            info!("stage {} rests: no monitor shows it", key.0);
            commands.entity(stage).despawn();
        }
    }
    for id in wanted {
        if stages.iter().any(|(_, key)| key.0 == id) {
            continue;
        }
        let seed = seeds.0.get(&id).cloned().unwrap_or_default();
        // The --suite flag picks stage A's suite.
        let asked = if id == StageId::FIRST { config.suite.clone() } else { None };
        let suite = starting_suite(&config, &available, asked.or(seed.suite));
        let stage = commands
            .spawn((
                Name::new(format!("Stage {id}")),
                Stage,
                StageKey(id),
                CharacterState { suite, ..default() },
                seed.gaze,
                Remembered(seed.suites),
                Transform::from_translation(Vec3::X * STAGE_SPACING * id.index() as f32),
                // Layer 0 is everything outside the stages, which no stage camera sees.
                Propagate(RenderLayers::layer(id.index() + 1)),
            ))
            .id();
        commands.spawn((view_bundle(), ChildOf(stage)));
        info!("stage {id} starts");
    }
}

// A stage going away leaves its seed behind, so it picks up where it was when it comes back.
fn remember_stage(
    removed: On<Remove, Stage>,
    stages: Query<StageMemory>,
    mut seeds: ResMut<StageSeeds>,
) {
    if let Ok((key, state, gaze, remembered, suite)) = stages.get(removed.entity) {
        seeds.0.insert(key.0, seed_of(state, gaze, remembered, suite));
    }
}

type StageMemory = (
    &'static StageKey,
    &'static CharacterState,
    &'static GazeSettings,
    &'static Remembered,
    Option<&'static ActiveSuite>,
);

fn seed_of(
    state: &CharacterState,
    gaze: &GazeSettings,
    remembered: &Remembered,
    suite: Option<&ActiveSuite>,
) -> StageSeed {
    let mut suites = remembered.0.clone();
    if let Some(suite) = suite {
        suites.entry(suite.folder()).or_default().merge(Picks::from_state(state));
    }
    StageSeed { suite: state.suite.clone(), gaze: gaze.clone(), suites }
}

// What the app saves: every stage's seed, running or not.
#[derive(SystemParam)]
pub struct StageMemories<'w, 's> {
    seeds: Res<'w, StageSeeds>,
    stages: Query<'w, 's, StageMemory>,
}

impl StageMemories<'_, '_> {
    pub fn all(&self) -> BTreeMap<StageId, StageSeed> {
        let mut all = self.seeds.0.clone();
        for (key, state, gaze, remembered, suite) in &self.stages {
            all.insert(key.0, seed_of(state, gaze, remembered, suite));
        }
        all
    }
}

pub fn stage_memory_changed(
    seeds: Res<StageSeeds>,
    stages: Query<
        (),
        (
            With<Stage>,
            // Remembered only changes along with CharacterState, on a switch of suite.
            Or<(Changed<CharacterState>, Changed<GazeSettings>, Added<ActiveSuite>)>,
        ),
    >,
) -> bool {
    seeds.is_changed() || !stages.is_empty()
}

// The cursor over a screen focuses the stage it shows. Off the screens, the focus stays.
fn follow_cursor(
    cursor: Res<CursorPosition>,
    screens: Query<(&Screen, &ShowsStage)>,
    stages: Query<(Entity, &StageKey)>,
    mut focused: ResMut<FocusedStage>,
) {
    let stage_of = |id: StageId| stages.iter().find(|(_, key)| key.0 == id).map(|(e, _)| e);
    let pointed = cursor
        .0
        .and_then(|at| screens.iter().find(|(screen, _)| screen.window == at.window))
        .and_then(|(_, shows)| stage_of(shows.0));
    let kept = focused.0.filter(|&stage| stages.contains(stage));
    let first = || stages.iter().min_by_key(|(_, key)| key.0).map(|(stage, _)| stage);
    focused.set_if_neq(FocusedStage(pointed.or(kept).or_else(first)));
}

// The stage an entity belongs to: the nearest one above it, or itself.
#[derive(SystemParam)]
pub(crate) struct StageOf<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    stages: Query<'w, 's, (), With<Stage>>,
}

impl StageOf<'_, '_> {
    pub fn get(&self, entity: Entity) -> Option<Entity> {
        std::iter::once(entity)
            .chain(self.parents.iter_ancestors(entity))
            .find(|&e| self.stages.contains(e))
    }
}

// What the panel reads: one of the focused stage's components.
#[derive(SystemParam)]
pub struct Focused<'w, 's, T: Component> {
    focused: Res<'w, FocusedStage>,
    stages: Query<'w, 's, &'static T, With<Stage>>,
}

impl<T: Component> Focused<'_, '_, T> {
    pub fn get(&self) -> Option<&T> {
        self.stages.get(self.focused.0?).ok()
    }
}

// What the panel changes: one of the focused stage's components.
#[derive(SystemParam)]
pub struct FocusedMut<'w, 's, T: Component<Mutability = Mutable>> {
    focused: Res<'w, FocusedStage>,
    stages: Query<'w, 's, &'static mut T, With<Stage>>,
}

impl<T: Component<Mutability = Mutable>> FocusedMut<'_, '_, T> {
    pub fn get(&self) -> Option<&T> {
        self.stages.get(self.focused.0?).ok()
    }

    pub fn get_mut(&mut self) -> Option<Mut<'_, T>> {
        self.stages.get_mut(self.focused.0?).ok()
    }
}

// Run conditions for the panel, standing in for resource_changed and resource_added. Moving
// the focus to another stage counts as a change of everything it shows.

pub fn focused_changed<T: Component>(
    focused: Res<FocusedStage>,
    stages: Query<Ref<T>, With<Stage>>,
) -> bool {
    let current = focused.0.and_then(|stage| stages.get(stage).ok());
    current.is_some_and(|value| focused.is_changed() || value.is_changed())
}

pub fn focused_added<T: Component>(
    focused: Res<FocusedStage>,
    stages: Query<Ref<T>, With<Stage>>,
) -> bool {
    let current = focused.0.and_then(|stage| stages.get(stage).ok());
    current.is_some_and(|value| focused.is_changed() || value.is_added())
}

pub fn focused_has<T: Component>(
    focused: Res<FocusedStage>,
    stages: Query<(), (With<T>, With<Stage>)>,
) -> bool {
    focused.0.is_some_and(|stage| stages.contains(stage))
}
