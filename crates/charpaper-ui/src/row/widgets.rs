use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use super::Setting;
use crate::icons::Icon;
use crate::icons::icon;
use crate::locale::Tr;
use crate::theme::*;
use crate::widgets::*;

const CHEVRON_SIZE: FontSize = FontSize::Px(14.0);
const THUMB: f32 = 14.0;

// The part of a row the wheel changes rather than scrolls past: a number's track and value, the
// segments, a switch, or a picker's value.
#[derive(Component)]
pub(crate) struct Control;

// Outlined while the wheel would change its row, and outlined in its own color, often none,
// the rest of the time.
#[derive(Component)]
pub(crate) struct Well(pub Swatch);

// The text of the current value: a number, or the name of the current choice.
#[derive(Component)]
pub(crate) struct Value;

// The accent along a number's track, as far as the number is into its range.
#[derive(Component)]
pub(crate) struct FillBar;

// The knob at the end of the fill.
#[derive(Component)]
pub(crate) struct Thumb;

// A number's track, which dragging along steps it.
#[derive(Component, Default)]
pub(crate) struct Scrub {
    // Pixels dragged that haven't made a whole step yet.
    pub carry: f32,
}

// Holds a row's segment buttons, one per choice.
#[derive(Component)]
pub(crate) struct SegmentBar;

// A row's switch, which a click flips.
#[derive(Component)]
pub(crate) struct Toggle;

// The list that opens under a picker, with a button per choice.
#[derive(Component)]
pub(crate) struct PickerList;

// A choice in a row's segments or list, by its place in it.
#[derive(Component)]
pub(crate) struct Choose(pub usize);

// A picker's or drawer's value, which opens or closes its list or rows.
#[derive(Component)]
pub(crate) struct Open;

// The rows a drawer opens under its value.
#[derive(Component)]
pub(crate) struct Drawer;

