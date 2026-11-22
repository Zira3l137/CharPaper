// What the mouse did this frame, as hotspots need it: where the pointer points into which
// stage's picture, and how long the mouse had kept still. Stillness goes by real time: the
// app may draw a frame a second while nobody looks, and game time would barely move then.

use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;
use bevy::window::CursorLeft;
use bevy::window::CursorMoved;

use crate::camera::SceneCamera;
use crate::render::ScreenParts;
use crate::screens::Screen;
use crate::screens::in_picture;
use crate::stage::StageOf;

#[derive(Resource, Default)]
pub(super) struct Mouse {
    // The stage pointed into, the ray into its scene, and where in the window.
    pub(super) sight: Option<(Entity, Ray3d, Vec2)>,
    pub(super) real_now: f64,
    // When it last moved or a button was pressed, in real time.
    last_active: Option<f64>,
    // How long it had kept still, if it moved this frame.
    pub(super) came_back_after: Option<f64>,
}

impl Mouse {
    // How long it has kept still, in real time.
    pub(super) fn still(&self) -> f64 {
        self.real_now - self.last_active.unwrap_or(self.real_now)
    }
}

// Where the pointer is over the desktop, as the window's own input or the desktop's forwarded
// to it tells. Unlike CursorPosition, it is gone while another app's window covers the desktop.
#[derive(Default)]
pub(super) struct Pointer(Option<(Entity, Vec2)>);

#[allow(clippy::too_many_arguments)]
pub(super) fn mouse(
    real: Res<Time<Real>>,
    buttons: Res<ButtonInput<MouseButton>>,
    hover: Option<Res<HoverMap>>,
    mut moved: MessageReader<CursorMoved>,
    mut left: MessageReader<CursorLeft>,
    mut pointer: Local<Pointer>,
    mut mouse: ResMut<Mouse>,
    windows: Query<&Window>,
    screens: Query<(&Screen, &ScreenParts)>,
    cameras: Query<(&Camera, &GlobalTransform), With<SceneCamera>>,
    stage_of: StageOf,
) {
    let real_now = real.elapsed_secs_f64();
    mouse.real_now = real_now;
    let last_active = *mouse.last_active.get_or_insert(real_now);
    let mut active = buttons.get_just_pressed().next().is_some();
    for event in moved.read() {
        pointer.0 = Some((event.window, event.position));
        active = true;
    }
    mouse.came_back_after = active.then_some(real_now - last_active);
    if active {
        mouse.last_active = Some(real_now);
    }
    if left.read().last().is_some() {
        pointer.0 = None;
    }
    // Over the panel, nothing under it reacts. Picking finds UI and the window under it alone,
    // and the scene image is UI too: it has to be pickable, since the orbit camera is steered
    // by drags and scrolls on it.
    let hits = hover.as_ref().and_then(|map| map.get(&PointerId::Mouse));
    let on_panel = hits.is_some_and(|hits| {
        hits.keys().any(|&hit| {
            !windows.contains(hit) && !screens.iter().any(|(_, parts)| parts.image_node == hit)
        })
    });
    mouse.sight = pointer.0.filter(|_| !on_panel).and_then(|(window, position)| {
        let (_, parts) = screens.iter().find(|(screen, _)| screen.window == window)?;
        let (camera, camera_global) = cameras.get(parts.camera).ok()?;
        let window = windows.get(window).ok()?;
        let at = in_picture(camera, position / Vec2::new(window.width(), window.height()))?;
        let ray = camera.viewport_to_world(camera_global, at).ok()?;
        Some((stage_of.get(parts.camera)?, ray, position))
    });
}
