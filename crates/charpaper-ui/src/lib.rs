mod arrows;
mod elements;
mod fonts;
mod held;
mod help;
mod locale;
mod overlay;
mod panel;
mod rows;
mod tabs;
mod theme;
mod wheel;
mod widgets;

use bevy::prelude::*;
use charpaper_settings::UiState;

pub use locale::Locale;
pub use locale::UiConfig;

// Whether the pointer is on the panel or on the button that brings it back. The app keeps the
// frame rate up meanwhile, so the panel answers at once whatever the scene's own pace.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelHovered(pub bool);

// The collapsible settings panel on the desktop. The frame and every tab register their
// own systems; a new tab is a module in `tabs/` plus a line here and in panel.rs.
pub struct SettingsPanelPlugin {
    pub config: UiConfig,
}

impl Plugin for SettingsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
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
                tabs::character::CharacterPlugin,
                tabs::look::LookPlugin,
                tabs::graphics::GraphicsPlugin,
                tabs::sound::SoundPlugin,
                tabs::system::SystemTabPlugin,
                wheel::WheelPlugin,
                held::HeldPlugin,
                arrows::ArrowsPlugin,
                rows::RowsPlugin,
                help::HelpPlugin,
            ));
    }
}
