mod config;
mod helpers;
mod pages;
mod render_tab;
mod scene_tab;
mod theme;
mod widgets;

use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;
use charpaper_scene::LookBackup;
use charpaper_scene::RenderSettings;
pub use config::UiConfig;
use serde::Deserialize;
use serde::Serialize;

use crate::config::UiLocale;
use crate::helpers::BaseBackground;
use crate::helpers::ButtonBuilder;
use crate::helpers::ButtonReactiveExt;
use crate::helpers::ContainerBuilder;
use crate::helpers::UiButton;
use crate::helpers::UiContainer;
use crate::helpers::UiElement;
use crate::helpers::UiNode;
use crate::helpers::WithBackground;
use crate::helpers::WithText;
use crate::theme::*;
use crate::widgets::*;

/// The panel's pages. Only pages with something working behind them exist;
/// the others from the design are added as their features land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tab {
    #[default]
    Character,
    Scene,
    Render,
}

impl Tab {
    fn title(self, locale: &UiLocale) -> &str {
        match self {
            Tab::Character => locale.get_or("tab.character", "Character"),
            Tab::Scene => locale.get_or("tab.scene", "Scene"),
            Tab::Render => locale.get_or("tab.render", "Render"),
        }
    }
}

/// Saved between runs by the app, so the panel reopens as it was left.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    pub is_menu_closed: bool,
    pub tab: Tab,
}

/// The line under the title in the panel's header: the suite's name.
#[derive(Component)]
pub(crate) struct StatusText;

pub struct CustomUiPlugin {
    pub config: UiConfig,
    pub state: UiState,
}

impl Plugin for CustomUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone());
        app.insert_resource(self.state.clone());
        app.add_systems(Startup, spawn_ui);
        app.add_systems(Update, (update_ui, style_tabs).run_if(resource_changed::<UiState>));
        app.add_systems(
            Update,
            (
                pages::fill_outfits.run_if(resource_added::<ActiveSuite>),
                pages::style_outfits.run_if(resource_changed::<CharacterState>),
                // Eager, so `resource_added` runs every frame and its idea of
                // "since last time" stays current; short-circuited behind a
                // change on the same frame, it would report the clips as new
                // once more on the next.
                pages::show_animation.run_if(
                    resource_changed::<CharacterState>.or_eager(resource_added::<CharacterClips>),
                ),
                scene_tab::show_scene_tab.run_if(resource_added::<ActiveSuite>),
                // The suite changes when an environment's maps finish baking,
                // which can make its Brightness row relevant.
                scene_tab::show_rows.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_exists_and_changed::<ActiveSuite>),
                ),
                scene_tab::show_scene_values.run_if(
                    resource_changed::<CharacterState>
                        .or_eager(resource_exists_and_changed::<ActiveLook>),
                ),
                scene_tab::show_restore.run_if(resource_changed::<LookBackup>),
                render_tab::show_render_values.run_if(resource_changed::<RenderSettings>),
            )
                .chain(),
        );
        app.add_observer(on_button_click);
        app.add_observer(pages::on_scene_click);
        app.add_observer(scene_tab::on_scene_tab_click);
        app.add_observer(render_tab::on_render_click);
    }
}

pub fn update_ui(state: Res<UiState>, ui_elements: Query<(&mut Node, &UiElement)>) {
    for (mut node, element) in ui_elements {
        let shown = match element {
            UiElement::Container(UiContainer::MainMenu) => !state.is_menu_closed,
            UiElement::Button(UiButton::RevealMenu) => state.is_menu_closed,
            UiElement::Container(UiContainer::Page(tab)) => *tab == state.tab,
            _ => continue,
        };
        node.display = if shown { Display::Flex } else { Display::None };
    }
}

