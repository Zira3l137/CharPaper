mod button;

use bevy::prelude::*;
use bevy::text::FontSource;

pub(crate) use button::BaseBackground;
pub(crate) use button::ButtonBuilder;
pub(crate) use button::HoverFeedback;

use crate::icons::Icon;
use crate::icons::icon;
use crate::locale::Localized;
use crate::locale::Tr;
use crate::theme::*;

// Ignored by picking, so a click on the text reaches the button under it.
pub(crate) fn label(text: impl Into<String>, size: FontSize, color: Swatch) -> impl Bundle {
    (Text::new(text), font(size), Ink(color), Pickable::IGNORE)
}

// The system's UI font (Segoe UI on Windows) rather than Bevy's built-in one, which covers
// little beyond ASCII and would show a translation into Cyrillic, say, as empty boxes.
pub(crate) fn font(size: FontSize) -> TextFont {
    TextFont { font: FontSource::SystemUi, font_size: size, ..default() }
}

// Filled in, and filled in again on a language change, by `locale::relabel`.
pub(crate) fn translated(text: Tr, size: FontSize, color: Swatch) -> impl Bundle {
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
pub(crate) fn translated_button(text: Tr) -> ButtonBuilder {
    control(ButtonBuilder::translated(text), CONTROL_SIZE)
        .node(|n| n.padding = UiRect::horizontal(Val::Px(14.0)))
}

// A pill with a knob that sits left while off, and right on the accent while on. `paint_switch`
// sets which. Ignored by picking but for the track, which takes the click.
#[derive(Component)]
pub(crate) struct SwitchKnob;

const SWITCH: Vec2 = Vec2::new(36.0, 20.0);
const KNOB_SIZE: f32 = 14.0;
// From the inside of the track's border.
const KNOB_INSET: f32 = (SWITCH.y - 2.0 - KNOB_SIZE) / 2.0;

pub(crate) fn switch_track(parts: impl Bundle) -> impl Bundle {
    (
        Node {
            width: Val::Px(SWITCH.x),
            height: Val::Px(SWITCH.y),
            flex_shrink: 0.0,
            border: UiRect::all(LINE),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BaseBackground::default(),
        Fill(TRACK),
        Stroke(CLEAR),
        parts,
        children![(
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(KNOB_INSET),
                left: Val::Px(KNOB_INSET),
                width: Val::Px(KNOB_SIZE),
                height: Val::Px(KNOB_SIZE),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            Fill(TEXT_LABEL),
            Pickable::IGNORE,
            SwitchKnob,
        )],
    )
}

// Written only on a change, so the panel isn't laid out again for nothing.
pub(crate) fn paint_switch(
    on: bool,
    mut track: Mut<Fill>,
    mut knob: Mut<Node>,
    mut ink: Mut<Fill>,
) {
    track.set_if_neq(Fill(if on { ACCENT } else { TRACK }));
    ink.set_if_neq(Fill(if on { KNOB } else { TEXT_LABEL }));
    let left = Val::Px(if on { SWITCH.x - 2.0 - KNOB_INSET - KNOB_SIZE } else { KNOB_INSET });
    if knob.left != left {
        knob.left = left;
    }
}

// A tag that is on or off, one of a few names to pick any of, such as an outfit's parts. On, it
// is tinted with the accent and ticked; `paint_chip` sets which.
#[derive(Component)]
pub(crate) struct ChipTick;

pub(crate) fn chip(name: &str, parts: impl Bundle) -> impl Bundle {
    (
        Node {
            height: CONTROL_HEIGHT,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            padding: UiRect::horizontal(Val::Px(12.0)),
            border: UiRect::all(LINE),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BaseBackground::default(),
        Fill(WELL_BG),
        Stroke(BORDER),
        parts,
        children![
            (
                icon(Icon::Check, FontSize::Px(14.0), TEXT),
                Node { display: Display::None, ..default() },
                ChipTick,
            ),
            label(name, CONTROL_SIZE, TEXT_LABEL),
        ],
    )
}

// Written only on a change, so the panel isn't laid out again for nothing.
pub(crate) fn paint_chip(
    on: bool,
    mut fill: Mut<Fill>,
    mut stroke: Mut<Stroke>,
    mut tick: Mut<Node>,
    mut ink: Mut<Ink>,
) {
    fill.set_if_neq(Fill(if on { SELECTED_BG } else { WELL_BG }));
    stroke.set_if_neq(Stroke(if on { ACCENT } else { BORDER }));
    ink.set_if_neq(Ink(if on { TEXT } else { TEXT_LABEL }));
    let shown = display(on);
    if tick.display != shown {
        tick.display = shown;
    }
}

pub(crate) fn display(shown: bool) -> Display {
    if shown { Display::Flex } else { Display::None }
}