impl Setting {
    // A number on a track, with its value beside it. The wheel over the track steps it, and so
    // does dragging along it.
    pub(crate) fn stepper(self) -> impl Bundle {
        let track = (
            Node {
                flex_grow: 1.0,
                height: Val::Px(4.0),
                // So the thumb stays inside at either end.
                margin: UiRect::horizontal(Val::Px(THUMB / 2.0)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            Fill(TRACK),
            Pickable::IGNORE,
            children![
                (
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::ZERO,
                        bottom: Val::ZERO,
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    Fill(BAR),
                    Pickable::IGNORE,
                    FillBar,
                ),
                (
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px((4.0 - THUMB) / 2.0),
                        width: Val::Px(THUMB),
                        height: Val::Px(THUMB),
                        margin: UiRect::left(Val::Px(-THUMB / 2.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    Fill(KNOB),
                    Stroke(KNOB),
                    Well(KNOB),
                    Pickable::IGNORE,
                    Thumb,
                ),
            ],
        );
        let value = (
            Node {
                width: Val::Px(52.0),
                flex_shrink: 0.0,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            Pickable::IGNORE,
            children![(label("-", CONTROL_SIZE, TEXT_DIM), Value)],
        );
        // Pickable all over, so the wheel and a drag find it between the track and the value too.
        let control = (
            Node {
                flex_grow: 1.0,
                height: CONTROL_HEIGHT,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            Control,
            Scrub::default(),
            Hovered::default(),
            children![track, value],
        );
        (line(self.label, control), self.row(true))
    }

    // Every choice side by side in a groove, for a short list: a click picks one, the wheel
    // moves along them. The buttons are made from the choices the row shows.
    pub(crate) fn segments(self) -> impl Bundle {
        let control = (
            Node {
                flex_grow: 1.0,
                height: CONTROL_HEIGHT,
                padding: UiRect::all(Val::Px(2.0)),
                column_gap: Val::Px(2.0),
                border: UiRect::all(LINE),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            Fill(WELL_BG),
            Stroke(CLEAR),
            Well(CLEAR),
            Control,
            Hovered::default(),
            SegmentBar,
        );
        (line(self.label, control), self.row(false))
    }

    // On or off, at the end of the row. A click flips it; the wheel turns it on going up and
    // off going down.
    pub(crate) fn switch(self) -> impl Bundle {
        let control = switch_track((Control, Well(CLEAR), Toggle, Hovered::default()));
        (end_line(self.label, control), self.row(false))
    }

    // The current choice at the end of the row, which opens a list of them all under it. For
    // lists too long to show side by side, such as animations or environments. The wheel over
    // it moves along the list.
    pub(crate) fn picker(self) -> impl Bundle {
        let title = self.label;
        // A long name is cut off rather than wrapped onto a second line, and never pushes the
        // chevron out.
        let value = (
            choice_box(),
            children![
                (
                    label("-", BODY_SIZE, TEXT_LABEL),
                    TextLayout::no_wrap(),
                    Node { min_width: Val::ZERO, overflow: Overflow::clip(), ..default() },
                    Value,
                ),
                (
                    icon(Icon::ChevronDown, CHEVRON_SIZE, TEXT_DIM),
                    Node { flex_shrink: 0.0, ..default() }
                ),
            ],
        );
        // With the list hidden it has no height, so the row takes no more room than any other.
        let list = (
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                margin: UiRect::bottom(Val::Px(6.0)),
                padding: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            Fill(WELL_BG),
            Pickable::IGNORE,
            PickerList,
        );
        (
            Node { flex_direction: FlexDirection::Column, ..default() },
            Pickable::IGNORE,
            self.row(false),
            children![end_line(title, value), list],
        )
    }

    // A value made of several numbers, such as a color, or one with more to say than fits. Its
    // value opens rows or other content of their own under it, as a picker's opens a list, and
    // a chip of `chip`, if any, beside it shows the whole at a glance.
    pub(crate) fn drawer(self, chip: Option<Swatch>, rows: impl Bundle) -> impl Bundle {
        let title = self.label;
        let chip = (
            Node {
                display: display(chip.is_some()),
                width: Val::Px(14.0),
                height: Val::Px(14.0),
                flex_shrink: 0.0,
                border: UiRect::all(LINE),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            Fill(chip.unwrap_or(CLEAR)),
            Stroke(BORDER),
            Pickable::IGNORE,
        );
        let value = (
            choice_box(),
            children![
                chip,
                (label("-", BODY_SIZE, TEXT_LABEL), Value),
                (
                    icon(Icon::ChevronDown, CHEVRON_SIZE, TEXT_DIM),
                    Node { flex_shrink: 0.0, ..default() }
                ),
            ],
        );
        // As wide as the whole row: the rows inside have labels of their own.
        let drawer = (
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                margin: UiRect::bottom(Val::Px(6.0)),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(2.0)),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            Fill(WELL_BG),
            Pickable::IGNORE,
            Drawer,
            rows,
        );
        (
            Node { flex_direction: FlexDirection::Column, ..default() },
            Pickable::IGNORE,
            self.row(false),
            children![end_line(title, value), drawer],
        )
    }
}

// A picker's or drawer's value: plain text on the card until the pointer is on it, with room to
// show a hover.
fn choice_box() -> impl Bundle {
    (
        Node {
            min_width: Val::ZERO,
            height: CONTROL_HEIGHT,
            padding: UiRect::horizontal(Val::Px(8.0)),
            column_gap: Val::Px(6.0),
            align_items: AlignItems::Center,
            border: UiRect::all(LINE),
            border_radius: BorderRadius::all(RADIUS),
            ..default()
        },
        BaseBackground::default(),
        Fill(CARD_BG),
        Stroke(CLEAR),
        Well(CLEAR),
        Open,
        Control,
        Hovered::default(),
    )
}

// A label, then the row's control filling the rest. Pickable, so the pointer over the label is
// on the row too, for the help line.
fn line(title: Tr, control: impl Bundle) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            height: ROW_HEIGHT,
            ..default()
        },
        children![
            (
                Node { width: LABEL_WIDTH, flex_shrink: 0.0, ..default() },
                Pickable::IGNORE,
                children![translated(title, BODY_SIZE, TEXT_LABEL)],
            ),
            control,
        ],
    )
}

// A label taking what room there is, then the row's control at its own size at the end.
fn end_line(title: Tr, control: impl Bundle) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(12.0),
            height: ROW_HEIGHT,
            ..default()
        },
        children![
            (
                Node { flex_shrink: 0.0, ..default() },
                Pickable::IGNORE,
                children![translated(title, BODY_SIZE, TEXT_LABEL)],
            ),
            control,
        ],
    )
}

// One of the choices in a picker's list. The current one is tinted with the accent.
pub(crate) fn list_item(label: &str, index: usize, current: bool) -> impl Bundle {
    ButtonBuilder::new(label)
        .font_size(BODY_SIZE)
        .text_color(if current { TEXT } else { TEXT_LABEL })
        .fill(if current { SELECTED_BG } else { WELL_BG })
        .node(|n| {
            n.height = Val::Px(32.0);
            n.padding = UiRect::horizontal(Val::Px(10.0));
            n.align_items = AlignItems::Center;
            n.border_radius = BorderRadius::all(INNER_RADIUS);
        })
        .build(Choose(index))
}

// One of a row's segments. The current one is raised out of the groove.
pub(crate) fn option_button(label: &str, index: usize, current: bool) -> impl Bundle {
    ButtonBuilder::new(label)
        .font_size(CONTROL_SIZE)
        .text_color(if current { TEXT } else { TEXT_DIM })
        .fill(if current { RAISED_BG } else { WELL_BG })
        .node(|n| {
            n.flex_grow = 1.0;
            n.flex_basis = Val::ZERO;
            n.justify_content = JustifyContent::Center;
            n.align_items = AlignItems::Center;
            n.border_radius = BorderRadius::all(INNER_RADIUS);
        })
        .build(Choose(index))
}
