use bevy::prelude::*;
use bevy::text::FontCx;

// The panel's text uses the system's fonts, which the font library maps from their files and
// Bevy forgets two frames after their last use (Bevy 0.19.1, `trim_source_cache`). The next
// text laid out with one maps the file again, and Bevy, which tells fonts apart by that
// mapping, takes it for a new font: it draws every glyph again into new 1 MB atlas images and
// never frees the old ones. Text that changes every few frames, such as a value being tuned
// or the performance monitor, made memory grow without end.
//
// A shared cache hands back the same mapping as long as any text still uses it, and the
// panel's text always does.
pub(crate) struct FontsPlugin;

impl Plugin for FontsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, keep_font_files);
    }
}

fn keep_font_files(mut fonts: ResMut<FontCx>) {
    fonts.source_cache.make_shared();
}
