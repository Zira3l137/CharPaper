use std::time::Duration;

use bevy::prelude::*;
use charpaper_scene::CharacterState;
use charpaper_scene::FocusedMut;

use crate::row::FocusedWorld;
use crate::row::Step;
use crate::row::stepped;

// How long the steps must stop before a held choice is put in place.
const SETTLE: Duration = Duration::from_millis(300);

// Choices that load files: a suite, an environment, an exported camera. Picked with the wheel
// or a held arrow they wait here until the steps stop, so spinning past five environments
// loads one rather than five. Their rows already show them meanwhile.
pub(crate) struct HeldPlugin;

impl Plugin for HeldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Held>().add_systems(Update, apply_settled);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Loads {
    Suite,
    Environment,
    Camera,
}

impl Loads {
    fn of(self, state: &CharacterState) -> &Option<String> {
        match self {
            Loads::Suite => &state.suite,
            Loads::Environment => &state.environment,
            Loads::Camera => &state.camera,
        }
    }

    fn of_mut(self, state: &mut CharacterState) -> &mut Option<String> {
        match self {
            Loads::Suite => &mut state.suite,
            Loads::Environment => &mut state.environment,
            Loads::Camera => &mut state.camera,
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct Held(Vec<HeldChoice>);

struct HeldChoice {
    loads: Loads,
    value: Option<String>,
    at: Duration,
}

// What the row shows, and where its next step starts from: the held choice, or else the one in
// place. None without a stage.
pub(crate) fn current(world: &World, loads: Loads) -> Option<Option<String>> {
    let state = world.focused::<CharacterState>()?;
    let held = world.resource::<Held>().0.iter().find(|choice| choice.loads == loads);
    Some(held.map_or_else(|| loads.of(state).clone(), |choice| choice.value.clone()))
}

// The `change` of these rows. A click's choice is put in place at once, and drops anything held
// for the row, which would otherwise land after it.
pub(crate) fn step(world: &mut World, loads: Loads, options: &[Option<String>], step: &Step) {
    let Some(next) = current(world, loads).and_then(|current| stepped(options, &current, step))
    else {
        return;
    };
    let now = world.resource::<Time<Real>>().elapsed();
    let mut held = world.resource_mut::<Held>();
    held.0.retain(|choice| choice.loads != loads);
    if step.continuous {
        held.0.push(HeldChoice { loads, value: next, at: now });
        return;
    }
    if let Some(mut state) = world.focused_mut::<CharacterState>() {
        *loads.of_mut(&mut state) = next;
    }
}

fn apply_settled(
    time: Res<Time<Real>>,
    mut held: ResMut<Held>,
    mut state: FocusedMut<CharacterState>,
) {
    let Some(mut state) = state.get_mut() else {
        return;
    };
    // Read first: touching `held` mutably would mark it changed every frame.
    if held.0.is_empty() {
        return;
    }
    let now = time.elapsed();
    held.0.retain(|choice| {
        if now.saturating_sub(choice.at) < SETTLE {
            return true;
        }
        *choice.loads.of_mut(&mut state) = choice.value.clone();
        false
    });
}
