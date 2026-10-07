use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::AvailableSuites;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;
use charpaper_scene::Expressions;
use charpaper_scene::GazeSettings;
use charpaper_scene::SkinObjects;
use i18n_embed_fl::fl;

use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::Step;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::locale::Locale;
use crate::theme::*;
use crate::widgets::*;

// Which suite, skin, animation and expression. Clicks write to CharacterState and the
// widgets restyle themselves from it, so nothing here keeps a copy.
pub(crate) struct CharacterTabPlugin;

impl Plugin for CharacterTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_observer(on_step).add_systems(
            Update,
            (
                fill_outfits.run_if(resource_added::<ActiveSuite>),
                show_suite.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_changed::<AvailableSuites>),
                ),
                style_outfits.run_if(resource_changed::<CharacterState>),
                fill_objects.run_if(resource_changed::<SkinObjects>),
                show_expression.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_changed::<Expressions>)
                        .or_eager(resource_changed::<Locale>),
                ),
                show_gaze.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_changed::<GazeSettings>)
                        .or_eager(resource_changed::<Locale>),
                ),
                show_objects.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_changed::<SkinObjects>)
                        .or_eager(resource_changed::<Locale>),
                ),
                // or_eager keeps resource_added evaluated every frame. Skipped on a frame with a
                // change, it would report the clips as new once more on the next one.
                show_animation.run_if(
                    resource_changed::<CharacterState>.or_eager(resource_added::<CharacterClips>),
                ),
            )
                .chain(),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![
        section(
            |l| fl!(l, "section-suite"),
            Section::Suite,
            cycler(|l| fl!(l, "label-suite"), Cycler::Suite),
        ),
        section(
            |l| fl!(l, "section-outfit"),
            Section::Outfit,
            (
                Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() },
                Pickable::IGNORE,
                children![
                    (
                        Node {
                            display: Display::Grid,
                            grid_template_columns: vec![RepeatedGridTrack::flex(3, 1.0)],
                            row_gap: Val::Px(8.0),
                            column_gap: Val::Px(8.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                        UiContainer::OutfitGrid,
                    ),
                    section(
                        |l| fl!(l, "section-parts"),
                        Section::Parts,
                        (
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                            UiContainer::ObjectList,
                        ),
                    ),
                ],
            ),
        ),
        section(
            |l| fl!(l, "section-animation"),
            Section::Animation,
            cycler(|l| fl!(l, "label-animation"), Cycler::Animation),
        ),
        section(
            |l| fl!(l, "section-expression"),
            Section::Expression,
            cycler(|l| fl!(l, "label-expression"), Cycler::Expression),
        ),
        section(
            |l| fl!(l, "section-gaze"),
            Section::Gaze,
            cycler(|l| fl!(l, "label-follow-cursor"), Cycler::FollowCursor),
        ),
    ]
}

// One tile per skin, rebuilt for each suite.
fn fill_outfits(
    mut commands: Commands,
    suite: Res<ActiveSuite>,
    containers: Query<(Entity, &UiContainer, &mut Node)>,
) {
    for (entity, container, mut node) in containers {
        match container {
            UiContainer::Section(Section::Outfit) => {
                node.display = display(!suite.skins.is_empty())
            }
            UiContainer::OutfitGrid => {
                commands.entity(entity).despawn_children().with_children(|grid| {
                    for skin in &suite.skins {
                        grid.spawn(
                            button(&skin.name)
                                .font_size(SMALL_SIZE)
                                .node(|n| n.height = Val::Px(40.0))
                                .build(UiButton::Skin(skin.name.clone())),
                        );
                    }
                });
            }
            _ => {}
        }
    }
}

