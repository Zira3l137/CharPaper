use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;
use charpaper_scene::Focused;
use charpaper_scene::FocusedMut;
use charpaper_scene::SkinObjects;
use charpaper_scene::Thumbnails;
use charpaper_scene::focused_added;
use charpaper_scene::focused_changed;
use charpaper_scene::focused_has;
use i18n_embed_fl::fl;

use crate::icons::Icon;
use crate::icons::icon;
use crate::panel::new_panel;
use crate::section::Content;
use crate::section::Section;
use crate::theme::*;
use crate::widgets::*;

// The outfit section of the Character tab: a tile per skin of the suite, and the worn skin's
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
                show_thumbnails.run_if(
                    resource_changed::<Thumbnails>.or_eager(any_match_filter::<Added<TilePicture>>),
                ),
                fill_objects.run_if(focused_changed::<SkinObjects>.or_eager(new_panel)),
                show_objects.run_if(
                    focused_changed::<CharacterState>
                        .or_eager(focused_changed::<SkinObjects>)
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

// Where the skin's picture goes, a placeholder until it's taken.
#[derive(Component)]
struct TilePicture(String);

// In a tile's picture until the picture is taken.
#[derive(Component)]
pub(super) struct TilePlaceholder;

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
            crate::section::subsection(
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
                let name = skin.name.clone();
                grid.spawn(picture_tile(
                    &skin.name,
                    Icon::Outfit,
                    SkinTile(name.clone()),
                    TilePicture(name),
                ));
            }
        });
    }
}

// A picture over a name, such as an outfit's or a character's, with `placeholder` until the
// picture is taken. The tile is the button, with `action` saying what a click does; the parts
// inside ignore the pointer. `picture` marks where the picture goes.
pub(super) fn picture_tile(
    name: &str,
    placeholder: Icon,
    action: impl Bundle,
    picture: impl Bundle,
) -> impl Bundle {
    let picture = (
        Node {
            width: Val::Percent(100.0),
            aspect_ratio: Some(1.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(INNER_RADIUS),
            ..default()
        },
        Fill(WELL_BG),
        Pickable::IGNORE,
        picture,
        children![(icon(placeholder, FontSize::Px(28.0), TRACK), TilePlaceholder)],
    );
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.5)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            min_width: Val::ZERO,
            ..default()
        },
        BaseBackground::default(),
        Fill(BUTTON_BG),
        Stroke(CLEAR),
        action,
        children![
            picture,
            (
                label(name, SMALL_SIZE, TEXT_LABEL),
                TextLayout::no_wrap(),
                Node {
                    min_width: Val::ZERO,
                    overflow: Overflow::clip(),
                    margin: UiRect::new(Val::Px(2.0), Val::Px(2.0), Val::ZERO, Val::Px(2.0)),
                    ..default()
                },
            ),
        ],
    )
}

// A tile's picture, once there is one, in place of its placeholder.
pub(super) fn show_picture(
    commands: &mut Commands,
    (entity, children, shown): (Entity, &Children, bool),
    image: &Handle<Image>,
    placeholders: &mut Query<&mut Node, With<TilePlaceholder>>,
) {
    if !shown {
        commands.entity(entity).insert(ImageNode::new(image.clone()));
    }
    for &child in &**children {
        if let Ok(mut node) = placeholders.get_mut(child)
            && node.display != Display::None
        {
            node.display = Display::None;
        }
    }
}

// A picture the app has taken replaces the placeholder. Taking them is up to the scene,
// which does so in the background when a suite first shows.
fn show_thumbnails(
    mut commands: Commands,
    suite: Focused<ActiveSuite>,
    thumbnails: Res<Thumbnails>,
    pictures: Query<(Entity, &TilePicture, &Children, Has<ImageNode>)>,
    mut placeholders: Query<&mut Node, With<TilePlaceholder>>,
) {
    let Some(suite) = suite.get() else {
        return;
    };
    for (entity, picture, children, shown) in &pictures {
        if let Some(image) = thumbnails.outfit(&suite.root, &picture.0) {
            show_picture(&mut commands, (entity, children, shown), image, &mut placeholders);
        }
    }
}

fn style_skins(state: Focused<CharacterState>, tiles: Query<(&SkinTile, &mut Fill, &mut Stroke)>) {
    let Some(state) = state.get() else {
        return;
    };
    for (tile, mut fill, mut border) in tiles {
        let worn = state.skin.as_ref() == Some(&tile.0);
        fill.set_if_neq(Fill(if worn { SELECTED_BG } else { BUTTON_BG }));
        border.set_if_neq(Stroke(if worn { ACCENT } else { CLEAR }));
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
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(12.0),
                        height: ROW_HEIGHT,
                        ..default()
                    },
                    Pickable::IGNORE,
                    children![
                        label(name, BODY_SIZE, TEXT_LABEL),
                        switch_track(ObjectSwitch(name.to_string())),
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
    mut lists: Query<&mut Node, (With<ObjectList>, Without<SwitchKnob>)>,
    mut switches: Query<(&ObjectSwitch, &Children, &mut Fill), Without<SwitchKnob>>,
    mut knobs: Query<(&mut Node, &mut Fill), With<SwitchKnob>>,
) {
    let (Some(state), Some(objects)) = (state.get(), objects.get()) else {
        return;
    };
    for mut node in &mut lists {
        node.display = display(objects.names().count() > 1);
    }
    let hidden = state.skin.as_ref().and_then(|skin| state.hidden.get(skin));
    for (switch, children, track) in &mut switches {
        let shown = !hidden.is_some_and(|hidden| hidden.contains(&switch.0));
        if let Some(Ok((knob, ink))) = children.first().map(|&knob| knobs.get_mut(knob)) {
            paint_switch(shown, track, knob, ink);
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
