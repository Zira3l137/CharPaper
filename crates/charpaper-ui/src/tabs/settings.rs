use std::collections::BTreeSet;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::Justify;
use bevy::text::LineBreak;
use charpaper_scene::CursorPosition;
use charpaper_scene::Screen;
use charpaper_scene::ScreenMonitor;
use charpaper_scene::StageKey;
use charpaper_scene::focused_changed;
use charpaper_settings::Changelog;
use charpaper_settings::ChosenMonitor;
use charpaper_settings::DisplayMode;
use charpaper_settings::Hsv;
use charpaper_settings::Monitors;
use charpaper_settings::Palette;
use charpaper_settings::PanelSide;
use charpaper_settings::ScreenSettings;
use charpaper_settings::StageId;
use charpaper_settings::StartAtLogin;
use charpaper_settings::SystemSettings;
use charpaper_settings::UiState;
use charpaper_settings::UpdateAction;
use charpaper_settings::UpdateStatus;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use crate::icons::Icon;
use crate::icons::icon;
use crate::locale::Languages;
use crate::locale::Locale;
use crate::panel::Hint;
use crate::panel::new_panel;
use crate::row::percent;
use crate::row::*;
use crate::section::Content;
use crate::section::Section;
use crate::section::section;
use crate::theme::*;
use crate::widgets::*;

// The list of monitors, which are not rows but a button each, and the list of what changed
// in a newer version.
pub(crate) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PickedMonitor>()
            .add_observer(on_click)
            .add_observer(open_release_page)
            .add_systems(
                Update,
                (fill_monitors, show_monitors).chain().run_if(
                    resource_changed::<ScreenSettings>
                        .or_eager(resource_changed::<Monitors>)
                        .or_eager(resource_changed::<PickedMonitor>)
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(focused_changed::<StageKey>)
                        .or_eager(new_panel),
                ),
            )
            .add_systems(
                Update,
                show_banner.run_if(resource_changed::<UpdateStatus>.or_eager(new_panel)),
            )
            .add_systems(
                Update,
                fill_changelog.run_if(
                    resource_changed::<Changelog>
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(new_panel),
                ),
            );
    }
}

// Which monitor the Display section's Shows row is for: the first one until another is
// clicked. Not saved.
#[derive(Resource, Default)]
pub(crate) struct PickedMonitor(Option<String>);

// The monitors, drawn to scale and side by side as they stand.
#[derive(Component)]
#[require(Content)]
struct MonitorStrip;

// A monitor, by its id. A click picks it.
#[derive(Component)]
struct MonitorTile(String);

// The stage a monitor shows, on its tile.
#[derive(Component)]
struct StagePill;

// The picked monitor's name and size.
#[derive(Component)]
struct PickedName;

#[derive(Component)]
struct UpdateBanner;

#[derive(Component)]
struct ChangelogList;

#[derive(Component)]
struct OpenReleasePage;

// Settings of the app itself rather than of what it shows: how and when it runs, which
// monitors it is on and which stage each shows, how often it draws and when it stops, and the
// panel's language and looks.
pub(crate) fn page() -> impl Bundle {
    children![update_banner(), app_section(), display_section(), interface()]
}

// Over everything else on the tab while a newer version is on offer, and gone otherwise.
fn update_banner() -> impl Bundle {
    (
        Node {
            display: Display::None,
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(12.0), Val::Px(4.0)),
            border_radius: BorderRadius::all(CARD_RADIUS),
            ..default()
        },
        Fill(SELECTED_BG),
        Pickable::IGNORE,
        UpdateBanner,
        children![
            (
                Node { align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() },
                Pickable::IGNORE,
                children![
                    icon(Icon::Download, FontSize::Px(16.0), ACCENT),
                    translated(|l| fl!(l, "update-banner"), TITLE_SIZE, TEXT),
                ],
            ),
            update_version(),
            update(),
        ],
    )
}

fn show_banner(status: Res<UpdateStatus>, banners: Query<&mut Node, With<UpdateBanner>>) {
    let shown = display(!matches!(*status, UpdateStatus::Idle));
    for mut node in banners {
        if node.display != shown {
            node.display = shown;
        }
    }
}

