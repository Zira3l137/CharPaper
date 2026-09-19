mod config;
mod helpers;

use bevy::prelude::*;
pub use config::UiConfig;

use crate::helpers::ButtonBuilder;
use crate::helpers::ButtonReactiveExt;
use crate::helpers::ContainerBuilder;
use crate::helpers::UiButton;
use crate::helpers::UiContainer;
use crate::helpers::UiElement;
use crate::helpers::UiNode;
use crate::helpers::WithBackground;
use crate::helpers::WithText;
use crate::helpers::defaults::*;

#[derive(Resource, Default, Debug, Clone)]
pub struct UiState {
    pub is_menu_closed: bool,
}

pub struct CustomUiPlugin {
    pub config: UiConfig,
    pub state: UiState,
}

impl Plugin for CustomUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone());
        app.insert_resource(self.state.clone());
        app.add_systems(Startup, spawn_ui);
        app.add_systems(Update, update_ui);
        app.add_observer(on_button_click);
    }
}

pub fn update_ui(state: Res<UiState>, ui_elements: Query<(&mut Node, &UiElement)>) {
    if !state.is_changed() {
        return;
    }
    for (mut node, element) in ui_elements {
        match element {
            UiElement::Container(container) => {
                if let UiContainer::MainMenu = container {
                    if state.is_menu_closed {
                        node.display = Display::None;
                    } else {
                        node.display = Display::Flex;
                    }
                }
            }
            UiElement::Button(btn) => {
                if let UiButton::RevealMenu = btn {
                    if state.is_menu_closed {
                        node.display = Display::Flex;
                    } else {
                        node.display = Display::None;
                    }
                }
            }
        }
    }
}

pub fn spawn_ui(mut commands: Commands, config: Res<UiConfig>, state: Res<UiState>) {
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
                    ButtonBuilder::default()
                        .text("<")
                        .width(Val::Px(40.0))
                        .bg_color(BTN_BG_COLOR)
                        .font_size(BTN_TEXT_SIZE)
                        .padding(BTN_PADDING)
                        .border_radius(BTN_RADIUS)
                        .align_self(AlignSelf::Center)
                        .edit_node(|n| {
                            n.display =
                                if state.is_menu_closed { Display::Flex } else { Display::None }
                        })
                        .build_with(UiElement::Button(UiButton::RevealMenu)),
                )
                .with_button_feedback();
            parent
                .spawn(
                    // NOTE: Menu container, fixed width and full height, with a column flex direction
                    ContainerBuilder::default()
                        .width(Val::Px(256.0))
                        .height(Val::Percent(100.0))
                        .bg_color(CONTAINER_BG_COLOR)
                        .padding(CONTAINER_PADDING)
                        .bg_shadow_color(Color::linear_rgba(0.0, 0.0, 0.0, 0.50))
                        .bg_shadow_offset(Val::Px(-2.0), Val::Px(0.0))
                        .bg_shadow_blur_radius(Val::Px(8.0))
                        .edit_node(|n| {
                            n.flex_direction = FlexDirection::Column;
                            n.display =
                                if state.is_menu_closed { Display::None } else { Display::Flex };
                        })
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
                                    .build_with(UiElement::Button(UiButton::Exit)),
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
                                    .build_with(UiElement::Button(UiButton::HideMenu)),
                            ]
                        )])
                        .build_with(UiElement::Container(UiContainer::MainMenu)),
                )
                .with_button_feedback();
        });
}

fn on_button_click(
    event: On<Pointer<Click>>,
    ui_elements: Query<&UiElement>,
    mut state: ResMut<UiState>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Ok(element) = ui_elements.get(event.entity)
        && let UiElement::Button(button) = element
    {
        match button {
            UiButton::Exit => {
                info!("Exiting application normally");
                exit.write(AppExit::Success);
            }
            UiButton::HideMenu => {
                info!("Hiding menu");
                state.is_menu_closed = true;
            }
            UiButton::RevealMenu => {
                info!("Revealing menu");
                state.is_menu_closed = false;
            } // _ => {}
        }
    }
}
