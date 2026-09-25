//! The few widgets the settings panel is built from, following the design's
//! widget kit. Each returns a bundle to spawn, so pages read as a tree.

use bevy::prelude::*;

use crate::helpers::Background;
use crate::helpers::BaseBackground;
use crate::helpers::ButtonBuilder;
use crate::helpers::UiButton;
use crate::helpers::UiElement;
use crate::helpers::UiNode;
use crate::helpers::WithBackground;
use crate::helpers::WithText;
use crate::theme::*;

/// A plain text node. Ignored by picking so a click on a label reaches the
/// button under it, and hover feedback does not flicker between the two.
pub(crate) fn label(text: impl Into<String>, size: FontSize, color: Color) -> impl Bundle {
    (Text::new(text), TextFont { font_size: size, ..default() }, TextColor(color), Pickable::IGNORE)
}

/// The shared look of every clickable box: outlined, rounded, text centred.
fn control(text: &str, size: FontSize) -> ButtonBuilder {
    ButtonBuilder::default()
        .text(text)
        .font_size(size)
        .text_color(TEXT)
        .bg_color(BUTTON_BG)
        .border(LINE)
        .border_color(BORDER)
        .border_radius(RADIUS)
        .height(CONTROL_HEIGHT)
        .justify_content(JustifyContent::Center)
        .align_items(AlignItems::Center)
}

/// A text button, sized to its label. Left unbuilt so a caller can restyle it
/// (the Quit button's colours, a tab's width) before `build_with`.
pub(crate) fn button(text: &str) -> ButtonBuilder {
    control(text, CONTROL_SIZE).edit_node(|n| n.padding = UiRect::horizontal(Val::Px(14.0)))
}

/// A square button holding one character, such as `<` or `>`.
pub(crate) fn glyph_button(glyph: &str, action: UiButton) -> impl Bundle {
    control(glyph, GLYPH_SIZE)
        .width(CONTROL_HEIGHT)
        .edit_node(|n| n.flex_shrink = 0.0)
        .build_with(UiElement::Button(action))
}

/// Changes a button's resting fill. Hover feedback brightens from
/// [`BaseBackground`], so both have to change together.
pub(crate) fn paint(base: &mut BaseBackground, background: &mut BackgroundColor, fill: Color) {
    base.0 = Background::Color(fill);
    background.0 = fill;
}
