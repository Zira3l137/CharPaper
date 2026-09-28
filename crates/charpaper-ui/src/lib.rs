mod elements;
mod locale;
mod panel;
mod tabs;
mod theme;
mod widgets;

use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub use locale::UiConfig;
pub use locale::UiLocale;

// Only tabs with something working behind them exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tab {
    #[default]
    Character,
    Scene,
    Render,
}

impl Tab {
    pub(crate) fn title(self, locale: &UiLocale) -> &str {
        match self {
            Tab::Character => locale.get_or("tab.character", "Character"),
            Tab::Scene => locale.get_or("tab.scene", "Scene"),
            Tab::Render => locale.get_or("tab.render", "Render"),
        }
    }
}

// Saved between runs by the app, so the panel reopens as it was left.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    pub is_menu_closed: bool,
    pub tab: Tab,
    pub advanced_outfit: bool,
}

// The collapsible settings panel on the desktop. The frame and every tab register their
// own systems; a new tab is a module in `tabs/` plus a line here and in panel.rs.
pub struct SettingsPanelPlugin {
    pub config: UiConfig,
    pub state: UiState,
}

impl Plugin for SettingsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone()).insert_resource(self.state.clone()).add_plugins((
            panel::PanelPlugin,
            tabs::character::CharacterTabPlugin,
            tabs::scene::SceneTabPlugin,
            tabs::render::RenderTabPlugin,
        ));
    }
}
