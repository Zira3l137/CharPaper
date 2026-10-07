use std::time::Duration;

use bevy::prelude::*;
use charpaper_scene::CharacterState;

use crate::elements::Cycler;
use crate::elements::Step;

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

#[derive(Resource, Default)]
pub(crate) struct Held(Vec<HeldChoice>);

struct HeldChoice {
    cycler: Cycler,
    value: Option<String>,
    at: Duration,
}

impl Held {
    // What the row shows, and where its next step starts from.
    pub(crate) fn current(&self, cycler: Cycler, applied: &Option<String>) -> Option<String> {
        let held = self.0.iter().find(|choice| choice.cycler == cycler);
        held.map_or_else(|| applied.clone(), |choice| choice.value.clone())
    }

    // Holds a choice from the wheel or a held arrow. A click's choice is handed back to put in
    // place at once, and drops anything held for the row, which would otherwise land after it.
    pub(crate) fn hold(
        &mut self,
        step: &Step,
        value: Option<String>,
        now: Duration,
    ) -> Option<Option<String>> {
        self.0.retain(|choice| choice.cycler != step.cycler);
        if !step.continuous {
            return Some(value);
        }
        self.0.push(HeldChoice { cycler: step.cycler, value, at: now });
        None
    }
}

fn apply_settled(time: Res<Time<Real>>, mut held: ResMut<Held>, mut state: ResMut<CharacterState>) {
    // Read first: touching `held` mutably would mark it changed every frame.
    if held.0.is_empty() {
        return;
    }
    let now = time.elapsed();
    held.0.retain(|choice| {
        if now.saturating_sub(choice.at) < SETTLE {
            return true;
        }
        let value = choice.value.clone();
        match choice.cycler {
            Cycler::Suite => state.suite = value,
            Cycler::Environment => state.environment = value,
            Cycler::Camera => state.camera = value,
            _ => {}
        }
        false
    });
}