fn style_outfits(
    state: Res<CharacterState>,
    tiles: Query<(&UiButton, &mut BaseBackground, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (button, mut base, mut background, mut border) in tiles {
        let UiButton::Skin(name) = button else {
            continue;
        };
        let worn = state.skin.as_ref() == Some(name);
        paint(&mut base, &mut background, if worn { SELECTED_BG } else { BUTTON_BG });
        *border = BorderColor::all(if worn { ACCENT } else { BORDER });
    }
}

// Only with two or more suites to pick from.
fn show_suite(
    state: Res<CharacterState>,
    available: Res<AvailableSuites>,
    mut sections: Query<(&UiContainer, &mut Node)>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    show_container(&mut sections, UiContainer::Section(Section::Suite), available.0.len() > 1);
    set_value(&mut values, Cycler::Suite, state.suite.as_deref().unwrap_or("-"));
}

// Expressions arrive with the worn skin's file, so this follows every skin change.
fn show_expression(
    state: Res<CharacterState>,
    expressions: Res<Expressions>,
    locale: Res<Locale>,
    mut sections: Query<(&UiContainer, &mut Node)>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    let shown = !expressions.0.is_empty();
    show_container(&mut sections, UiContainer::Section(Section::Expression), shown);
    let neutral = fl!(locale, "expression-neutral");
    set_value(&mut values, Cycler::Expression, state.expression().unwrap_or(&neutral));
}

// Only for a suite whose suite.toml says which bones follow the cursor.
fn show_gaze(
    suite: Option<Res<ActiveSuite>>,
    settings: Res<GazeSettings>,
    locale: Res<Locale>,
    mut sections: Query<(&UiContainer, &mut Node)>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    let shown = suite.is_some_and(|suite| suite.gaze.is_some());
    show_container(&mut sections, UiContainer::Section(Section::Gaze), shown);
    set_value(&mut values, Cycler::FollowCursor, &on_off(&locale, settings.follow_cursor));
}

// Clips arrive a moment after the suite, once their files have loaded.
fn show_animation(
    state: Res<CharacterState>,
    clips: Option<Res<CharacterClips>>,
    mut sections: Query<(&UiContainer, &mut Node)>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    let any = clips.is_some_and(|clips| clips.names().next().is_some());
    show_container(&mut sections, UiContainer::Section(Section::Animation), any);
    set_value(&mut values, Cycler::Animation, state.animation.as_deref().unwrap_or("-"));
}

// One row per mesh object of the worn skin.
fn fill_objects(
    mut commands: Commands,
    objects: Res<SkinObjects>,
    containers: Query<(Entity, &UiContainer)>,
) {
    let Some((list, _)) = containers.iter().find(|(_, c)| **c == UiContainer::ObjectList) else {
        return;
    };
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
                        .build(UiButton::Object(name.to_string())),
                ],
            ));
        }
    });
}

// Only with two or more objects: switching off a skin's only object is switching off the skin.
fn show_objects(
    state: Res<CharacterState>,
    objects: Res<SkinObjects>,
    locale: Res<Locale>,
    mut nodes: Query<(&UiContainer, &mut Node)>,
    buttons: Query<(&UiButton, &Children)>,
    mut texts: Query<&mut Text>,
) {
    let several = objects.names().count() > 1;
    show_container(&mut nodes, UiContainer::Section(Section::Parts), several);

    let hidden = state.skin.as_ref().and_then(|skin| state.hidden.get(skin));
    for (button, children) in &buttons {
        let UiButton::Object(name) = button else {
            continue;
        };
        let shown = !hidden.is_some_and(|h| h.contains(name));
        for &child in &**children {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = on_off(&locale, shown).into();
            }
        }
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    mut state: ResMut<CharacterState>,
) {
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    match button {
        UiButton::Skin(name) => state.skin = Some(name.clone()),
        UiButton::Object(name) => {
            let Some(skin) = state.skin.clone() else {
                return;
            };
            let hidden = state.hidden.entry(skin).or_default();
            if !hidden.remove(name) {
                hidden.insert(name.clone());
            }
        }
        _ => {}
    }
}

fn on_step(
    event: On<Step>,
    clips: Option<Res<CharacterClips>>,
    available: Res<AvailableSuites>,
    expressions: Res<Expressions>,
    mut state: ResMut<CharacterState>,
    mut gaze: ResMut<GazeSettings>,
) {
    let (by, wrap) = (event.by, event.wraps());
    match event.cycler {
        Cycler::FollowCursor => gaze.follow_cursor = flip(gaze.follow_cursor, by, wrap),
        Cycler::Suite => {
            let options: Vec<Option<String>> = available.0.iter().cloned().map(Some).collect();
            if let Some(next) = step(&options, &state.suite, by, wrap) {
                state.suite = next;
            }
        }
        Cycler::Expression => {
            let Some(skin) = state.skin.clone() else {
                return;
            };
            // Neutral is an empty name; see CharacterState::expressions.
            let options: Vec<String> =
                std::iter::once(String::new()).chain(expressions.0.iter().cloned()).collect();
            let current = state.expression().unwrap_or_default().to_string();
            if let Some(next) = step(&options, &current, by, wrap) {
                state.expressions.insert(skin, next);
            }
        }
        Cycler::Animation => {
            let Some(clips) = clips else {
                return;
            };
            let options: Vec<Option<String>> = clips.names().map(|n| Some(n.to_string())).collect();
            if let Some(next) = step(&options, &state.animation, by, wrap) {
                state.animation = next;
            }
        }
        _ => {}
    }
}
