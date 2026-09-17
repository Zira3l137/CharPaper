mod config;
mod helpers;

use bevy::prelude::*;
pub use config::UiConfig;

use crate::helpers::ButtonBuilder;
use crate::helpers::ButtonReactiveExt;
use crate::helpers::UiButton;
use crate::helpers::defaults::*;

pub struct CustomUiPlugin {
    pub config: UiConfig,
}

impl Plugin for CustomUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone());
        app.add_systems(Startup, spawn_ui);
    }
}

pub fn spawn_ui(mut commands: Commands, config: Res<UiConfig>) {
    let locale = &config.locale;
    commands.spawn(helpers::ui_root()).with_children(|parent| {
        parent
            .spawn(
                ButtonBuilder::default()
                    .text(locale.get("exit"))
                    .bg_color(BTN_BG_COLOR)
                    .font_size(BTN_TEXT_SIZE)
                    .padding(BTN_PADDING)
                    .border_radius(BTN_RADIUS)
                    .align_items(AlignItems::Center)
                    .align_self(AlignSelf::Center)
                    .justify_content(JustifyContent::Center)
                    .justify_self(JustifySelf::Center)
                    .build_marked(UiButton::Exit),
            )
            .with_button_feedback()
            .observe(on_down);
    });
}

fn on_down(event: On<Pointer<Click>>, buttons: Query<&UiButton>, mut exit: MessageWriter<AppExit>) {
    if let Ok(button) = buttons.get(event.entity) {
        match button {
            UiButton::Exit => {
                info!("Exiting application normally");
                exit.write(AppExit::Success)
            }
        };
    }
}
