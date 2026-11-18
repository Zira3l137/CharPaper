use bevy::picking::hover::Hovered;
use bevy::prelude::percent;
use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::Focused;
use charpaper_scene::ScreenSync;
use charpaper_scene::ScreenUi;
use charpaper_scene::StageKey;
use charpaper_settings::PanelSide;
use charpaper_settings::Tab;
use charpaper_settings::UiState;
use charpaper_settings::UpdateStatus;
use i18n_embed_fl::fl;

use crate::PanelHovered;
use crate::locale::Locale;
use crate::locale::Tr;
use crate::row::Row;
use crate::tabs;
use crate::theme::*;
use crate::widgets::*;

// The frame around the tabs: the button that brings the panel back, the header, the tab
// bar and the footer. Every screen has a panel of its own. They all show and change the same
// settings, and share which tab is open and what is folded.
pub(crate) struct PanelPlugin;

impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelHovered>()
            .add_systems(PreUpdate, spawn_panels.after(ScreenSync))
            .add_observer(despawn_panel)
            .add_systems(Update, (track_hover, show_status, show_help, hide_reveal_button))
            .add_systems(Update, (add_update_dots, show_update_dots).chain())
            .add_systems(
                Update,
                (show_panel, style_tabs, dock_panels)
                    .run_if(resource_changed::<UiState>.or_eager(new_panel)),
            )
            .add_observer(on_click);
    }
}

// The root of the panel on one screen.
#[derive(Component)]
pub(crate) struct Panel {
    screen: Entity,
}

// The panel itself, which hides away to the button that brings it back.
#[derive(Component)]
struct MainMenu;

// Holds the reveal button, with room around it, and stays behind, empty, when the button
// hides itself, so the pointer still has something to find: picking skips hidden nodes, not
// empty shown ones.
#[derive(Component, Default)]
struct RevealZone {
    // Seconds without the pointer on it.
    idle: f32,
}

// Seconds the reveal button stays after the pointer leaves it, when it hides itself.
const REVEAL_LINGERS: f32 = 3.0;

const REVEAL_BUTTON: Vec2 = Vec2::new(40.0, 80.0);
// How far past the reveal button its zone reaches, into the screen and up and down, so a
// quick, rough move to the edge finds the button.
const REVEAL_REACH: f32 = 32.0;

// A tab's page. Each scrolls on its own, so a tab is where it was left when you come back to it.
#[derive(Component)]
pub(crate) struct Page(pub Tab);

#[derive(Component, Clone, Copy, PartialEq)]
enum PanelButton {
    Hide,
    Reveal,
    Exit,
    Tab(Tab),
}

// On the reveal button and the System tab while a newer version waits for an answer, since
// the panel is the only place CharPaper can tell anyone.
#[derive(Component)]
struct UpdateDot;

// The line under the panel's title: the suite's name.
#[derive(Component)]
struct StatusText;

// The line at the bottom of the panel saying what the row under the pointer does.
#[derive(Component)]
struct HelpText;

// For the systems that fill in what a panel shows. Each only runs on a change of what it
// shows, which a panel made later, for a monitor switched on, would otherwise wait for.
pub(crate) fn new_panel(panels: Query<(), Added<Panel>>) -> bool {
    !panels.is_empty()
}

fn spawn_panels(
    mut commands: Commands,
    ui: Res<UiState>,
    screens: Query<(Entity, &ScreenUi)>,
    panels: Query<&Panel>,
) {
    for (screen, camera) in &screens {
        if !panels.iter().any(|root| root.screen == screen) {
            spawn_panel(&mut commands, &ui, screen, camera.0);
        }
    }
}

// Along with the screen, so the panel never outlives the camera it draws through.
fn despawn_panel(
    removed: On<Remove, ScreenUi>,
    panels: Query<(Entity, &Panel)>,
    mut commands: Commands,
) {
    for (panel, root) in &panels {
        if root.screen == removed.entity {
            commands.entity(panel).despawn();
        }
    }
}

