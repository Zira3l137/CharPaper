mod glyphs;

use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::text::FontSource;

pub(crate) use glyphs::Icon;

use crate::theme::Ink;
use crate::theme::Swatch;

// The panel's icons are the glyphs of a font of their own, which tools/icons.py makes from the
// SVGs in resources/icons. So they are drawn as text is, sharp at any size and UI scale, and
// take the theme's colors as any label does.
pub(crate) struct IconsPlugin;

impl Plugin for IconsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, add_font);
    }
}

// A fixed handle rather than a loaded one, so that a bundle can name the font without reaching
// into the world, the way bundles name the system's font.
const ICON_FONT: Handle<Font> = uuid_handle!("08130af2-021f-4988-9878-68cb0cfd6f55");

fn add_font(mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(include_bytes!("../../assets/icons.ttf").to_vec());
    // Only a handle by index can be stale, never one by UUID.
    fonts.insert(&ICON_FONT, font).expect("a UUID handle is never stale");
}

// Each glyph fills its em square, so at this size an icon is `size` pixels square.
pub(crate) fn icon_font(size: FontSize) -> TextFont {
    TextFont { font: FontSource::Handle(ICON_FONT), font_size: size, ..default() }
}

// Ignored by picking, as a label is, so a click on it reaches the button under it.
pub(crate) fn icon(icon: Icon, size: FontSize, color: Swatch) -> impl Bundle {
    (Text::new(icon.glyph()), icon_font(size), Ink(color), Pickable::IGNORE)
}
