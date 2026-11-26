use bevy::prelude::*;

use super::Change;
use super::RowOf;
use super::Step;
use super::widgets::Scrub;

// Pixels along the track for each step, whatever the number's range: dragging works like
// dragging a number field in Blender, not like pulling a slider to a spot.
const PIXELS_PER_STEP: f32 = 8.0;

// Dragging along a number's track steps it, a step for every few pixels the pointer moves.
pub(crate) struct ScrubPlugin;

impl Plugin for ScrubPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(start).add_observer(drag);
    }
}

fn start(mut event: On<Pointer<DragStart>>, mut tracks: Query<&mut Scrub>) {
    if let Ok(mut scrub) = tracks.get_mut(event.entity) {
        scrub.carry = 0.0;
        event.propagate(false);
    }
}

fn drag(
    mut event: On<Pointer<Drag>>,
    mut commands: Commands,
    mut tracks: Query<&mut Scrub>,
    row_of: RowOf,
) {
    let Ok(mut scrub) = tracks.get_mut(event.entity) else {
        return;
    };
    event.propagate(false);
    if event.button != PointerButton::Primary {
        return;
    }
    scrub.carry += event.delta.x;
    let steps = (scrub.carry / PIXELS_PER_STEP).trunc();
    if steps == 0.0 {
        return;
    }
    scrub.carry -= steps * PIXELS_PER_STEP;
    if let Some((row, _)) = row_of.get(event.entity) {
        let step = Step { change: Change::By(steps as i32), continuous: true };
        commands.queue(step.on(row));
    }
}