fn spawn_panel(commands: &mut Commands, ui: &UiState, screen: Entity, camera: Entity) {
    // Spans the screen only to lay the panel out, so it lets the pointer through to the scene.
    let root = (
        Node { width: percent(100), height: percent(100), ..default() },
        Pickable::IGNORE,
        UiTargetCamera(camera),
        Panel { screen },
    );
    commands.spawn(root).with_children(|root| {
        root.spawn(reveal_zone(ui.is_menu_closed)).with_children(|zone| {
            zone.spawn(reveal_button()).with_hover_feedback();
        });
        root.spawn(main_menu(ui.is_menu_closed))
            .with_children(|panel| {
                panel.spawn(header());
                panel.spawn(tab_bar());
                panel.spawn(page(Tab::Scene, ui.tab, tabs::scene::page()));
                panel.spawn(page(Tab::Graphics, ui.tab, tabs::graphics::page()));
                panel.spawn(page(Tab::Sound, ui.tab, tabs::sound::page()));
                panel.spawn(page(Tab::System, ui.tab, tabs::system::page()));
                panel.spawn(footer());
            })
            .with_hover_feedback();
    });
}

// Which edge it sits on, and which way the button's arrow points, is up to `dock_panels`.
fn reveal_zone(closed: bool) -> impl Bundle {
    let height = REVEAL_BUTTON.y + 2.0 * REVEAL_REACH;
    (
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(50.0),
            margin: UiRect::top(Val::Px(-height / 2.0)),
            width: Val::Px(REVEAL_BUTTON.x + REVEAL_REACH),
            height: Val::Px(height),
            align_items: AlignItems::Center,
            display: display(closed),
            ..default()
        },
        // Felt, but not in the way: the scene under the empty part still gets the pointer.
        Pickable { should_block_lower: false, is_hoverable: true },
        RevealZone::default(),
        Hovered::default(),
    )
}

fn reveal_button() -> impl Bundle {
    ButtonBuilder::new("<")
        .font_size(GLYPH_SIZE)
        .text_color(TEXT)
        .fill(PANEL_BG)
        .border_color(BORDER)
        .node(|n| {
            n.width = Val::Px(REVEAL_BUTTON.x);
            n.height = Val::Px(REVEAL_BUTTON.y);
            n.justify_content = JustifyContent::Center;
            n.align_items = AlignItems::Center;
        })
        .build(PanelButton::Reveal)
}

fn main_menu(closed: bool) -> impl Bundle {
    (
        Node {
            display: display(!closed),
            position_type: PositionType::Absolute,
            top: PANEL_MARGIN,
            bottom: PANEL_MARGIN,
            width: PANEL_WIDTH,
            flex_direction: FlexDirection::Column,
            border: UiRect::all(LINE),
            border_radius: BorderRadius::all(PANEL_RADIUS),
            ..default()
        },
        Fill(PANEL_BG),
        Stroke(BORDER),
        BoxShadow(vec![ShadowStyle {
            color: Color::linear_rgba(0.0, 0.0, 0.0, 0.50),
            x_offset: Val::Px(-2.0),
            y_offset: Val::Px(0.0),
            blur_radius: Val::Px(8.0),
            spread_radius: Val::Px(0.0),
        }]),
        // Pickable, unlike most nodes inside it: the pointer anywhere on the panel stops here
        // instead of reaching the scene behind it.
        MainMenu,
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
        Stroke(BORDER),
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
            glyph_button(">", PanelButton::Hide),
        ],
    )
}

fn tab_bar() -> impl Bundle {
    let tab = |tab: Tab| {
        translated_button(tab_title(tab))
            .node(|n| {
                n.flex_grow = 1.0;
                n.flex_basis = Val::ZERO;
            })
            .build(PanelButton::Tab(tab))
    };
    (
        Node {
            flex_shrink: 0.0,
            column_gap: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
            border: UiRect::bottom(LINE),
            ..default()
        },
        Stroke(BORDER),
        Pickable::IGNORE,
        children![tab(Tab::Scene), tab(Tab::Graphics), tab(Tab::Sound), tab(Tab::System)],
    )
}

// The wheel does the scrolling.
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
        Page(tab),
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
        Stroke(BORDER),
        Pickable::IGNORE,
        children![
            (
                label("", SMALL_SIZE, TEXT_DIM),
                Node { flex_grow: 1.0, min_width: Val::ZERO, ..default() },
                HelpText,
            ),
            translated_button(|l| fl!(l, "quit"))
                .text_color(DANGER_TEXT)
                .border_color(DANGER_BORDER)
                .build(PanelButton::Exit),
        ],
    )
}

