use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use charpaper_scene::CursorAt;
use charpaper_scene::CursorPosition;

use crate::wallpaper::Backend;

pub(crate) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, track_cursor);
    }
}

// The backend knows where the cursor is anywhere on screen. Bevy knows only while it is over
// the window, and never once the window is attached, since the desktop keeps its input.
fn track_cursor(
    _main_thread: NonSendMarker,
    mut backend: ResMut<Backend>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut cursor: ResMut<CursorPosition>,
) {
    let Ok((window, settings)) = windows.single() else {
        cursor.set_if_neq(CursorPosition(None));
        return;
    };
    let position = backend
        .0
        .cursor_position()
        .map(Vec2::from_array)
        .or_else(|| settings.physical_cursor_position());
    cursor.set_if_neq(CursorPosition(position.map(|position| CursorAt { window, position })));
}
