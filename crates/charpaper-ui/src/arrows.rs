use std::time::Duration;

use bevy::prelude::*;

use crate::elements::Step;
use crate::elements::UiButton;

// How long an arrow is held before it starts repeating, and how often it repeats after that.
const DELAY: Duration = Duration::from_millis(400);
const INTERVAL: Duration = Duration::from_millis(70);

// The `<` and `>` beside a value. Like the arrows of a spin box, they step on press rather than
// on click, and keep stepping while held.
pub(crate) struct ArrowsPlugin;

impl Plugin for ArrowsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Pressed>()
            .add_observer(press)
            .add_observer(release)
            .add_observer(leave)
            .add_systems(Update, repeat);
    }
}

#[derive(Resource, Default)]
struct Pressed(Option<Holding>);

struct Holding {
    arrow: Entity,
    step: Step,
    next: Duration,
}

fn press(
    event: On<Pointer<Press>>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    buttons: Query<&UiButton>,
    mut pressed: ResMut<Pressed>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Some(step) = buttons.get(event.entity).ok().and_then(UiButton::step) else {
        return;
    };
    commands.trigger(step);
    let repeated = Step { continuous: true, ..step };
    pressed.0 = Some(Holding { arrow: event.entity, step: repeated, next: time.elapsed() + DELAY });
}

fn release(_event: On<Pointer<Release>>, mut pressed: ResMut<Pressed>) {
    if pressed.0.is_some() {
        pressed.0 = None;
    }
}

// Sliding off the arrow stops it, as letting go does.
fn leave(event: On<Pointer<Out>>, mut pressed: ResMut<Pressed>) {
    if pressed.0.as_ref().is_some_and(|holding| holding.arrow == event.entity) {
        pressed.0 = None;
    }
}

fn repeat(mut commands: Commands, time: Res<Time<Real>>, mut pressed: ResMut<Pressed>) {
    let now = time.elapsed();
    if pressed.0.as_ref().is_none_or(|holding| now < holding.next) {
        return;
    }
    if let Some(holding) = pressed.0.as_mut() {
        commands.trigger(holding.step);
        holding.next = now + INTERVAL;
    }
}
