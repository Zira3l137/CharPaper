use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::AvailableSuites;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;
use charpaper_scene::Expressions;
use charpaper_scene::GazeSettings;
use charpaper_scene::SkinObjects;

use crate::UiConfig;
use crate::UiState;
use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::locale::UiLocale;
use crate::theme::*;
use crate::widgets::*;

// Which suite, skin, animation and expression. Clicks write to CharacterState and the
// widgets restyle themselves from it, so nothing here keeps a copy.
pub(crate) struct CharacterTabPlugin;

impl Plugin for CharacterTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
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
                    resource_changed::<CharacterState>.or_eager(resource_changed::<Expressions>),
                ),
                show_gaze.run_if(
                    resource_changed::<CharacterState>.or_eager(resource_changed::<GazeSettings>),
                ),
                show_objects.run_if(
                    resource_changed::<UiState>
                        .or_eager(resource_changed::<CharacterState>)
                        .or_eager(resource_changed::<SkinObjects>),
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

pub(crate) fn page(locale: &UiLocale) -> impl Bundle {
    children![
        section(
            locale.get_or("section.suite", "CHARACTER"),
            Section::Suite,
            cycler(locale.get_or("label.suite", "Character"), Cycler::Suite),
        ),
        section(
            locale.get_or("section.outfit", "OUTFIT"),
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
                    cycler(locale.get_or("label.advanced", "Advanced"), Cycler::Advanced),
                    (
                        Node {
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                        UiContainer::ObjectList,
                    ),
                ],
            ),
        ),
        section(
            locale.get_or("section.animation", "ANIMATION"),
            Section::Animation,
            cycler(locale.get_or("label.animation", "Animation"), Cycler::Animation),
        ),
        section(
            locale.get_or("section.expression", "EXPRESSION"),
            Section::Expression,
            cycler(locale.get_or("label.expression", "Expression"), Cycler::Expression),
        ),
        section(
            locale.get_or("section.gaze", "GAZE"),
            Section::Gaze,
            cycler(locale.get_or("label.follow_cursor", "Follow cursor"), Cycler::FollowCursor),
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
            UiContainer::Section(Section::Outfit) => node.display = display(!suite.skins.is_empty()),
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
    config: Res<UiConfig>,
    mut sections: Query<(&UiContainer, &mut Node)>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    let shown = !expressions.0.is_empty();
    show_container(&mut sections, UiContainer::Section(Section::Expression), shown);
    let neutral = config.locale.get_or("expression.neutral", "Neutral");
    set_value(&mut values, Cycler::Expression, state.expression().unwrap_or(neutral));
}

// Only for a suite whose suite.toml says which bones follow the cursor.
fn show_gaze(
    suite: Option<Res<ActiveSuite>>,
    settings: Res<GazeSettings>,
    config: Res<UiConfig>,
    mut sections: Query<(&UiContainer, &mut Node)>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    let shown = suite.is_some_and(|suite| suite.gaze.is_some());
    show_container(&mut sections, UiContainer::Section(Section::Gaze), shown);
    set_value(&mut values, Cycler::FollowCursor, on_off(&config.locale, settings.follow_cursor));
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
    ui: Res<UiState>,
    state: Res<CharacterState>,
    objects: Res<SkinObjects>,
    config: Res<UiConfig>,
    mut nodes: Query<(&UiContainer, &mut Node)>,
    buttons: Query<(&UiButton, &Children)>,
    mut texts: Query<&mut Text>,
    values: Query<(Entity, &CyclerValue)>,
) {
    let locale = &config.locale;
    let several = objects.names().count() > 1;
    show_container(&mut nodes, UiContainer::Row(Cycler::Advanced), several);
    show_container(&mut nodes, UiContainer::ObjectList, several && ui.advanced_outfit);

    let hidden = state.skin.as_ref().and_then(|skin| state.hidden.get(skin));
    for (button, children) in &buttons {
        let UiButton::Object(name) = button else {
            continue;
        };
        let shown = !hidden.is_some_and(|h| h.contains(name));
        for &child in &**children {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = on_off(locale, shown).into();
            }
        }
    }
    for (entity, value) in &values {
        if value.0 == Cycler::Advanced {
            if let Ok(mut text) = texts.get_mut(entity) {
                text.0 = on_off(locale, ui.advanced_outfit).into();
            }
        }
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    clips: Option<Res<CharacterClips>>,
    available: Res<AvailableSuites>,
    expressions: Res<Expressions>,
    mut state: ResMut<CharacterState>,
    mut ui: ResMut<UiState>,
    mut gaze: ResMut<GazeSettings>,
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

    let Some((cycler, forward)) = button.step() else {
        return;
    };
    match cycler {
        Cycler::Advanced => ui.advanced_outfit ^= true,
        Cycler::FollowCursor => gaze.follow_cursor ^= true,
        Cycler::Suite => {
            let options: Vec<Option<String>> = available.0.iter().cloned().map(Some).collect();
            if let Some(next) = step(&options, &state.suite, forward) {
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
            if let Some(next) = step(&options, &current, forward) {
                state.expressions.insert(skin, next);
            }
        }
        Cycler::Animation => {
            let Some(clips) = clips else {
                return;
            };
            let options: Vec<Option<String>> = clips.names().map(|n| Some(n.to_string())).collect();
            if let Some(next) = step(&options, &state.animation, forward) {
                state.animation = next;
            }
        }
        _ => {}
    }
}
