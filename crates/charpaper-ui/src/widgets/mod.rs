mod button;

use bevy::prelude::*;

pub(crate) use button::BaseBackground;
pub(crate) use button::ButtonBuilder;
pub(crate) use button::HoverFeedback;

use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use crate::locale::Localized;
use crate::locale::Tr;
use crate::theme::*;

// Ignored by picking, so a click on the text reaches the button under it.
pub(crate) fn label(text: impl Into<String>, size: FontSize, color: Color) -> impl Bundle {
    (Text::new(text), TextFont { font_size: size, ..default() }, TextColor(color), Pickable::IGNORE)
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

pub(crate) fn glyph_button(glyph: &str, action: UiButton) -> impl Bundle {
    control(ButtonBuilder::new(glyph), GLYPH_SIZE)
        .node(|n| {
            n.width = CONTROL_HEIGHT;
            n.flex_shrink = 0.0;
        })
        .build(action)
}

// Hidden until something fills it.
pub(crate) fn section(title: Tr, section: Section, content: impl Bundle) -> impl Bundle {
    section_with(Display::None, title, section, content)
}

// Shown from the start, for settings that exist whatever the suite holds.
pub(crate) fn open_section(title: Tr, section: Section, content: impl Bundle) -> impl Bundle {
    section_with(Display::Flex, title, section, content)
}

fn section_with(
    display: Display,
    title: Tr,
    section: Section,
    content: impl Bundle,
) -> impl Bundle {
    (
        Node { display, flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() },
        UiContainer::Section(section),
        Pickable::IGNORE,
        children![translated(title, SMALL_SIZE, TEXT_DIM), content],
    )
}

pub(crate) fn rows(content: impl Bundle) -> impl Bundle {
    (
        Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
        Pickable::IGNORE,
        content,
    )
}

// A label, then a value stepped through with `<` and `>`.
pub(crate) fn cycler(title: Tr, cycler: Cycler) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            height: Val::Px(36.0),
            ..default()
        },
        UiContainer::Row(cycler),
        Pickable::IGNORE,
        children![
            (
                Node { width: LABEL_WIDTH, flex_shrink: 0.0, ..default() },
                Pickable::IGNORE,
                children![translated(title, BODY_SIZE, TEXT_LABEL)],
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

// Changes a button's resting fill along with what it shows right now.
pub(crate) fn paint(base: &mut BaseBackground, background: &mut BackgroundColor, fill: Color) {
    base.0 = fill;
    background.0 = fill;
}

pub(crate) fn display(shown: bool) -> Display {
    if shown { Display::Flex } else { Display::None }
}

pub(crate) fn show_container(
    nodes: &mut Query<(&UiContainer, &mut Node)>,
    target: UiContainer,
    shown: bool,
) {
    for (container, mut node) in nodes.iter_mut() {
        if *container == target {
            node.display = display(shown);
        }
    }
}

pub(crate) fn set_value(values: &mut Query<(&CyclerValue, &mut Text)>, cycler: Cycler, text: &str) {
    for (value, mut shown) in values.iter_mut() {
        if value.0 == cycler {
            shown.0 = text.to_string();
        }
    }
}

pub(crate) fn on_off(locale: &FluentLanguageLoader, on: bool) -> String {
    if on { fl!(locale, "value-on") } else { fl!(locale, "value-off") }
}

// Wraps around. An unknown current value steps to the first option.
pub(crate) fn step<T: PartialEq + Clone>(options: &[T], current: &T, forward: bool) -> Option<T> {
    let len = options.len();
    if len == 0 {
        return None;
    }
    let index = match options.iter().position(|o| o == current) {
        Some(i) if forward => (i + 1) % len,
        Some(i) => (i + len - 1) % len,
        None => 0,
    };
    Some(options[index].clone())
}