fn show_panel(
    ui: Res<UiState>,
    parts: Query<(&mut Node, Option<&Page>, Has<RevealZone>, Has<MainMenu>)>,
) {
    for (mut node, page, zone, menu) in parts {
        let shown = match page {
            Some(page) => page.0 == ui.tab,
            None if zone => ui.is_menu_closed,
            None if menu => !ui.is_menu_closed,
            None => continue,
        };
        node.display = display(shown);
    }
}

fn style_tabs(
    ui: Res<UiState>,
    tabs: Query<(&PanelButton, &Children, &mut Fill, &mut Stroke)>,
    mut texts: Query<&mut Ink>,
) {
    for (button, children, mut fill, mut border) in tabs {
        let PanelButton::Tab(tab) = button else {
            continue;
        };
        let active = *tab == ui.tab;
        let (background, outline, text) =
            if active { (ACCENT, ACCENT, ON_ACCENT) } else { (BUTTON_BG, BORDER, TEXT_DIM) };
        fill.set_if_neq(Fill(background));
        border.set_if_neq(Stroke(outline));
        for &child in &**children {
            if let Ok(mut ink) = texts.get_mut(child) {
                ink.set_if_neq(Ink(text));
            }
        }
    }
}

// Keeps the panel to the edge of the screen the theme picks, with the button that brings it
// back on that edge, and points both arrows the way the panel moves.
fn dock_panels(
    ui: Res<UiState>,
    parts: Query<(
        &mut Node,
        Option<&PanelButton>,
        Has<MainMenu>,
        Has<RevealZone>,
        Option<&Children>,
    )>,
    mut texts: Query<&mut Text>,
) {
    let left = ui.theme.side == PanelSide::Left;
    let (hide, reveal) = if left { ("<", ">") } else { (">", "<") };
    for (mut node, button, menu, zone, children) in parts {
        if zone {
            let edge = if left {
                (Val::ZERO, Val::Auto, JustifyContent::FlexStart)
            } else {
                (Val::Auto, Val::ZERO, JustifyContent::FlexEnd)
            };
            if (node.left, node.right, node.justify_content) != edge {
                (node.left, node.right, node.justify_content) = edge;
            }
            continue;
        }
        let glyph = match button {
            Some(PanelButton::Hide) => hide,
            Some(PanelButton::Reveal) => {
                // Flush with the edge, so the side touching it has no border or rounding.
                let mut border = UiRect::all(LINE);
                let radius = if left {
                    border.left = Val::ZERO;
                    BorderRadius::right(PANEL_RADIUS)
                } else {
                    border.right = Val::ZERO;
                    BorderRadius::left(PANEL_RADIUS)
                };
                if (node.border, node.border_radius) != (border, radius) {
                    node.border = border;
                    node.border_radius = radius;
                }
                reveal
            }
            _ if menu => {
                let gap = if left { (PANEL_MARGIN, Val::Auto) } else { (Val::Auto, PANEL_MARGIN) };
                if (node.left, node.right) != gap {
                    node.left = gap.0;
                    node.right = gap.1;
                }
                continue;
            }
            _ => continue,
        };
        let Some(children) = children else {
            continue;
        };
        for &child in &**children {
            if let Ok(mut text) = texts.get_mut(child)
                && text.0 != glyph
            {
                text.0 = glyph.to_string();
            }
        }
    }
}

// With the setting on, the reveal button hides once the pointer has been off it for a while,
// and comes back on the frame the pointer reaches its place. Hiding the panel counts as
// touching it, so you see where the button went before it goes.
fn hide_reveal_button(
    time: Res<Time>,
    ui: Res<UiState>,
    mut was_closed: Local<bool>,
    update: Res<UpdateStatus>,
    mut zones: Query<(&mut RevealZone, &Hovered, &Children)>,
    mut buttons: Query<&mut Visibility, With<PanelButton>>,
) {
    let just_closed = ui.is_menu_closed && !*was_closed;
    *was_closed = ui.is_menu_closed;
    // It stays while its dot is on, or nobody would see the dot.
    let keep = !ui.hide_reveal_button || just_closed || waiting(&update);
    for (mut zone, hovered, children) in &mut zones {
        if keep || hovered.get() {
            zone.idle = 0.0;
        } else {
            zone.idle += time.delta_secs();
        }
        let shown =
            if zone.idle < REVEAL_LINGERS { Visibility::Inherited } else { Visibility::Hidden };
        for &child in &**children {
            if let Ok(mut visibility) = buttons.get_mut(child) {
                visibility.set_if_neq(shown);
            }
        }
    }
}

