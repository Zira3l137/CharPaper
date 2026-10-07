mod button;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::FontSource;

pub(crate) use button::BaseBackground;
pub(crate) use button::ButtonBuilder;
pub(crate) use button::HoverFeedback;

use crate::elements::Change;
use crate::elements::Chevron;
use crate::elements::Control;
use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::Step;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::elements::Well;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

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
pub(crate) fn shown_section(title: Tr, section: Section, content: impl Bundle) -> impl Bundle {
    section_with(Display::Flex, title, section, content)
}

// Whether the body is folded is up to the panel, which sets it from UiState on the first frame.
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
        children![
            fold_header(title, section),
            (
                Node { flex_direction: FlexDirection::Column, ..default() },
                UiContainer::Body(section),
                Pickable::IGNORE,
                children![content],
            ),
        ],
    )
}

// The whole title row is the button. It's filled with the panel's own color, which looks like
// no fill at rest but gives hover feedback something to brighten.
fn fold_header(title: Tr, section: Section) -> impl Bundle {
    (
        Node {
            height: Val::Px(24.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            padding: UiRect::horizontal(Val::Px(4.0)),
            border_radius: BorderRadius::all(RADIUS),
            ..default()
        },
        BaseBackground(PANEL_BG),
        BackgroundColor(PANEL_BG),
        UiButton::Fold(section),
        children![
            // Fixed width: the two arrows differ, and the title shouldn't shift between them.
            (
                Node { width: Val::Px(10.0), justify_content: JustifyContent::Center, ..default() },
                Pickable::IGNORE,
                children![(label("", SMALL_SIZE, TEXT_DIM), Chevron(section))],
            ),
            translated(title, SMALL_SIZE, TEXT_DIM),
        ],
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
            // Pickable, so the wheel finds it in the gaps between the arrows and the value too.
            (
                Node { flex_grow: 1.0, column_gap: Val::Px(4.0), ..default() },
                Control(cycler),
                Hovered::default(),
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
                        Well(cycler),
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

// The option the step lands on. Moving by places goes round past either end when the step
// wraps and stops there otherwise. An unknown current value moves to the first option.
pub(crate) fn step<T: PartialEq + Clone>(options: &[T], current: &T, event: &Step) -> Option<T> {
    let len = options.len() as i32;
    let index = match (event.change, options.iter().position(|o| o == current)) {
        (Change::To(index), _) => index as i32,
        (Change::By(by), Some(i)) if event.wraps() => (i as i32 + by).rem_euclid(len),
        (Change::By(by), Some(i)) => (i as i32 + by).clamp(0, len - 1),
        (Change::By(_), None) => 0,
    };
    options.get(index as usize).cloned()
}

// A switch is a list of Off and On: a click flips it, going up turns it on, going down off.
pub(crate) fn flip(on: bool, event: &Step) -> bool {
    step(&[false, true], &on, event).unwrap_or(on)
}
