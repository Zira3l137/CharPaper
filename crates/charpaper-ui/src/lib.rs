mod elements;
mod locale;
mod panel;
mod tabs;
mod theme;
mod widgets;

use bevy::prelude::*;
use i18n_embed_fl::fl;
use serde::Deserialize;
use serde::Serialize;

pub use locale::Locale;
pub use locale::UiConfig;

use crate::locale::Tr;

// Only tabs with something working behind them exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tab {
    #[default]
    Character,
    Scene,
    Render,
    System,
}

impl Tab {
    pub(crate) fn title(self) -> Tr {
        match self {
            Tab::Character => |l| fl!(l, "tab-character"),
            Tab::Scene => |l| fl!(l, "tab-scene"),
            Tab::Render => |l| fl!(l, "tab-render"),
            Tab::System => |l| fl!(l, "tab-system"),
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
    // A tag such as `de-DE`, for a translation in `locales/`. None follows the system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

// The collapsible settings panel on the desktop. The frame and every tab register their
// own systems; a new tab is a module in `tabs/` plus a line here and in panel.rs.
pub struct SettingsPanelPlugin {
    pub config: UiConfig,
    pub state: UiState,
}

impl Plugin for SettingsPanelPlugin {
    fn build(&self, app: &mut App) {
        // Whatever started the app in a language, a flag included, becomes the one chosen.
        let state = UiState { language: self.config.language.clone(), ..self.state.clone() };
        app.insert_resource(locale::Locale::load(&self.config))
            .insert_resource(locale::Languages::find(&self.config.locales_dir))
            .insert_resource(self.config.clone())
            .insert_resource(state)
            .add_systems(
                Update,
                (locale::switch_language.run_if(resource_changed::<UiState>), locale::relabel)
                    .chain(),
            )
            .add_plugins((
                panel::PanelPlugin,
                tabs::character::CharacterTabPlugin,
                tabs::scene::SceneTabPlugin,
                tabs::render::RenderTabPlugin,
                tabs::system::SystemTabPlugin,
            ));
    }
}
