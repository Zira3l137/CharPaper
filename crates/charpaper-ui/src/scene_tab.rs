//! The Scene tab: which camera looks at the character and which environment
//! surrounds it.

use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;

use crate::Tab;
use crate::UiConfig;
use crate::UiState;
use crate::config::UiLocale;
use crate::helpers::Cycler;
use crate::helpers::CyclerValue;
use crate::helpers::Section;
use crate::helpers::UiButton;
use crate::helpers::UiContainer;
use crate::helpers::UiElement;
use crate::pages::step;
use crate::widgets::*;

pub(crate) fn scene_page(locale: &UiLocale) -> impl Bundle {
    children![
        section(
            locale.get_or("section.camera", "CAMERA"),
            UiContainer::Section(Section::Camera),
            cycler(locale.get_or("label.camera", "Camera"), Cycler::Camera),
        ),
        section(
            locale.get_or("section.environment", "ENVIRONMENT"),
            UiContainer::Section(Section::Environment),
            cycler(locale.get_or("label.environment", "Environment"), Cycler::Environment),
        ),
    ]
}

/// A cycler with a single option has nothing to pick, so each section only
/// shows with two or more, and the tab only with at least one section. A
/// Scene tab remembered from another suite then falls back to the first tab.
pub(crate) fn show_scene_tab(
    suite: Res<ActiveSuite>,
    nodes: Query<(&UiElement, &mut Node)>,
    mut ui: ResMut<UiState>,
) {
    let cameras = !suite.cameras.is_empty();
    let environments = suite.environments.len() > 1;
    if !cameras && !environments {
        if ui.tab == Tab::Scene {
            ui.tab = Tab::default();
        }
        return;
    }
    for (element, mut node) in nodes {
        let shown = match element {
            UiElement::Container(UiContainer::Section(Section::Camera)) => cameras,
            UiElement::Container(UiContainer::Section(Section::Environment)) => environments,
            UiElement::Container(UiContainer::TabBar)
            | UiElement::Button(UiButton::Tab(Tab::Scene)) => true,
            _ => continue,
        };
        node.display = if shown { Display::Flex } else { Display::None };
    }
}

pub(crate) fn show_scene_values(
    state: Res<CharacterState>,
    config: Res<UiConfig>,
    values: Query<(&CyclerValue, &mut Text)>,
) {
    let orbit = config.locale.get_or("camera.orbit", "Orbit");
    for (value, mut text) in values {
        text.0 = match value.0 {
            Cycler::Camera => state.camera.clone().unwrap_or_else(|| orbit.to_string()),
            Cycler::Environment => state.environment.clone().unwrap_or_else(|| "-".into()),
            _ => continue,
        };
    }
}

pub(crate) fn on_scene_tab_click(
    event: On<Pointer<Click>>,
    elements: Query<&UiElement>,
    suite: Option<Res<ActiveSuite>>,
    mut state: ResMut<CharacterState>,
) {
    let (Ok(UiElement::Button(button)), Some(suite)) = (elements.get(event.entity), suite) else {
        return;
    };
    let (cycler, forward) = match button {
        UiButton::Previous(cycler) => (*cycler, false),
        UiButton::Next(cycler) => (*cycler, true),
        _ => return,
    };
    match cycler {
        Cycler::Camera => {
            // `None` is the orbit camera, which every suite has.
            let options: Vec<Option<String>> = std::iter::once(None)
                .chain(suite.cameras.iter().map(|c| Some(c.name.clone())))
                .collect();
            if let Some(next) = step(&options, &state.camera, forward) {
                state.camera = next;
            }
        }
        Cycler::Environment => {
            let options: Vec<Option<String>> =
                suite.environments.iter().map(|e| Some(e.name.clone())).collect();
            if let Some(next) = step(&options, &state.environment, forward) {
                state.environment = next;
            }
        }
        _ => {}
    }
}