// Hovered is kept up to date by Bevy, true while the pointer is on the entity or anything
// inside it.
fn track_hover(parts: Query<&Hovered>, mut panel: ResMut<PanelHovered>) {
    panel.set_if_neq(PanelHovered(parts.iter().any(Hovered::get)));
}

// The suite's name, and with several stages running, which stage the panel shows. Every
// frame, but a Text is only touched when it changes, so the panel isn't laid out again.
fn show_status(
    suite: Focused<ActiveSuite>,
    stage: Focused<StageKey>,
    stages: Query<(), With<StageKey>>,
    locale: Res<Locale>,
    mut status: Query<&mut Text, With<StatusText>>,
) {
    let name = suite.get().map(|suite| suite.name.clone()).unwrap_or_default();
    let wanted = match stage.get() {
        Some(key) if stages.iter().count() > 1 => {
            format!("{} · {name}", fl!(locale, "stage-name", letter = key.0.to_string()))
        }
        _ => name,
    };
    for mut text in &mut status {
        if text.0 != wanted {
            text.0.clone_from(&wanted);
        }
    }
}

// A sentence about the row under the pointer. With none under it, how the panel works, which is
// the only place the wheel is mentioned.
fn show_help(
    locale: Res<Locale>,
    rows: Query<(Entity, &Row, Ref<Hovered>)>,
    parents: Query<&ChildOf>,
    new_lines: Query<(), Added<HelpText>>,
    mut lines: Query<&mut Text, With<HelpText>>,
) {
    let hover_moved = rows.iter().any(|(_, _, hovered)| hovered.is_changed());
    if !locale.is_changed() && !hover_moved && new_lines.is_empty() {
        return;
    }
    // A row in a drawer is hovered along with the row holding it: the innermost one is meant.
    let hovered = rows
        .iter()
        .filter(|(_, _, hovered)| hovered.get())
        .max_by_key(|&(row, _, _)| parents.iter_ancestors(row).count())
        .map(|(_, row, _)| row.help);
    let text = match hovered {
        Some(help) => help(&locale),
        None => fl!(locale, "help-panel"),
    };
    for mut line in &mut lines {
        line.0.clone_from(&text);
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&PanelButton>,
    mut ui: ResMut<UiState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    match button {
        PanelButton::Exit => {
            info!("quitting");
            exit.write(AppExit::Success);
        }
        PanelButton::Hide => ui.is_menu_closed = true,
        PanelButton::Reveal => ui.is_menu_closed = false,
        PanelButton::Tab(tab) => ui.tab = *tab,
    }
}

fn waiting(update: &UpdateStatus) -> bool {
    matches!(update, UpdateStatus::Available { .. } | UpdateStatus::Failed { .. })
}

fn add_update_dots(
    mut commands: Commands,
    buttons: Query<(Entity, &PanelButton), Added<PanelButton>>,
) {
    for (button, kind) in &buttons {
        if matches!(kind, PanelButton::Reveal | PanelButton::Tab(Tab::System)) {
            commands.entity(button).with_child((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(5.0),
                    right: Val::Px(5.0),
                    width: Val::Px(7.0),
                    height: Val::Px(7.0),
                    border_radius: BorderRadius::all(Val::Px(3.5)),
                    display: Display::None,
                    ..default()
                },
                Fill(ACCENT),
                Pickable::IGNORE,
                UpdateDot,
            ));
        }
    }
}

fn show_update_dots(update: Res<UpdateStatus>, mut dots: Query<&mut Node, With<UpdateDot>>) {
    let shown = display(waiting(&update));
    for mut dot in &mut dots {
        if dot.display != shown {
            dot.display = shown;
        }
    }
}

fn tab_title(tab: Tab) -> Tr {
    match tab {
        Tab::Scene => |l| fl!(l, "tab-scene"),
        Tab::Graphics => |l| fl!(l, "tab-graphics"),
        Tab::Sound => |l| fl!(l, "tab-sound"),
        Tab::System => |l| fl!(l, "tab-system"),
    }
}
