//! The Character tab, and how clicking on it changes the scene.
//!
//! The scene owns the truth in [`CharacterState`]: clicks write to it, and the
//! widgets restyle themselves from it. Nothing here keeps a copy, so a change
//! made elsewhere (a config reload, later a hotkey) shows up here too.

use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;

use crate::StatusText;
use crate::config::UiLocale;
use crate::helpers::BaseBackground;
use crate::helpers::Cycler;
use crate::helpers::CyclerValue;
use crate::helpers::Section;
use crate::helpers::UiButton;
use crate::helpers::UiContainer;
use crate::helpers::UiElement;
use crate::helpers::UiNode;
use crate::helpers::WithText;
use crate::theme::*;
use crate::widgets::*;

pub(crate) fn character_page(locale: &UiLocale) -> impl Bundle {
    children![
        section(
            locale.get_or("section.outfit", "OUTFIT"),
            UiContainer::Section(Section::Outfit),
            (
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![RepeatedGridTrack::flex(3, 1.0)],
                    row_gap: Val::Px(8.0),
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                Pickable::IGNORE,
                UiElement::Container(UiContainer::OutfitGrid),
            ),
        ),
        section(
            locale.get_or("section.animation", "ANIMATION"),
            UiContainer::Section(Section::Animation),
            cycler(locale.get_or("label.animation", "Animation"), Cycler::Animation),
        ),
    ]
}

/// Skins are known as soon as the suite is, so their tiles are made once.
pub(crate) fn fill_outfits(
    mut commands: Commands,
    suite: Res<ActiveSuite>,
    containers: Query<(Entity, &UiElement, &mut Node)>,
    mut status: Query<&mut Text, With<StatusText>>,
) {
    for mut text in &mut status {
        text.0 = suite.name.clone();
    }
    if suite.skins.is_empty() {
        return;
    }

    for (entity, element, mut node) in containers {
        match element {
            UiElement::Container(UiContainer::Section(Section::Outfit)) => {
                node.display = Display::Flex;
            }
            UiElement::Container(UiContainer::OutfitGrid) => {
                commands.entity(entity).with_children(|grid| {
                    for skin in &suite.skins {
                        grid.spawn(
                            button(&skin.name)
                                .font_size(SMALL_SIZE)
                                .edit_node(|n| n.height = Val::Px(40.0))
                                .build_with(UiElement::Button(UiButton::Skin(skin.name.clone()))),
                        );
                    }
                });
            }
            _ => {}
        }
    }
}

pub(crate) fn style_outfits(
    state: Res<CharacterState>,
    tiles: Query<(&UiElement, &mut BaseBackground, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (element, mut base, mut background, mut border) in tiles {
        let UiElement::Button(UiButton::Skin(name)) = element else {
            continue;
        };
        let worn = state.skin.as_ref() == Some(name);
        paint(&mut base, &mut background, if worn { SELECTED_BG } else { BUTTON_BG });
        *border = BorderColor::all(if worn { ACCENT } else { BORDER });
    }
}

/// Clips arrive a moment after the suite, once their files have loaded, so
/// the section appears then rather than at startup.
pub(crate) fn show_animation(
    state: Res<CharacterState>,
    clips: Option<Res<CharacterClips>>,
    mut sections: Query<(&UiElement, &mut Node)>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    let any = clips.is_some_and(|clips| clips.names().next().is_some());
    for (element, mut node) in &mut sections {
        if *element == UiElement::Container(UiContainer::Section(Section::Animation)) {
            node.display = if any { Display::Flex } else { Display::None };
        }
    }
    for (value, mut text) in &mut values {
        if value.0 == Cycler::Animation {
            text.0 = state.animation.clone().unwrap_or_else(|| "-".into());
        }
    }
}

/// Handles the clicks that change the scene. The panel's own buttons (tabs,
/// collapse, quit) stay in `on_button_click`.
pub(crate) fn on_scene_click(
    event: On<Pointer<Click>>,
    elements: Query<&UiElement>,
    clips: Option<Res<CharacterClips>>,
    mut state: ResMut<CharacterState>,
) {
    let Ok(UiElement::Button(button)) = elements.get(event.entity) else {
        return;
    };
    match button {
        UiButton::Skin(name) => state.skin = Some(name.clone()),
        UiButton::Previous(Cycler::Animation) | UiButton::Next(Cycler::Animation) => {
            let Some(clips) = clips else {
                return;
            };
            let options: Vec<Option<String>> = clips.names().map(|n| Some(n.to_string())).collect();
            let forward = matches!(button, UiButton::Next(_));
            if let Some(next) = step(&options, &state.animation, forward) {
                state.animation = next;
            }
        }
        _ => {}
    }
}

/// The option before or after `current`, wrapping around. An unknown or unset
/// `current` steps to the first option.
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
