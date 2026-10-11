use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use charpaper_scene::CursorAt;
use charpaper_scene::CursorPosition;

use crate::wallpaper::Backend;
use crate::wallpaper::windows::OnMonitor;

pub(crate) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, track_cursor);
    }
}

// The backend knows where the cursor is anywhere on screen. Bevy knows only while it is over
// one of the windows, and never once they are attached, since the desktop keeps their input.
fn track_cursor(
    _main_thread: NonSendMarker,
    mut backend: ResMut<Backend>,
    windows: Query<(Entity, &Window, Option<&OnMonitor>)>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut last: Local<Option<Entity>>,
    mut cursor: ResMut<CursorPosition>,
) {
    let at = match backend.0.cursor_position() {
        Some(at) => {
            let placed = |window: Entity| windows.get(window).ok()?.2.map(|on| on.0.area);
            let over = windows
                .iter()
                .find(|(_, _, on)| on.is_some_and(|on| on.0.area.contains(at)))
                .map(|(window, _, _)| window);
            // Off our monitors, measured from the window it was over last, so the character
            // keeps looking that way.
            let window = over
                .or(*last)
                .filter(|&window| placed(window).is_some())
                .or_else(|| primary.iter().next());
            *last = window;
            window.and_then(|window| {
                let area = placed(window)?;
                let position = IVec2::from_array(at) - IVec2::new(area.x, area.y);
                Some(CursorAt { window, position: position.as_vec2() })
            })
        }
        None => windows.iter().find_map(|(window, settings, _)| {
            Some(CursorAt { window, position: settings.physical_cursor_position()? })
        }),
    };
    cursor.set_if_neq(CursorPosition(at));
}
