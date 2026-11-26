use bevy::prelude::*;
use charpaper_scene::Focused;
use charpaper_scene::focused_changed;
use charpaper_settings::Graphics;
use charpaper_settings::Preset;
use i18n_embed_fl::fl;

use super::graphics::edit_graphics;
use crate::locale::Tr;
use crate::panel::new_panel;
use crate::section::Content;
use crate::section::Section;
use crate::theme::*;
use crate::widgets::*;

// The presets as cards rather than a row: a name, what it's for, and bars for how far it goes,
// with Custom across the bottom. The open one is outlined by the accent.
pub(crate) struct PresetsPlugin;

impl Plugin for PresetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            style_cards.run_if(focused_changed::<Graphics>.or_eager(new_panel)),
        );
    }
}

#[derive(Component)]
#[require(Content)]
struct PresetCard(Preset);

pub(super) fn section() -> impl Bundle {
    crate::section::section(
        |l| fl!(l, "section-quality"),
        Section::Quality,
        children![(
            Node {
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.0)],
                row_gap: Val::Px(8.0),
                column_gap: Val::Px(8.0),
                padding: UiRect::vertical(Val::Px(10.0)),
                ..default()
            },
            Pickable::IGNORE,
            children![
                card(
                    Preset::Fastest,
                    |l| fl!(l, "preset-fastest"),
                    |l| fl!(l, "preset-fastest-note")
                ),
                card(
                    Preset::Moderate,
                    |l| fl!(l, "preset-moderate"),
                    |l| fl!(l, "preset-moderate-note")
                ),
                card(Preset::Fancy, |l| fl!(l, "preset-fancy"), |l| fl!(l, "preset-fancy-note")),
                card(Preset::Best, |l| fl!(l, "preset-best"), |l| fl!(l, "preset-best-note")),
                card(Preset::Custom, |l| fl!(l, "preset-custom"), |l| fl!(l, "preset-custom-note")),
            ],
        )],
    )
}

fn card(preset: Preset, name: Tr, note: Tr) -> impl Bundle {
    // Each bar is a step up from Fastest; Custom's mixed bars say it's a mix.
    let lit = match preset {
        Preset::Fastest => [true, false, false, false],
        Preset::Moderate => [true, true, false, false],
        Preset::Fancy => [true, true, true, false],
        Preset::Best => [true, true, true, true],
        Preset::Custom => [true, false, true, true],
    };
    let bar = |lit: bool| {
        (
            Node {
                flex_grow: 1.0,
                height: Val::Px(4.0),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            Fill(if lit { ACCENT } else { TRACK }),
            Pickable::IGNORE,
        )
    };
    let span = if preset == Preset::Custom { 2 } else { 1 };
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(1.5)),
            border_radius: BorderRadius::all(RADIUS),
            grid_column: GridPlacement::span(span),
            ..default()
        },
        BaseBackground::default(),
        Fill(WELL_BG),
        Stroke(CLEAR),
        PresetCard(preset),
        children![
            (
                Node {
                    column_gap: Val::Px(3.0),
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                Pickable::IGNORE,
                children![bar(lit[0]), bar(lit[1]), bar(lit[2]), bar(lit[3])],
            ),
            translated(name, BODY_SIZE, TEXT),
            translated(note, SMALL_SIZE, TEXT_DIM),
        ],
    )
}

fn style_cards(graphics: Focused<Graphics>, cards: Query<(&PresetCard, &mut Fill, &mut Stroke)>) {
    let Some(graphics) = graphics.get() else {
        return;
    };
    for (card, mut fill, mut border) in cards {
        let open = card.0 == graphics.preset;
        fill.set_if_neq(Fill(if open { SELECTED_BG } else { WELL_BG }));
        border.set_if_neq(Stroke(if open { ACCENT } else { CLEAR }));
    }
}

fn on_click(event: On<Pointer<Click>>, mut commands: Commands, cards: Query<&PresetCard>) {
    if let Ok(&PresetCard(preset)) = cards.get(event.entity) {
        commands.queue(move |world: &mut World| {
            edit_graphics(world, |graphics, render| graphics.pick(preset, render));
        });
    }
}