fn app_section() -> impl Bundle {
    section(
        |l| fl!(l, "section-app"),
        Section::App,
        children![
            Setting {
                label: |l| fl!(l, "label-mode"),
                help: |l| fl!(l, "help-mode"),
                show: |w, l| {
                    let mode = w.resource::<SystemSettings>().mode;
                    Some(choice(&DisplayMode::ALL, &mode, |mode| match mode {
                        DisplayMode::Wallpaper => fl!(l, "mode-wallpaper"),
                        DisplayMode::Windowed => fl!(l, "mode-window"),
                    }))
                },
                // The app restarts in the new mode, so a wheel passing over the row mustn't.
                change: |w, step| {
                    if !step.continuous {
                        pick(&DisplayMode::ALL, &mut w.resource_mut::<SystemSettings>().mode, step);
                    }
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-start-at-login"),
                help: |l| fl!(l, "help-start-at-login"),
                show: |w, l| {
                    let start = w.resource::<StartAtLogin>().0;
                    Some(switch(l, start == Some(true)).when(start.is_some()))
                },
                change: |w, step| {
                    if let Some(on) = &mut w.resource_mut::<StartAtLogin>().0 {
                        flip(on, step);
                    }
                },
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-check-updates"),
                help: |l| fl!(l, "help-check-updates"),
                show: |w, l| Some(switch(l, w.resource::<SystemSettings>().check_for_updates)),
                change: |w, step| {
                    flip(&mut w.resource_mut::<SystemSettings>().check_for_updates, step);
                },
            }
            .switch(),
        ],
    )
}

// The banner's two rows are buttons rather than settings: segments with none current, which act
// on a click and ignore the wheel passing over them.

// The version on offer, and how its download goes. Its value opens what changed since the
// running version, with a button to the release's page for the rest.
fn update_version() -> impl Bundle {
    Setting {
        label: |l| fl!(l, "label-update-version"),
        help: |l| fl!(l, "help-update-version"),
        show: |w, l| {
            let text = match w.resource::<UpdateStatus>() {
                UpdateStatus::Idle => return Some(Shown::Hidden),
                UpdateStatus::Available { version } => version.clone(),
                UpdateStatus::Downloading { version, done, total } => {
                    let percent = (done * 100).checked_div(*total).unwrap_or(0);
                    fl!(l, "update-downloading", version = version.as_str(), percent = percent)
                }
                UpdateStatus::Failed { version, .. } => {
                    fl!(l, "update-failed", version = version.as_str())
                }
                UpdateStatus::Installing { version } => {
                    fl!(l, "update-restarting", version = version.as_str())
                }
            };
            Some(Shown::Choice { labels: vec![text], current: Some(0) })
        },
        change: |_, _| {},
    }
    .drawer(
        None,
        children![
            (
                Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
                Pickable::IGNORE,
                ChangelogList,
            ),
            translated_button(|l| fl!(l, "update-open-page"))
                .node(|n| {
                    n.align_self = AlignSelf::FlexStart;
                    n.margin = UiRect::vertical(Val::Px(8.0));
                })
                .build(OpenReleasePage),
        ],
    )
}

fn update() -> impl Bundle {
    Setting {
        label: |l| fl!(l, "label-update"),
        help: |l| fl!(l, "help-update"),
        show: |w, l| {
            let install = match w.resource::<UpdateStatus>() {
                UpdateStatus::Available { .. } => fl!(l, "update-install"),
                UpdateStatus::Failed { .. } => fl!(l, "update-retry"),
                _ => return Some(Shown::Hidden),
            };
            Some(Shown::Choice { labels: vec![install, fl!(l, "update-skip")], current: None })
        },
        change: |w, step| {
            let action = match step.change {
                Change::To(0) => UpdateAction::Install,
                Change::To(1) => UpdateAction::Skip,
                _ => return,
            };
            w.write_message(action);
        },
    }
    .segments()
}

fn display_section() -> impl Bundle {
    section(
        |l| fl!(l, "section-display"),
        Section::Display,
        children![
            (
                Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexEnd,
                    column_gap: Val::Px(TILE_GAP),
                    padding: UiRect::vertical(Val::Px(8.0)),
                    ..default()
                },
                Pickable::IGNORE,
                MonitorStrip,
            ),
            (
                label("", SMALL_SIZE, TEXT_DIM),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                Node { margin: UiRect::bottom(Val::Px(8.0)), ..default() },
                PickedName,
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    border: UiRect::top(LINE),
                    ..default()
                },
                Stroke(BORDER),
                Pickable::IGNORE,
                children![monitor_shows()],
            ),
            // For the stage the panel shows, with two monitors or more showing it.
            Setting {
                label: |l| fl!(l, "label-layout"),
                help: |l| fl!(l, "help-layout"),
                show: |w, l| {
                    let stage = w.focused::<StageKey>().map(|key| key.0);
                    let showing = w.resource::<Monitors>().0.iter();
                    let several =
                        showing.filter(|m| m.stage.is_some() && m.stage == stage).count() > 1;
                    let spanned = &w.resource::<ScreenSettings>().spanned;
                    let span = stage.is_some_and(|stage| spanned.contains(&stage));
                    let label = |&span: &bool| match span {
                        false => fl!(l, "layout-mirror"),
                        true => fl!(l, "layout-span"),
                    };
                    Some(choice(&[false, true], &span, label).when(several))
                },
                change: |w, step| {
                    let Some(stage) = w.focused::<StageKey>().map(|key| key.0) else {
                        return;
                    };
                    let mut screen = w.resource_mut::<ScreenSettings>();
                    let mut span = screen.spanned.contains(&stage);
                    flip(&mut span, step);
                    if span {
                        screen.spanned.insert(stage);
                    } else {
                        screen.spanned.remove(&stage);
                    }
                },
            }
            .segments(),
        ],
    )
}

