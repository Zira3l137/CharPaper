use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use i18n_embed_fl::fl;

use crate::PanelHovered;
use crate::Tab;
use crate::UiState;
use crate::elements::Chevron;
use crate::elements::StatusText;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::tabs;
use crate::theme::*;
use crate::widgets::*;

// The frame around the tabs: the button that brings the panel back, the header, the tab
// bar and the footer.
pub(crate) struct PanelPlugin;

impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelHovered>()
            .add_systems(Startup, spawn_panel)
            .add_systems(Update, track_hover)
            .add_systems(
                Update,
                (show_panel, style_tabs, show_folds).run_if(resource_changed::<UiState>),
            )
            .add_systems(Update, show_suite_name.run_if(resource_added::<ActiveSuite>))
            .add_observer(on_click);
    }
}

fn spawn_panel(mut commands: Commands, ui: Res<UiState>) {
    // Spans the screen only to lay the panel out, so it lets the pointer through to the scene.
    let screen =
        (Node { width: percent(100), height: percent(100), ..default() }, Pickable::IGNORE);
    commands.spawn(screen).with_children(|root| {
        root.spawn((reveal_button(ui.is_menu_closed), Hovered::default())).with_hover_feedback();
        root.spawn(main_menu(ui.is_menu_closed))
            .with_children(|panel| {
                panel.spawn(header());
                panel.spawn(tab_bar());
                panel.spawn(page(Tab::Character, ui.tab, tabs::character::page()));
                panel.spawn(page(Tab::Scene, ui.tab, tabs::scene::page()));
                panel.spawn(page(Tab::Render, ui.tab, tabs::render::page()));
                panel.spawn(page(Tab::System, ui.tab, tabs::system::page()));
                panel.spawn(footer());
            })
            .with_hover_feedback();
    });
}

fn reveal_button(closed: bool) -> impl Bundle {
    ButtonBuilder::new("<")
        .font_size(GLYPH_SIZE)
        .text_color(TEXT)
        .fill(PANEL_BG)
        .border_color(BORDER)
        .node(|n| {
            n.position_type = PositionType::Absolute;
            n.right = Val::ZERO;
            n.top = Val::Percent(50.0);
            n.margin.top = Val::Px(-40.0);
            n.width = Val::Px(40.0);
            n.height = Val::Px(80.0);
            n.justify_content = JustifyContent::Center;
            n.align_items = AlignItems::Center;
            n.border = UiRect { right: Val::ZERO, ..UiRect::all(LINE) };
            n.border_radius = BorderRadius::left(PANEL_RADIUS);
            n.display = display(closed);
        })
        .build(UiButton::RevealMenu)
}

fn main_menu(closed: bool) -> impl Bundle {
    (
        Node {
            display: display(!closed),
            position_type: PositionType::Absolute,
            top: PANEL_MARGIN,
            right: PANEL_MARGIN,
            bottom: PANEL_MARGIN,
            width: PANEL_WIDTH,
            flex_direction: FlexDirection::Column,
            border: UiRect::all(LINE),
            border_radius: BorderRadius::all(PANEL_RADIUS),
            ..default()
        },
        BackgroundColor(PANEL_BG),
        BorderColor::all(BORDER),
        BoxShadow(vec![ShadowStyle {
            color: Color::linear_rgba(0.0, 0.0, 0.0, 0.50),
            x_offset: Val::Px(-2.0),
            y_offset: Val::Px(0.0),
            blur_radius: Val::Px(8.0),
            spread_radius: Val::Px(0.0),
        }]),
        // Pickable, unlike most nodes inside it: the pointer anywhere on the panel stops here
        // instead of reaching the scene behind it.
        UiContainer::MainMenu,
        Hovered::default(),
    )
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

fn tab_bar() -> impl Bundle {
    let tab = |tab: Tab| {
        translated_button(tab.title())
            .node(|n| {
                n.flex_grow = 1.0;
                n.flex_basis = Val::ZERO;
                // Hidden until a suite gives it something to show.
                if tab == Tab::Scene {
                    n.display = Display::None;
                }
            })
            .build(UiButton::Tab(tab))
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
        children![tab(Tab::Character), tab(Tab::Scene), tab(Tab::Render), tab(Tab::System)],
    )
}

// Each page scrolls on its own, so a tab is where it was left when you come back to it. The
// wheel module does the scrolling.
fn page(tab: Tab, active: Tab, content: impl Bundle) -> impl Bundle {
    (
        Node {
            display: display(tab == active),
            flex_grow: 1.0,
            // Otherwise the page grows to fit its rows and pushes the footer out of the panel.
            min_height: Val::ZERO,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.0),
            padding: UiRect::all(Val::Px(16.0)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        UiContainer::Page(tab),
        content,
    )
}

fn footer() -> impl Bundle {
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
            translated_button(|l| fl!(l, "quit"))
                .text_color(DANGER_TEXT)
                .border_color(DANGER_BORDER)
                .build(UiButton::Exit),
        ],
    )
}

// Written only on a real change: UiState changes on every tab switch too, and an untouched
// Node or Text spares Bevy laying the panel out again.
fn show_folds(
    ui: Res<UiState>,
    mut bodies: Query<(&UiContainer, &mut Node)>,
    mut chevrons: Query<(&Chevron, &mut Text)>,
) {
    for (container, mut node) in &mut bodies {
        if let UiContainer::Body(section) = container {
            let shown = display(!ui.is_folded(*section));
            if node.display != shown {
                node.display = shown;
            }
        }
    }
    for (chevron, mut text) in &mut chevrons {
        let arrow = if ui.is_folded(chevron.0) { "►" } else { "▼" };
        if text.0 != arrow {
            text.0 = arrow.to_string();
        }
    }
}

fn show_panel(ui: Res<UiState>, nodes: Query<(&mut Node, AnyOf<(&UiContainer, &UiButton)>)>) {
    for (mut node, element) in nodes {
        let shown = match element {
            (Some(UiContainer::MainMenu), _) => !ui.is_menu_closed,
            (Some(UiContainer::Page(tab)), _) => *tab == ui.tab,
            (_, Some(UiButton::RevealMenu)) => ui.is_menu_closed,
            _ => continue,
        };
        node.display = display(shown);
    }
}

fn style_tabs(
    ui: Res<UiState>,
    tabs: Query<(
        &UiButton,
        &Children,
        &mut BaseBackground,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut texts: Query<&mut TextColor>,
) {
    for (button, children, mut base, mut background, mut border) in tabs {
        let UiButton::Tab(tab) = button else {
            continue;
        };
        let active = *tab == ui.tab;
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

// Hovered is kept up to date by Bevy, true while the pointer is on the entity or anything
// inside it.
fn track_hover(parts: Query<&Hovered>, mut panel: ResMut<PanelHovered>) {
    panel.set_if_neq(PanelHovered(parts.iter().any(Hovered::get)));
}

fn show_suite_name(suite: Res<ActiveSuite>, mut status: Query<&mut Text, With<StatusText>>) {
    for mut text in &mut status {
        text.0 = suite.name.clone();
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    mut ui: ResMut<UiState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    match button {
        UiButton::Exit => {
            info!("quitting");
            exit.write(AppExit::Success);
        }
        UiButton::HideMenu => ui.is_menu_closed = true,
        UiButton::RevealMenu => ui.is_menu_closed = false,
        UiButton::Tab(tab) => ui.tab = *tab,
        UiButton::Fold(section) => ui.toggle_fold(*section),
        _ => {}
    }
}
