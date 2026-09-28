use bevy::prelude::*;

use crate::helpers::Background;
use crate::helpers::BaseBackground;
use crate::helpers::ButtonBuilder;
use crate::helpers::Cycler;
use crate::helpers::CyclerValue;
use crate::helpers::UiButton;
use crate::helpers::UiContainer;
use crate::helpers::UiElement;
use crate::helpers::UiNode;
use crate::helpers::WithBackground;
use crate::helpers::WithText;
use crate::theme::*;

// Ignored by picking, so a click on the text reaches the button under it.
pub(crate) fn label(text: impl Into<String>, size: FontSize, color: Color) -> impl Bundle {
    (Text::new(text), TextFont { font_size: size, ..default() }, TextColor(color), Pickable::IGNORE)
}

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

pub(crate) fn button(text: &str) -> ButtonBuilder {
    control(text, CONTROL_SIZE).edit_node(|n| n.padding = UiRect::horizontal(Val::Px(14.0)))
}

pub(crate) fn glyph_button(glyph: &str, action: UiButton) -> impl Bundle {
    control(glyph, GLYPH_SIZE)
        .width(CONTROL_HEIGHT)
        .edit_node(|n| n.flex_shrink = 0.0)
        .build_with(UiElement::Button(action))
}

// Hidden until something fills it.
pub(crate) fn section(title: &str, marker: UiContainer, content: impl Bundle) -> impl Bundle {
    section_with(Display::None, title, marker, content)
}

pub(crate) fn open_section(title: &str, marker: UiContainer, content: impl Bundle) -> impl Bundle {
    section_with(Display::Flex, title, marker, content)
}

fn section_with(
    display: Display,
    title: &str,
    marker: UiContainer,
    content: impl Bundle,
) -> impl Bundle {
    (
        Node { display, flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() },
        UiElement::Container(marker),
        Pickable::IGNORE,
        children![label(title, SMALL_SIZE, TEXT_DIM), content],
    )
}

pub(crate) fn cycler(title: impl Into<String>, cycler: Cycler) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            height: Val::Px(36.0),
            ..default()
        },
        UiElement::Container(UiContainer::Row(cycler)),
        Pickable::IGNORE,
        children![
            (
                Node { width: LABEL_WIDTH, flex_shrink: 0.0, ..default() },
                Pickable::IGNORE,
                children![label(title, BODY_SIZE, TEXT_LABEL)],
            ),
            (
                Node { flex_grow: 1.0, column_gap: Val::Px(4.0), ..default() },
                Pickable::IGNORE,
                children![
                    glyph_button("<", UiButton::Previous(cycler)),
                    (
                        Node {
                            flex_grow: 1.0,
                            height: CONTROL_HEIGHT,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(LINE),
                            border_radius: BorderRadius::all(RADIUS),
                            ..default()
                        },
                        BackgroundColor(WELL_BG),
                        BorderColor::all(BORDER),
                        Pickable::IGNORE,
                        children![(label("-", BODY_SIZE, TEXT), CyclerValue(cycler))],
                    ),
                    glyph_button(">", UiButton::Next(cycler)),
                ],
            ),
        ],
    )
}

// Hover feedback brightens from BaseBackground, so both change together.
pub(crate) fn paint(base: &mut BaseBackground, background: &mut BackgroundColor, fill: Color) {
    base.0 = Background::Color(fill);
    background.0 = fill;
}