fn interface() -> impl Bundle {
    section(
        |l| fl!(l, "section-interface"),
        Section::Interface,
        children![
            Setting {
                label: |l| fl!(l, "label-language"),
                help: |l| fl!(l, "help-language"),
                show: |w, l| {
                    let languages = w.resource::<Languages>();
                    let label = |tag: &Option<String>| match tag {
                        None => fl!(l, "language-system"),
                        Some(tag) => languages.name(tag).unwrap_or(tag).to_string(),
                    };
                    let current = &w.resource::<UiState>().language;
                    Some(choice(&languages_of(languages), current, label))
                },
                change: |w, step| {
                    let options = languages_of(w.resource::<Languages>());
                    pick(&options, &mut w.resource_mut::<UiState>().language, step);
                },
            }
            .picker(),
            Setting {
                label: |l| fl!(l, "label-theme"),
                help: |l| fl!(l, "help-theme"),
                show: |w, l| {
                    let palette = w.resource::<UiState>().theme.palette;
                    Some(choice(&Palette::ALL, &palette, |palette| match palette {
                        Palette::Slate => fl!(l, "theme-slate"),
                        Palette::Midnight => fl!(l, "theme-midnight"),
                        Palette::Light => fl!(l, "theme-light"),
                    }))
                },
                change: |w, step| {
                    pick(&Palette::ALL, &mut w.resource_mut::<UiState>().theme.palette, step);
                },
            }
            .segments(),
            accent(),
            Setting {
                label: |l| fl!(l, "label-opacity"),
                help: |l| fl!(l, "help-opacity"),
                show: |w, _| {
                    let opacity = w.resource::<UiState>().theme.opacity;
                    Some(OPACITY.number(percent(opacity), opacity))
                },
                change: |w, step| {
                    let mut ui = w.resource_mut::<UiState>();
                    ui.theme.opacity = OPACITY.nudge(ui.theme.opacity, step);
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-size"),
                help: |l| fl!(l, "help-size"),
                show: |w, _| {
                    let size = w.resource::<UiState>().theme.size;
                    Some(SIZE.number(percent(size), size))
                },
                change: |w, step| {
                    let mut ui = w.resource_mut::<UiState>();
                    ui.theme.size = SIZE.nudge(ui.theme.size, step);
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-side"),
                help: |l| fl!(l, "help-side"),
                show: |w, l| {
                    let side = w.resource::<UiState>().theme.side;
                    Some(choice(&PanelSide::ALL, &side, |side| match side {
                        PanelSide::Left => fl!(l, "side-left"),
                        PanelSide::Right => fl!(l, "side-right"),
                    }))
                },
                change: |w, step| {
                    pick(&PanelSide::ALL, &mut w.resource_mut::<UiState>().theme.side, step);
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-auto-hide"),
                help: |l| fl!(l, "help-auto-hide"),
                show: |w, l| Some(switch(l, w.resource::<UiState>().hide_reveal_button)),
                change: |w, step| flip(&mut w.resource_mut::<UiState>().hide_reveal_button, step),
            }
            .switch(),
        ],
    )
}

const OPACITY: Scale = Scale { step: 0.05, min: 0.4, max: 1.0, per_unit: 100.0 };
const SIZE: Scale = Scale { step: 0.05, min: 0.75, max: 1.5, per_unit: 100.0 };
const HUE_STEP: f32 = 5.0;
const PART: Scale = Scale { step: 0.02, min: 0.0, max: 1.0, per_unit: 100.0 };

// The value is the accent as hex. Its hue, saturation and brightness are rows in a drawer under
// it, which change it; the wheel over the value itself does nothing.
fn accent() -> impl Bundle {
    Setting {
        label: |l| fl!(l, "label-accent"),
        help: |l| fl!(l, "help-accent"),
        show: |w, _| {
            let hex = w.resource::<UiState>().theme.accent.hex();
            Some(choice(std::slice::from_ref(&hex), &hex, String::clone))
        },
        change: |_, _| {},
    }
    .drawer(
        Some(ACCENT),
        children![
            Setting {
                label: |l| fl!(l, "label-accent-hue"),
                help: |l| fl!(l, "help-accent-hue"),
                show: |w, _| {
                    let hue = w.resource::<UiState>().theme.accent.hue;
                    Some(number(format!("{hue:.0}°"), hue, 0.0, 360.0))
                },
                // Round the color wheel and past 360 back to 0, either way.
                change: |w, step| {
                    let mut ui = w.resource_mut::<UiState>();
                    let accent = &mut ui.theme.accent;
                    let hue = (accent.hue + step.by() as f32 * HUE_STEP).round();
                    *accent = Hsv::new(hue, accent.saturation, accent.value);
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-accent-saturation"),
                help: |l| fl!(l, "help-accent-saturation"),
                show: |w, _| {
                    let saturation = w.resource::<UiState>().theme.accent.saturation;
                    Some(PART.number(percent(saturation), saturation))
                },
                change: |w, step| {
                    let mut ui = w.resource_mut::<UiState>();
                    let accent = &mut ui.theme.accent;
                    accent.saturation = PART.nudge(accent.saturation, step);
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-accent-brightness"),
                help: |l| fl!(l, "help-accent-brightness"),
                show: |w, _| {
                    let value = w.resource::<UiState>().theme.accent.value;
                    Some(PART.number(percent(value), value))
                },
                change: |w, step| {
                    let mut ui = w.resource_mut::<UiState>();
                    let accent = &mut ui.theme.accent;
                    accent.value = PART.nudge(accent.value, step);
                },
            }
            .stepper(),
        ],
    )
}

// A version's number, then its changes. Their sections are only named when there are several:
// most versions have little but Other.
fn fill_changelog(
    mut commands: Commands,
    changelog: Res<Changelog>,
    locale: Res<Locale>,
    lists: Query<Entity, With<ChangelogList>>,
) {
    for list in &lists {
        commands.entity(list).despawn_children().with_children(|list| {
            for notes in &changelog.0 {
                list.spawn((
                    label(&notes.version, BODY_SIZE, TEXT),
                    Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                ));
                let titled = notes.sections.len() > 1;
                for (title, items) in &notes.sections {
                    if titled {
                        list.spawn(label(section_title(&locale, title), SMALL_SIZE, TEXT_DIM));
                    }
                    for item in items {
                        list.spawn(label(format!("• {item}"), CONTROL_SIZE, TEXT_LABEL));
                    }
                }
            }
        });
    }
}

// The release notes' section names are cliff.toml's, in English.
fn section_title(l: &FluentLanguageLoader, title: &str) -> String {
    match title {
        "Features" => fl!(l, "changelog-features"),
        "Fixes" => fl!(l, "changelog-fixes"),
        "Performance" => fl!(l, "changelog-performance"),
        "Refactoring" => fl!(l, "changelog-refactoring"),
        "Documentation" => fl!(l, "changelog-documentation"),
        "Tests" => fl!(l, "changelog-tests"),
        "Build" => fl!(l, "changelog-build"),
        "Reverts" => fl!(l, "changelog-reverts"),
        "Other" => fl!(l, "changelog-other"),
        other => other.to_string(),
    }
}

fn open_release_page(
    event: On<Pointer<Click>>,
    buttons: Query<(), With<OpenReleasePage>>,
    mut actions: MessageWriter<UpdateAction>,
) {
    if buttons.contains(event.entity) {
        actions.write(UpdateAction::OpenPage);
    }
}

// None, following the system's languages, comes first.
fn languages_of(languages: &Languages) -> Vec<Option<String>> {
    std::iter::once(None).chain(languages.0.iter().map(|l| Some(l.tag.clone()))).collect()
}

// A connected monitor, left to right as they stand, then each chosen monitor that is
// unplugged, so it can be switched off while it is away.
#[derive(Clone, PartialEq)]
struct Tile {
    id: String,
    name: String,
    // None while unplugged.
    size: Option<UVec2>,
}

fn tiles(screen: &ScreenSettings, monitors: &Monitors) -> Vec<Tile> {
    let connected = monitors.0.iter().map(|m| Tile {
        id: m.id.clone(),
        name: m.name.clone(),
        size: Some(UVec2::new(m.width, m.height)),
    });
    let unplugged = screen.monitors.iter().filter(|chosen| monitors.get(&chosen.id).is_none());
    let unplugged = unplugged.map(|chosen| Tile {
        id: chosen.id.clone(),
        name: chosen.name.clone(),
        size: None,
    });
    connected.chain(unplugged).collect()
}

fn picked<'a>(picked: &PickedMonitor, tiles: &'a [Tile]) -> Option<&'a Tile> {
    let found = tiles.iter().find(|tile| picked.0.as_ref() == Some(&tile.id));
    found.or(tiles.first())
}

// The room the tiles share in the section's card, and how tall the tallest may be.
const STRIP: Vec2 = Vec2::new(300.0, 84.0);
const TILE_GAP: f32 = 10.0;
// Enough for the number over the stage, however many monitors.
const TILE_MIN: f32 = 48.0;
// An unplugged monitor's size isn't known.
const UNPLUGGED: UVec2 = UVec2::new(1920, 1080);

// One scale for all, so they keep their sizes to one another, as large as fits.
fn tile_sizes(tiles: &[Tile]) -> Vec<Vec2> {
    let sizes: Vec<Vec2> =
        tiles.iter().map(|tile| tile.size.unwrap_or(UNPLUGGED).as_vec2()).collect();
    let tallest = sizes.iter().map(|size| size.y).fold(1.0, f32::max);
    let wide: f32 = sizes.iter().map(|size| size.x).sum::<f32>().max(1.0);
    let room = STRIP.x - TILE_GAP * tiles.len().saturating_sub(1) as f32;
    let scale = (STRIP.y / tallest).min(room / wide);
    sizes.iter().map(|size| (*size * scale).max(Vec2::splat(TILE_MIN))).collect()
}

// Made again only when the monitors themselves change, or for a strip still empty. Picking one,
// or giving one another stage, only restyles the tiles.
fn fill_monitors(
    mut commands: Commands,
    screen: Res<ScreenSettings>,
    monitors: Res<Monitors>,
    strips: Query<(Entity, Option<&Children>), With<MonitorStrip>>,
    mut made: Local<Vec<Tile>>,
) {
    let tiles = tiles(&screen, &monitors);
    let changed = *made != tiles;
    let sizes = tile_sizes(&tiles);
    for (strip, children) in &strips {
        let filled = children.is_some_and(|c| !c.is_empty());
        if filled && !changed {
            continue;
        }
        commands.entity(strip).despawn_children().with_children(|strip| {
            for (number, (tile, size)) in tiles.iter().zip(&sizes).enumerate() {
                strip.spawn(monitor_tile(number + 1, &tile.id, *size));
            }
        });
    }
    *made = tiles;
}

fn monitor_tile(number: usize, id: &str, size: Vec2) -> impl Bundle {
    (
        Node {
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            border: UiRect::all(Val::Px(1.5)),
            border_radius: BorderRadius::all(RADIUS),
            ..default()
        },
        BaseBackground::default(),
        Fill(BUTTON_BG),
        Stroke(BORDER),
        MonitorTile(id.to_string()),
        Hint(|l| fl!(l, "help-monitor-tile")),
        Hovered::default(),
        children![
            label(number.to_string(), FontSize::Px(18.0), TEXT),
            (
                Node {
                    padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                Fill(TRACK),
                Pickable::IGNORE,
                children![(label("-", SMALL_SIZE, TEXT_LABEL), TextLayout::no_wrap(), StagePill)],
            ),
        ],
    )
}

// The picked tile in the accent, an unplugged one see-through.
fn show_monitors(
    screen: Res<ScreenSettings>,
    monitors: Res<Monitors>,
    picked_monitor: Res<PickedMonitor>,
    locale: Res<Locale>,
    mut tiles: Query<(Entity, &MonitorTile, &mut Fill, &mut Stroke)>,
    children: Query<&Children>,
    mut pills: Query<&mut Text, With<StagePill>>,
    mut names: Query<&mut Text, (With<PickedName>, Without<StagePill>)>,
) {
    let all = self::tiles(&screen, &monitors);
    let picked = picked(&picked_monitor, &all);
    for (entity, tile, mut fill, mut stroke) in &mut tiles {
        let connected = monitors.get(&tile.0).is_some();
        let (wanted_fill, wanted_stroke) = match picked.is_some_and(|p| p.id == tile.0) {
            true => (SELECTED_BG, ACCENT),
            false if connected => (BUTTON_BG, BORDER),
            false => (CLEAR, BORDER),
        };
        fill.set_if_neq(Fill(wanted_fill));
        stroke.set_if_neq(Stroke(wanted_stroke));
        let stage = stage_label(&locale, shown_stage(&screen, &monitors, &tile.0));
        for part in children.iter_descendants(entity) {
            if let Ok(mut pill) = pills.get_mut(part)
                && pill.0 != stage
            {
                pill.0.clone_from(&stage);
            }
        }
    }
    let name = picked.map(|tile| match tile.size {
        Some(size) => format!("{} · {}×{}", tile.name, size.x, size.y),
        None => format!("{} · {}", tile.name, fl!(locale, "monitor-missing")),
    });
    let name = name.unwrap_or_default();
    for mut text in &mut names {
        if text.0 != name {
            text.0.clone_from(&name);
        }
    }
}

// An unplugged monitor keeps its stage while it is chosen: switching it off unchooses it.
fn shown_stage(screen: &ScreenSettings, monitors: &Monitors, id: &str) -> Option<StageId> {
    match monitors.get(id) {
        Some(monitor) => monitor.stage,
        None => screen.monitors.iter().find(|chosen| chosen.id == id).map(|chosen| chosen.stage),
    }
}

fn stage_label(l: &FluentLanguageLoader, stage: Option<StageId>) -> String {
    match stage {
        Some(stage) => fl!(l, "stage-name", letter = stage.to_string()),
        None => on_off(l, false),
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    tiles: Query<&MonitorTile>,
    mut picked: ResMut<PickedMonitor>,
) {
    if let Ok(MonitorTile(id)) = tiles.get(event.entity) {
        picked.0 = Some(id.clone());
    }
}

// What the picked monitor shows: a stage, or Off.
fn monitor_shows() -> impl Bundle {
    Setting {
        label: |l| fl!(l, "label-monitor-shows"),
        help: |l| fl!(l, "help-monitor-shows"),
        show: |w, l| {
            let (screen, monitors) = (w.resource::<ScreenSettings>(), w.resource::<Monitors>());
            let tiles = tiles(screen, monitors);
            let id = &picked(w.resource::<PickedMonitor>(), &tiles)?.id;
            let current = shown_stage(screen, monitors, id);
            let options = offered(screen, monitors, id);
            Some(choice(&options, &current, |&stage| stage_label(l, stage)))
        },
        change: |w, step| {
            let picked_monitor = PickedMonitor(w.resource::<PickedMonitor>().0.clone());
            w.resource_scope(|w, monitors: Mut<Monitors>| {
                let mut screen = w.resource_mut::<ScreenSettings>();
                let tiles = tiles(&screen, &monitors);
                let Some(tile) = picked(&picked_monitor, &tiles) else {
                    return;
                };
                let current = shown_stage(&screen, &monitors, &tile.id);
                let options = offered(&screen, &monitors, &tile.id);
                if let Some(next) = stepped(&options, &current, step) {
                    show_on(&mut screen, &monitors, &tile.id, next);
                }
            });
        },
    }
    .picker()
}

// Off, then every stage. Not Off for the last connected monitor showing it, though: a wallpaper
// on no monitor is none.
fn offered(screen: &ScreenSettings, monitors: &Monitors, id: &str) -> Vec<Option<StageId>> {
    let others_on =
        chosen(screen, monitors).iter().any(|c| c.id != id && monitors.get(&c.id).is_some());
    let off = others_on || monitors.get(id).is_none();
    let stages = stages(screen, monitors).into_iter().map(Some);
    off.then_some(None).into_iter().chain(stages).collect()
}

// There are as many stages to pick from as monitors in the list, enough for each to show its
// own.
fn stages(screen: &ScreenSettings, monitors: &Monitors) -> Vec<StageId> {
    let unplugged = screen.monitors.iter().filter(|c| monitors.get(&c.id).is_none()).count();
    let count = (monitors.0.len() + unplugged).min(StageId::MAX);
    (0..count).filter_map(StageId::new).collect()
}

// Shows the stage on the monitor, or with None, stops showing the wallpaper there.
fn show_on(screen: &mut ScreenSettings, monitors: &Monitors, id: &str, stage: Option<StageId>) {
    let mut picked = chosen(screen, monitors);
    picked.retain(|c| c.id != id);
    if let Some(stage) = stage {
        let name = monitors.get(id).map(|m| m.name.clone());
        let name =
            name.or_else(|| screen.monitors.iter().find(|c| c.id == id).map(|c| c.name.clone()));
        picked.push(ChosenMonitor { id: id.to_string(), name: name.unwrap_or_default(), stage });
    }
    screen.monitors = picked;
}

// Nothing chosen yet means the monitors showing it now, so the first change builds on them.
fn chosen(screen: &ScreenSettings, monitors: &Monitors) -> Vec<ChosenMonitor> {
    if !screen.monitors.is_empty() {
        return screen.monitors.clone();
    }
    let showing = monitors.0.iter().filter_map(|m| {
        Some(ChosenMonitor { id: m.id.clone(), name: m.name.clone(), stage: m.stage? })
    });
    showing.collect()
}

// Which stage the monitor under the pointer shows: the one whose panel is in use. It offers the
// stages the Display section does, but not Off, which would take the panel away with it.
// Shown only while there are several stages.
pub(crate) fn this_monitor() -> impl Bundle {
    Setting {
        label: |l| fl!(l, "label-this-monitor"),
        help: |l| fl!(l, "help-this-monitor"),
        show: |w, l| {
            let here = monitor_here(w)?;
            let monitors = w.resource::<Monitors>();
            let current = monitors.get(&here)?.stage?;
            let shown: BTreeSet<StageId> = monitors.0.iter().filter_map(|m| m.stage).collect();
            let options = stages(w.resource::<ScreenSettings>(), monitors);
            let label = |&stage: &StageId| stage_label(l, Some(stage));
            Some(choice(&options, &current, label).when(shown.len() > 1))
        },
        change: |w, step| {
            let Some(here) = monitor_here(w) else {
                return;
            };
            w.resource_scope(|w, monitors: Mut<Monitors>| {
                let mut screen = w.resource_mut::<ScreenSettings>();
                let options = stages(&screen, &monitors);
                let Some(current) = monitors.get(&here).and_then(|m| m.stage) else {
                    return;
                };
                if let Some(next) = stepped(&options, &current, step) {
                    show_on(&mut screen, &monitors, &here, Some(next));
                }
            });
        },
    }
    .picker()
}

fn monitor_here(w: &World) -> Option<String> {
    let at = w.get_resource::<CursorPosition>()?.0?;
    let mut screens = w.try_query::<(&Screen, &ScreenMonitor)>()?;
    let mut screens = screens.iter(w);
    screens.find(|(screen, _)| screen.window == at.window).map(|(_, monitor)| monitor.0.clone())
}
