mod button;

use bevy::prelude::*;
use bevy::text::FontSource;

pub(crate) use button::BaseBackground;
pub(crate) use button::ButtonBuilder;
pub(crate) use button::HoverFeedback;

use crate::locale::Localized;
use crate::locale::Tr;
use crate::theme::*;

// Ignored by picking, so a click on the text reaches the button under it.
pub(crate) fn label(text: impl Into<String>, size: FontSize, color: Color) -> impl Bundle {
    (Text::new(text), font(size), TextColor(color), Pickable::IGNORE)
}

// The system's UI font (Segoe UI on Windows) rather than Bevy's built-in one, which covers
// little beyond ASCII and would show a translation into Cyrillic, say, as empty boxes.
pub(crate) fn font(size: FontSize) -> TextFont {
    TextFont { font: FontSource::SystemUi, font_size: size, ..default() }
}

// Filled in, and filled in again on a language change, by `locale::relabel`.
pub(crate) fn translated(text: Tr, size: FontSize, color: Color) -> impl Bundle {
    (label("", size, color), Localized(Some(text)))
}

fn control(builder: ButtonBuilder, size: FontSize) -> ButtonBuilder {
    builder.font_size(size).text_color(TEXT).fill(BUTTON_BG).border_color(BORDER).node(|n| {
        n.border = UiRect::all(LINE);
        n.border_radius = BorderRadius::all(RADIUS);
        n.height = CONTROL_HEIGHT;
        n.justify_content = JustifyContent::Center;
        n.align_items = AlignItems::Center;
    })
}

// Left unbuilt, so the caller can restyle it before `build`.
pub(crate) fn button(text: &str) -> ButtonBuilder {
    control(ButtonBuilder::new(text), CONTROL_SIZE)
        .node(|n| n.padding = UiRect::horizontal(Val::Px(14.0)))
}

pub(crate) fn translated_button(text: Tr) -> ButtonBuilder {
    control(ButtonBuilder::translated(text), CONTROL_SIZE)
        .node(|n| n.padding = UiRect::horizontal(Val::Px(14.0)))
}

pub(crate) fn glyph_button(glyph: &str, action: impl Bundle) -> impl Bundle {
    control(ButtonBuilder::new(glyph), GLYPH_SIZE)
        .node(|n| {
            n.width = CONTROL_HEIGHT;
            n.flex_shrink = 0.0;
        })
        .build(action)
}

// Changes a button's resting fill along with what it shows right now.
pub(crate) fn paint(base: &mut BaseBackground, background: &mut BackgroundColor, fill: Color) {
    base.0 = fill;
    background.0 = fill;
}

pub(crate) fn display(shown: bool) -> Display {
    if shown { Display::Flex } else { Display::None }
}
