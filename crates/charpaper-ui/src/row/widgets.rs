use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use super::Setting;
use crate::locale::Tr;
use crate::theme::*;
use crate::widgets::*;

// The part of a row the wheel changes rather than scrolls past: the arrows and value, the
// segments, or the picker's value.
#[derive(Component)]
pub(crate) struct Control;

// The box around a value, outlined while the wheel would change it.
#[derive(Component)]
pub(crate) struct Well;

// The text of the current value: a number, or the name of the current choice.
#[derive(Component)]
pub(crate) struct Value;

// The bar along the bottom of a number's box, as long as the number is far into its range.
#[derive(Component)]
pub(crate) struct FillBar;

// Holds a row's segment buttons, one per choice.
#[derive(Component)]
pub(crate) struct SegmentBar;

// The list that opens under a picker, with a button per choice.
#[derive(Component)]
pub(crate) struct PickerList;

// `<` and `>` beside a number, and which way they step.
#[derive(Component)]
pub(crate) struct Arrow(pub i32);

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
    // A number stepped with `<` and `>`, with a bar under it showing how far into its range it
    // is.
    pub(crate) fn stepper(self) -> impl Bundle {
        let well = (
            Node {
                flex_grow: 1.0,
                height: CONTROL_HEIGHT,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(LINE),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            Fill(WELL_BG),
            Stroke(BORDER),
            Pickable::IGNORE,
            Well,
            children![
                (label("-", BODY_SIZE, TEXT), Value),
                (
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::ZERO,
                        height: Val::Px(2.0),
                        ..default()
                    },
                    Fill(BAR),
                    Pickable::IGNORE,
                    FillBar,
                ),
            ],
        );
        // Pickable, so the wheel finds it in the gaps between the arrows and the value too.
        let control = (
            Node { flex_grow: 1.0, column_gap: Val::Px(4.0), ..default() },
            Control,
            Hovered::default(),
            children![glyph_button("<", Arrow(-1)), well, glyph_button(">", Arrow(1))],
        );
        (line(self.label, control), self.row(true))
    }

    // Every choice side by side, for a switch or a short list: a click picks one, the wheel
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
            Stroke(BORDER),
            Control,
            Well,
            Hovered::default(),
            SegmentBar,
        );
        (line(self.label, control), self.row(false))
    }

    // The current choice, which opens a list of them all under the row. For lists too long to
    // show side by side, such as animations or environments. The wheel over it moves along the
    // list.
    pub(crate) fn picker(self) -> impl Bundle {
        let title = self.label;
        let value = (
            Node {
                flex_grow: 1.0,
                height: CONTROL_HEIGHT,
                padding: UiRect::horizontal(Val::Px(10.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                border: UiRect::all(LINE),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            BaseBackground::default(),
            Fill(WELL_BG),
            Stroke(BORDER),
            Open,
            Control,
            Well,
            Hovered::default(),
            // A long name is cut off rather than wrapped onto a second line, and never pushes
            // the arrow out of the box.
            children![
                (
                    label("-", BODY_SIZE, TEXT),
                    TextLayout::no_wrap(),
                    Node { min_width: Val::ZERO, overflow: Overflow::clip(), ..default() },
                    Value,
                ),
                (label("▼", SMALL_SIZE, TEXT_DIM), Node { flex_shrink: 0.0, ..default() }),
            ],
        );
        // An empty column as wide as the labels keeps the list under the value. With the list
        // hidden it has no height, so the row takes no more room than any other.
        let list = (
            Node { column_gap: Val::Px(12.0), ..default() },
            Pickable::IGNORE,
            children![
                (Node { width: LABEL_WIDTH, flex_shrink: 0.0, ..default() }, Pickable::IGNORE),
                (
                    Node {
                        display: Display::None,
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        padding: UiRect::all(Val::Px(2.0)),
                        border: UiRect::all(LINE),
                        border_radius: BorderRadius::all(RADIUS),
                        ..default()
                    },
                    Fill(WELL_BG),
                    Stroke(BORDER),
                    Pickable::IGNORE,
                    PickerList,
                ),
            ],
        );
        (
            Node { flex_direction: FlexDirection::Column, ..default() },
            Pickable::IGNORE,
            self.row(false),
            children![line(title, value), list],
        )
    }

    // A value made of several numbers, such as a color. Its value opens rows of their own under
    // it, as a picker's opens a list, and a chip of `chip` beside it shows the whole at a glance.
    pub(crate) fn drawer(self, chip: Swatch, rows: impl Bundle) -> impl Bundle {
        let title = self.label;
        let chip = (
            Node {
                width: Val::Px(14.0),
                height: Val::Px(14.0),
                flex_shrink: 0.0,
                border: UiRect::all(LINE),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            Fill(chip),
            Stroke(BORDER),
            Pickable::IGNORE,
        );
        let value = (
            Node {
                flex_grow: 1.0,
                height: CONTROL_HEIGHT,
                padding: UiRect::horizontal(Val::Px(10.0)),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                border: UiRect::all(LINE),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            BaseBackground::default(),
            Fill(WELL_BG),
            Stroke(BORDER),
            Open,
            Control,
            Well,
            Hovered::default(),
            children![
                chip,
                (label("-", BODY_SIZE, TEXT), Node { flex_grow: 1.0, ..default() }, Value),
                (label("▼", SMALL_SIZE, TEXT_DIM), Node { flex_shrink: 0.0, ..default() }),
            ],
        );
        // As wide as the whole row rather than under the value only: the rows inside have labels
        // of their own.
        let drawer = (
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                margin: UiRect::top(Val::Px(4.0)),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(LINE),
                border_radius: BorderRadius::all(RADIUS),
                ..default()
            },
            Stroke(BORDER),
            Pickable::IGNORE,
            Drawer,
            rows,
        );
        (
            Node { flex_direction: FlexDirection::Column, ..default() },
            Pickable::IGNORE,
            self.row(false),
            children![line(title, value), drawer],
        )
    }
}

// A label, then the row's control. Pickable, so the pointer over the label is on the row too,
// for the help line.
fn line(title: Tr, control: impl Bundle) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            height: Val::Px(36.0),
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

// One of the choices in a picker's list. The current one stands out.
pub(crate) fn list_item(label: &str, index: usize, current: bool) -> impl Bundle {
    ButtonBuilder::new(label)
        .font_size(BODY_SIZE)
        .text_color(if current { TEXT } else { TEXT_LABEL })
        .fill(if current { SELECTED_BG } else { WELL_BG })
        .node(|n| {
            n.height = Val::Px(28.0);
            n.padding = UiRect::horizontal(Val::Px(8.0));
            n.align_items = AlignItems::Center;
            n.border_radius = BorderRadius::all(RADIUS);
        })
        .build(Choose(index))
}

// One of a row's segments. The current one stands out.
pub(crate) fn option_button(label: &str, index: usize, current: bool) -> impl Bundle {
    ButtonBuilder::new(label)
        .font_size(CONTROL_SIZE)
        .text_color(if current { TEXT } else { TEXT_DIM })
        .fill(if current { SELECTED_BG } else { WELL_BG })
        .node(|n| {
            n.flex_grow = 1.0;
            n.flex_basis = Val::ZERO;
            n.justify_content = JustifyContent::Center;
            n.align_items = AlignItems::Center;
            n.border_radius = BorderRadius::all(RADIUS);
        })
        .build(Choose(index))
}
