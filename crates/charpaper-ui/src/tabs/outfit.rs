use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;
use charpaper_scene::Focused;
use charpaper_scene::FocusedMut;
use charpaper_scene::SkinObjects;
use charpaper_scene::focused_added;
use charpaper_scene::focused_changed;
use charpaper_scene::focused_has;
use i18n_embed_fl::fl;

use crate::locale::Locale;
use crate::panel::new_panel;
use crate::row::on_off;
use crate::section::Content;
use crate::section::Section;
use crate::theme::*;
use crate::widgets::*;

// The outfit section of the Scene tab: a tile per skin of the suite, and the worn skin's
// objects to switch on and off. Lists the suite gives rather than rows, so this keeps them up.
pub(crate) struct OutfitPlugin;

impl Plugin for OutfitPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            (
                fill_skins.run_if(
                    focused_added::<ActiveSuite>
                        .or_eager(new_panel.and_eager(focused_has::<ActiveSuite>)),
                ),
                style_skins.run_if(focused_changed::<CharacterState>.or_eager(new_panel)),
                fill_objects.run_if(focused_changed::<SkinObjects>.or_eager(new_panel)),
                show_objects.run_if(
                    focused_changed::<CharacterState>
                        .or_eager(focused_changed::<SkinObjects>)
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(new_panel),
                ),
            )
                .chain(),
        );
    }
}

#[derive(Component)]
#[require(Content)]
struct SkinGrid;

#[derive(Component)]
#[require(Content)]
struct ObjectList;

// Wears the skin with this name.
#[derive(Component)]
struct SkinTile(String);

// Switches the worn skin's object with this name on or off.
#[derive(Component)]
struct ObjectSwitch(String);

pub(super) fn section() -> impl Bundle {
    crate::section::section(
        |l| fl!(l, "section-outfit"),
        Section::Outfit,
        children![
            (
                // Hidden until the suite's skins fill it.
                Node {
                    display: Display::None,
                    grid_template_columns: vec![RepeatedGridTrack::flex(3, 1.0)],
                    row_gap: Val::Px(8.0),
                    column_gap: Val::Px(8.0),
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                Pickable::IGNORE,
                SkinGrid,
            ),
            crate::section::section(
                |l| fl!(l, "section-parts"),
                Section::Parts,
                children![(
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                    ObjectList,
                )],
            ),
        ],
    )
}

// One tile per skin, made again for each suite.
fn fill_skins(
    mut commands: Commands,
    suite: Focused<ActiveSuite>,
    grids: Query<(Entity, &mut Node), With<SkinGrid>>,
) {
    let Some(suite) = suite.get() else {
        return;
    };
    for (grid, mut node) in grids {
        node.display = if suite.skins.is_empty() { Display::None } else { Display::Grid };
        commands.entity(grid).despawn_children().with_children(|grid| {
            for skin in &suite.skins {
                grid.spawn(
                    button(&skin.name)
                        .font_size(SMALL_SIZE)
                        .node(|n| n.height = Val::Px(40.0))
                        .build(SkinTile(skin.name.clone())),
                );
            }
        });
    }
}

fn style_skins(
    state: Focused<CharacterState>,
    tiles: Query<(&SkinTile, &mut BaseBackground, &mut BackgroundColor, &mut BorderColor)>,
) {
    let Some(state) = state.get() else {
        return;
    };
    for (tile, mut base, mut background, mut border) in tiles {
        let worn = state.skin.as_ref() == Some(&tile.0);
        paint(&mut base, &mut background, if worn { SELECTED_BG } else { BUTTON_BG });
        *border = BorderColor::all(if worn { ACCENT } else { BORDER });
    }
}

// One row per mesh object of the worn skin, in every panel.
fn fill_objects(
    mut commands: Commands,
    objects: Focused<SkinObjects>,
    lists: Query<Entity, With<ObjectList>>,
) {
    let Some(objects) = objects.get() else {
        return;
    };
    for list in &lists {
        commands.entity(list).despawn_children().with_children(|rows| {
            for name in objects.names() {
                rows.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        height: Val::Px(32.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                    children![
                        (
                            Node { flex_grow: 1.0, ..default() },
                            Pickable::IGNORE,
                            children![label(name, BODY_SIZE, TEXT_LABEL)],
                        ),
                        button("-")
                            .node(|n| n.width = Val::Px(56.0))
                            .build(ObjectSwitch(name.to_string())),
                    ],
                ));
            }
        });
    }
}

// Only with two or more objects: switching off a skin's only object is switching off the skin.
fn show_objects(
    state: Focused<CharacterState>,
    objects: Focused<SkinObjects>,
    locale: Res<Locale>,
    mut lists: Query<&mut Node, With<ObjectList>>,
    switches: Query<(&ObjectSwitch, &Children)>,
    mut texts: Query<&mut Text>,
) {
    let (Some(state), Some(objects)) = (state.get(), objects.get()) else {
        return;
    };
    for mut node in &mut lists {
        node.display = display(objects.names().count() > 1);
    }
    let hidden = state.skin.as_ref().and_then(|skin| state.hidden.get(skin));
    for (switch, children) in &switches {
        let shown = !hidden.is_some_and(|hidden| hidden.contains(&switch.0));
        for &child in &**children {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = on_off(&locale, shown);
            }
        }
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    tiles: Query<&SkinTile>,
    switches: Query<&ObjectSwitch>,
    mut state: FocusedMut<CharacterState>,
) {
    if let Ok(SkinTile(skin)) = tiles.get(event.entity)
        && let Some(mut state) = state.get_mut()
    {
        state.skin = Some(skin.clone());
    }
    if let Ok(ObjectSwitch(object)) = switches.get(event.entity)
        && let Some(mut state) = state.get_mut()
        && let Some(skin) = state.skin.clone()
    {
        let hidden = state.hidden.entry(skin).or_default();
        if !hidden.remove(object) {
            hidden.insert(object.clone());
        }
    }
}
