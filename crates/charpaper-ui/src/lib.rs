mod config;
mod helpers;

use bevy::prelude::*;
pub use config::UiConfig;

use crate::helpers::ButtonBuilder;
use crate::helpers::ButtonReactiveExt;
use crate::helpers::ContainerBuilder;
use crate::helpers::UiButton;
use crate::helpers::UiNode;
use crate::helpers::WithBackground;
use crate::helpers::WithText;
use crate::helpers::defaults::*;

pub struct CustomUiPlugin {
    pub config: UiConfig,
}

impl Plugin for CustomUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone());
        app.add_systems(Startup, spawn_ui);
        app.add_observer(on_exit);
    }
}

pub fn spawn_ui(mut commands: Commands, config: Res<UiConfig>) {
    let locale = &config.locale;
    commands
        // NOTE: Root node, with a row flex direction
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::End,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(
                    // NOTE: Menu container, fixed width and full height, with a column flex direction
                    ContainerBuilder::default()
                        .width(Val::Px(256.0))
                        .height(Val::Percent(100.0))
                        .bg_color(CONTAINER_BG_COLOR)
                        .padding(CONTAINER_PADDING)
                        .edit_node(|n| n.flex_direction = FlexDirection::Column)
                        .bg_shadow_color(Color::linear_rgba(0.0, 0.0, 0.0, 0.50))
                        .bg_shadow_offset(Val::Px(-2.0), Val::Px(0.0))
                        .bg_shadow_blur_radius(Val::Px(8.0))
                        .children(children![(
                            // NOTE: Menu header, fixed width and height, with a row flex direction
                            Node {
                                width: percent(100),
                                height: percent(100),
                                flex_direction: FlexDirection::Row,
                                ..default()
                            },
                            children![
                                ButtonBuilder::default()
                                    .text(locale.get("exit"))
                                    .bg_color(BTN_BG_COLOR)
                                    .font_size(BTN_TEXT_SIZE)
                                    .padding(BTN_PADDING)
                                    .border_radius(BTN_RADIUS)
                                    .justify_content(JustifyContent::Center)
                                    .align_self(AlignSelf::Start)
                                    .align_items(AlignItems::Center)
                                    .build_marked(UiButton::Exit),
                                Node { flex_grow: 1.0, ..default() },
                                ButtonBuilder::default()
                                    .text(">")
                                    .width(Val::Percent(20.0))
                                    .bg_color(BTN_BG_COLOR)
                                    .font_size(BTN_TEXT_SIZE)
                                    .padding(BTN_PADDING)
                                    .border_radius(BTN_RADIUS)
                                    .justify_content(JustifyContent::Center)
                                    .align_self(AlignSelf::Start)
                                    .align_items(AlignItems::Center)
                                    .build_marked(UiButton::HideMenu),
                            ]
                        )])
                        .build(),
                )
                .with_button_feedback();
        });
}

fn on_exit(event: On<Pointer<Click>>, buttons: Query<&UiButton>, mut exit: MessageWriter<AppExit>) {
    if let Ok(button) = buttons.get(event.entity) {
        if matches!(button, UiButton::Exit) {
            info!("Exiting application normally");
            exit.write(AppExit::Success);
        }
    }
}
