mod config;
mod helpers;

use bevy::prelude::*;
pub use config::UiConfig;

const BTN_PADDING: f32 = 10.0;
const BTN_BG_COLOR: (f32, f32, f32, f32) = (0.1, 0.8, 0.1, 1.0);
const BTN_TEXT_SIZE: f32 = 16.0;
const BTN_SIZE: (f32, f32) = (200.0, 50.0);

#[derive(Component, Debug)]
pub enum UiButton {
    Exit,
}

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
    info!("spawning ui");
    let locale = &config.locale;
    commands.spawn(helpers::ui_root()).with_children(|parent| {
        parent
            .spawn(helpers::button(
                BTN_SIZE,
                locale.get("exit"),
                BTN_TEXT_SIZE,
                BTN_BG_COLOR,
                BTN_PADDING,
                UiButton::Exit,
            ))
            .observe(on_click);
    });
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    mut exit: MessageWriter<AppExit>,
) {
    info!("click event received");
    if let Ok(button) = buttons.get(event.entity) {
        info!("button clicked: {:?}", button);
        match button {
            UiButton::Exit => exit.write(AppExit::Success),
        };
    }
}
