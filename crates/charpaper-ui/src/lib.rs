mod fonts;
mod held;
mod locale;
mod overlay;
mod panel;
mod row;
mod section;
mod tabs;
mod theme;
mod widgets;

use bevy::prelude::*;
use charpaper_settings::StartAtLogin;
use charpaper_settings::SystemSettings;
use charpaper_settings::UiState;

pub use locale::Locale;
pub use locale::UiConfig;

// Whether the pointer is on the panel or on the button that brings it back. The app keeps the
// frame rate up meanwhile, so the panel answers at once whatever the scene's own pace.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelHovered(pub bool);

// The collapsible settings panel on the desktop. A page of it is a module in `tabs/`, which lays
// out sections of rows; a new tab is such a module plus a line in panel.rs.
pub struct SettingsPanelPlugin {
    pub config: UiConfig,
}

impl Plugin for SettingsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .init_resource::<SystemSettings>()
            .init_resource::<StartAtLogin>()
            .insert_resource(locale::Locale::load(&self.config))
            .insert_resource(locale::Languages::find(&self.config.locales_dir))
            .insert_resource(self.config.clone())
            .add_systems(
                Update,
                (locale::switch_language.run_if(resource_changed::<UiState>), locale::relabel)
                    .chain(),
            )
            .add_plugins((
                fonts::FontsPlugin,
                panel::PanelPlugin,
                overlay::OverlayPlugin,
                section::SectionsPlugin,
                row::RowsPlugin,
                held::HeldPlugin,
                tabs::TabsPlugin,
            ));
    }
}
