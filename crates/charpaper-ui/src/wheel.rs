use std::time::Duration;

use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::elements::Change;
use crate::elements::Control;
use crate::elements::Cycler;
use crate::elements::Step;
use crate::elements::UiContainer;
use crate::elements::Well;
use crate::theme::*;

// Logical pixels, about one row.
const SCROLL_PER_NOTCH: f32 = 40.0;
// A pause this long ends a gesture.
const REST: Duration = Duration::from_millis(300);
// Steps closer together than this count as a fast spin.
const FAST: Duration = Duration::from_millis(60);
const TOP_SPEED: i32 = 4;

// The wheel over the panel: over a row's control it changes the value, anywhere else on a page
// it scrolls the page.
pub(crate) struct WheelPlugin;

impl Plugin for WheelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gesture>().add_observer(on_wheel).add_systems(Update, light_controls);
    }
}

// Turns of the wheel without a pause between them. A gesture keeps doing what its first notch
// did, so a page scrolling under a still pointer doesn't start changing whatever row slides
// beneath it, and tuning a row doesn't turn into scrolling when the pointer drifts off it.
#[derive(Resource, Default)]
struct Gesture {
    target: Option<Target>,
    last: Duration,
    // Part of a notch not stepped yet, from a touchpad or a free-spinning wheel.
    carry: f32,
    // When the last whole notch was stepped. Timed per notch rather than per event, since a
    // touchpad sends many small events however slowly the fingers move.
    stepped: Option<Duration>,
    // Steps per notch for a number, growing while the wheel spins fast.
    speed: i32,
}

#[derive(Clone, Copy)]
enum Target {
    Scroll(Entity),
    Tune(Cycler),
}

// A global observer sees the event at every entity it bubbles through, so this acts only where
// it starts and stops it there. Input that isn't on a page is left alone: the scene's camera
// listens for it on the scene image.
fn on_wheel(
    mut event: On<Pointer<Scroll>>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    parents: Query<&ChildOf>,
    containers: Query<&UiContainer>,
    controls: Query<&Control>,
    mut pages: Query<(&mut ScrollPosition, &ComputedNode)>,
    mut gesture: ResMut<Gesture>,
) {
    let start = event.entity;
    if start != event.original_event_target() {
        return;
    }
    let mut control = None;
    let mut page = None;
    for entity in std::iter::once(start).chain(parents.iter_ancestors(start)) {
        if let (None, Ok(found)) = (control, controls.get(entity)) {
            control = Some(found.0);
        }
        if let Ok(UiContainer::Page(_)) = containers.get(entity) {
            page = Some(entity);
            break;
        }
    }
    let Some(page) = page else {
        return;
    };
    event.propagate(false);

    let now = time.elapsed();
    let since = now.saturating_sub(gesture.last);
    gesture.last = now;
    if gesture.target.is_none() || since > REST {
        let target = control.map_or(Target::Scroll(page), Target::Tune);
        *gesture = Gesture { target: Some(target), last: now, carry: 0.0, stepped: None, speed: 1 };
    }

    let notches = match event.unit {
        MouseScrollUnit::Line => event.y,
        MouseScrollUnit::Pixel => event.y / SCROLL_PER_NOTCH,
    };
    let target = gesture.target;
    match target {
        Some(Target::Scroll(page)) => scroll(&mut pages, page, notches * SCROLL_PER_NOTCH),
        Some(Target::Tune(cycler)) => {
            gesture.carry += notches;
            let whole = gesture.carry.trunc();
            if whole == 0.0 {
                return;
            }
            gesture.carry -= whole;
            let fast = gesture.stepped.is_some_and(|stepped| now.saturating_sub(stepped) < FAST);
            gesture.speed = if fast { (gesture.speed + 1).min(TOP_SPEED) } else { 1 };
            gesture.stepped = Some(now);
            let speed = if cycler.accelerates() { gesture.speed } else { 1 };
            let change = Change::By(whole as i32 * speed);
            commands.trigger(Step { cycler, change, continuous: true });
        }
        None => {}
    }
}

// Bevy draws the page scrolled no further than its end, but keeps whatever offset is written
// here. Clamped here too, or scrolling past the end would have to be wound back before the
// page moved again.
fn scroll(pages: &mut Query<(&mut ScrollPosition, &ComputedNode)>, page: Entity, up: f32) {
    let Ok((mut position, page)) = pages.get_mut(page) else {
        return;
    };
    let overflow = page.content_size().y - page.size().y + page.scrollbar_size.y;
    let end = overflow.max(0.0) * page.inverse_scale_factor();
    let y = (position.y.min(end) - up).clamp(0.0, end);
    if y != position.y {
        position.y = y;
    }
}

// Outlines the value under the pointer, so it's plain that the wheel will change it.
fn light_controls(
    controls: Query<(&Control, &Hovered), Changed<Hovered>>,
    mut wells: Query<(&Well, &mut BorderColor)>,
) {
    for (control, hovered) in &controls {
        let color = if hovered.get() { LIVE_BORDER } else { BORDER };
        for (well, mut border) in &mut wells {
            if well.0 == control.0 {
                *border = BorderColor::all(color);
            }
        }
    }
}
