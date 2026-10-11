use bevy::ecs::component::Mutable;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::camera::ShownRig;
use crate::camera::view_bundle;
use crate::character::CueClock;
use crate::character::Expressions;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::config::SceneConfig;
use crate::environment::Baking;
use crate::environment::ShownEnvironment;
use crate::gaze::GazeRig;
use crate::gaze::GazeSettings;
use crate::look::LookBackup;
use crate::lut::ShownLut;
use crate::soundscape::Soundscape;
use crate::state::CharacterState;
use crate::suite::AvailableSuites;
use crate::suite::LoadedSuite;
use crate::suite::Remembered;
use crate::suite::discover_suites;
use crate::suite::starting_suite;

// One scene running on its own: a suite with its character, environment and view, and what
// the viewer picked for them. Everything a stage spawns hangs under it, so despawning it takes
// all of that away. For now there is always exactly one.
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

// The stage the viewer is pointing at: the one the panels show and change.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct FocusedStage(pub Option<Entity>);

pub(crate) struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FocusedStage>()
            .add_systems(Startup, spawn_stage.after(discover_suites));
    }
}

fn spawn_stage(
    mut commands: Commands,
    config: Res<SceneConfig>,
    available: Res<AvailableSuites>,
    mut focused: ResMut<FocusedStage>,
) {
    let state = CharacterState { suite: starting_suite(&config, &available), ..default() };
    let remembered = Remembered(config.remembered.clone());
    let stage = commands.spawn((Name::new("Stage"), Stage, state, config.gaze.clone(), remembered));
    let stage = stage.id();
    commands.spawn((view_bundle(), ChildOf(stage)));
    focused.0 = Some(stage);
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