fn style_tabs(
    state: Res<UiState>,
    tabs: Query<(
        &UiElement,
        &Children,
        &mut BaseBackground,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut texts: Query<&mut TextColor>,
) {
    for (element, children, mut base, mut background, mut border) in tabs {
        let UiElement::Button(UiButton::Tab(tab)) = element else {
            continue;
        };
        let active = *tab == state.tab;
        let (fill, outline, text) =
            if active { (ACCENT, ACCENT, ON_ACCENT) } else { (BUTTON_BG, BORDER, TEXT_DIM) };
        paint(&mut base, &mut background, fill);
        *border = BorderColor::all(outline);
        for &child in &**children {
            if let Ok(mut color) = texts.get_mut(child) {
                color.0 = text;
            }
        }
    }
}

pub fn spawn_ui(mut commands: Commands, config: Res<UiConfig>, state: Res<UiState>) {
    let locale = &config.locale;
    let shown = |visible: bool| if visible { Display::Flex } else { Display::None };

    commands.spawn(Node { width: percent(100), height: percent(100), ..default() }).with_children(
        |root| {
            root.spawn(
                ButtonBuilder::default()
                    .text("<")
                    .font_size(GLYPH_SIZE)
                    .text_color(TEXT)
                    .bg_color(PANEL_BG)
                    .border_color(BORDER)
                    .justify_content(JustifyContent::Center)
                    .align_items(AlignItems::Center)
                    .edit_node(|n| {
                        n.position_type = PositionType::Absolute;
                        n.right = Val::ZERO;
                        n.top = Val::Percent(50.0);
                        n.margin.top = Val::Px(-40.0);
                        n.width = Val::Px(40.0);
                        n.height = Val::Px(80.0);
                        n.border = UiRect { right: Val::ZERO, ..UiRect::all(LINE) };
                        n.border_radius = BorderRadius::left(PANEL_RADIUS);
                        n.display = shown(state.is_menu_closed);
                    })
                    .build_with(UiElement::Button(UiButton::RevealMenu)),
            )
            .with_button_feedback();

            root.spawn(
                ContainerBuilder::default()
                    .bg_color(PANEL_BG)
                    .border(LINE)
                    .border_color(BORDER)
                    .border_radius(PANEL_RADIUS)
                    .bg_shadow_color(Color::linear_rgba(0.0, 0.0, 0.0, 0.50))
                    .bg_shadow_offset(Val::Px(-2.0), Val::Px(0.0))
                    .bg_shadow_blur_radius(Val::Px(8.0))
                    .edit_node(|n| {
                        n.position_type = PositionType::Absolute;
                        n.top = PANEL_MARGIN;
                        n.right = PANEL_MARGIN;
                        n.bottom = PANEL_MARGIN;
                        n.width = PANEL_WIDTH;
                        n.flex_direction = FlexDirection::Column;
                        n.display = shown(!state.is_menu_closed);
                    })
                    .build_with(UiElement::Container(UiContainer::MainMenu)),
            )
            .with_children(|panel| {
                panel.spawn(header());
                panel.spawn(tab_bar(locale));
                panel
                    .spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(16.0)),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|content| {
                        content.spawn(page(
                            Tab::Character,
                            state.tab,
                            pages::character_page(locale),
                        ));
                        content.spawn(page(Tab::Scene, state.tab, scene_tab::scene_page(locale)));
                        content.spawn(page(
                            Tab::Render,
                            state.tab,
                            render_tab::render_page(locale),
                        ));
                    });
                panel.spawn(footer(locale));
            })
            .with_button_feedback();
        },
    );
}

fn header() -> impl Bundle {
    (
        Node {
            height: Val::Px(56.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::new(Val::Px(16.0), Val::Px(12.0), Val::ZERO, Val::ZERO),
            border: UiRect::bottom(LINE),
            ..default()
        },
        BorderColor::all(BORDER),
        Pickable::IGNORE,
        children![
            (
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    label("CHARPAPER", TITLE_SIZE, TEXT),
                    (label("", SMALL_SIZE, TEXT_DIM), StatusText),
                ],
            ),
            glyph_button(">", UiButton::HideMenu),
        ],
    )
}

/// A flex row rather than the design's grid, so a tab can be hidden without
/// leaving an empty column behind.
///
/// The Scene tab starts hidden until a suite gives it something to show.
fn tab_bar(locale: &UiLocale) -> impl Bundle {
    let tab = |tab: Tab| {
        button(tab.title(locale))
            .edit_node(|n| {
                n.flex_grow = 1.0;
                n.flex_basis = Val::ZERO;
                if tab == Tab::Scene {
                    n.display = Display::None;
                }
            })
            .build_with(UiElement::Button(UiButton::Tab(tab)))
    };
    (
        Node {
            flex_shrink: 0.0,
            column_gap: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
            border: UiRect::bottom(LINE),
            ..default()
        },
        BorderColor::all(BORDER),
        Pickable::IGNORE,
        UiElement::Container(UiContainer::TabBar),
        children![tab(Tab::Character), tab(Tab::Scene), tab(Tab::Render)],
    )
}

fn page(tab: Tab, active: Tab, content: impl Bundle) -> impl Bundle {
    (
        Node {
            display: if tab == active { Display::Flex } else { Display::None },
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.0),
            ..default()
        },
        Pickable::IGNORE,
        UiElement::Container(UiContainer::Page(tab)),
        content,
    )
}

fn footer(locale: &UiLocale) -> impl Bundle {
    (
        Node {
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
            border: UiRect::top(LINE),
            ..default()
        },
        BorderColor::all(BORDER),
        Pickable::IGNORE,
        children![
            (Node { flex_grow: 1.0, ..default() }, Pickable::IGNORE),
            button(locale.get_or("quit", "Quit"))
                .text_color(DANGER_TEXT)
                .border_color(DANGER_BORDER)
                .build_with(UiElement::Button(UiButton::Exit)),
        ],
    )
}

fn on_button_click(
    event: On<Pointer<Click>>,
    ui_elements: Query<&UiElement>,
    mut state: ResMut<UiState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(UiElement::Button(button)) = ui_elements.get(event.entity) else {
        return;
    };
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
        }
        UiButton::Tab(tab) => state.tab = *tab,
        _ => {}
    }
}
