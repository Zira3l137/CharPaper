// A module per tab, each laying out its page of sections and rows. Rows need nothing else; a
// tab's plugin only keeps up the few parts that are not rows, such as the outfit grid.

pub(crate) mod graphics;
mod look;
pub(crate) mod outfit;
pub(crate) mod scene;
pub(crate) mod sound;
pub(crate) mod system;

use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::AvailableSuites;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;
use charpaper_scene::Expressions;
use charpaper_scene::FocusedStage;
use charpaper_scene::Stage;
use charpaper_scene::StageKey;
use charpaper_settings::GazeSettings;
use charpaper_settings::Graphics;
use charpaper_settings::Monitors;
use charpaper_settings::RenderSettings;
use charpaper_settings::ScreenSettings;
use charpaper_settings::UiState;
use charpaper_settings::Volumes;

use crate::held::Held;
use crate::locale::Locale;
use crate::row::Row;

pub(crate) struct TabsPlugin;

impl Plugin for TabsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((outfit::OutfitPlugin, graphics::GraphicsPlugin, system::SystemPlugin));
    }
}

type StageSources = (
    Changed<CharacterState>,
    Changed<ActiveSuite>,
    Changed<ActiveLook>,
    Changed<Graphics>,
    Changed<GazeSettings>,
    Changed<Expressions>,
    Changed<CharacterClips>,
    Changed<StageKey>,
);

// Whether anything the rows read from has changed, which is when they work out what they show
// again. A row reading from something new adds it here. Moving the focus to another stage
// changes all that the stage gives them.
pub(crate) fn sources_changed(
    focus: Res<FocusedStage>,
    stages: Query<(), (With<Stage>, Or<StageSources>)>,
    settings: (Res<UiState>, Res<RenderSettings>, Res<ScreenSettings>, Res<Volumes>),
    world: (Res<Monitors>, Res<AvailableSuites>, Res<Held>, Res<Locale>),
    new_rows: Query<(), Added<Row>>,
) -> bool {
    let (ui, render, screen, volumes) = settings;
    let (monitors, suites, held, locale) = world;
    focus.is_changed()
        || focus.0.is_some_and(|stage| stages.contains(stage))
        || ui.is_changed()
        || render.is_changed()
        || screen.is_changed()
        || volumes.is_changed()
        || monitors.is_changed()
        || suites.is_changed()
        || held.is_changed()
        || locale.is_changed()
        || !new_rows.is_empty()
}
