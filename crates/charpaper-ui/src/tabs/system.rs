use bevy::prelude::*;
use charpaper_scene::StageKey;
use charpaper_scene::focused_changed;
use charpaper_settings::ChosenMonitor;
use charpaper_settings::DisplayMode;
use charpaper_settings::FpsLimit;
use charpaper_settings::Monitors;
use charpaper_settings::RenderSettings;
use charpaper_settings::ScreenSettings;
use charpaper_settings::StageId;
use charpaper_settings::StartAtLogin;
use charpaper_settings::SystemSettings;
use charpaper_settings::UiState;
use i18n_embed_fl::fl;

use crate::locale::Languages;
use crate::locale::Locale;
use crate::panel::new_panel;
use crate::row::*;
use crate::section::Content;
use crate::section::Section;
use crate::section::section;
use crate::theme::*;
use crate::widgets::*;

// The list of monitors, which are not rows but a button each.
pub(crate) struct SystemPlugin;

impl Plugin for SystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            (fill_monitors, show_monitors).chain().run_if(
                resource_changed::<ScreenSettings>
                    .or_eager(resource_changed::<Monitors>)
                    .or_eager(resource_changed::<Locale>)
                    .or_eager(focused_changed::<StageKey>)
                    .or_eager(new_panel),
            ),
        );
    }
}

#[derive(Component)]
#[require(Content)]
struct MonitorList;

// Shows the wallpaper on the monitor with this id, or stops showing it there.
#[derive(Component)]
struct MonitorButton(String);

// Settings of the app itself rather than of what it shows: how and when it runs, which
// monitors it is on and which stage each shows, how often it draws and when it stops, and the
// panel's language.
pub(crate) fn page() -> impl Bundle {
    children![app_section(), display_section(), frame_rate(), pause(), interface()]
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
            .segments(),
        ],
    )
}

fn display_section() -> impl Bundle {
    section(
        |l| fl!(l, "section-display"),
        Section::Display,
        children![
            (
                Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
                Pickable::IGNORE,
                MonitorList,
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

fn frame_rate() -> impl Bundle {
    section(
        |l| fl!(l, "section-frame-rate"),
        Section::FrameRate,
        children![
            Setting {
                label: |l| fl!(l, "label-fps-limit"),
                help: |l| fl!(l, "help-fps-limit"),
                show: |w, l| {
                    let current = w.resource::<RenderSettings>().fps_limit;
                    Some(choice(&FpsLimit::ALL, &current, |limit| match limit.per_second() {
                        Some(fps) => format!("{fps:.0}"),
                        None => fl!(l, "value-max"),
                    }))
                },
                change: |w, step| {
                    let mut settings = w.resource_mut::<RenderSettings>();
                    pick(&FpsLimit::ALL, &mut settings.fps_limit, step);
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-monitor"),
                help: |l| fl!(l, "help-monitor"),
                show: |w, l| Some(switch(l, w.resource::<UiState>().performance_monitor)),
                change: |w, step| flip(&mut w.resource_mut::<UiState>().performance_monitor, step),
            }
            .segments(),
        ],
    )
}

fn pause() -> impl Bundle {
    section(
        |l| fl!(l, "section-pause"),
        Section::Pause,
        children![
            Setting {
                label: |l| fl!(l, "label-pause-fullscreen"),
                help: |l| fl!(l, "help-pause-fullscreen"),
                show: |w, l| Some(switch(l, w.resource::<RenderSettings>().pause_when_fullscreen)),
                change: |w, step| {
                    flip(&mut w.resource_mut::<RenderSettings>().pause_when_fullscreen, step);
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-pause-covered"),
                help: |l| fl!(l, "help-pause-covered"),
                show: |w, l| Some(switch(l, w.resource::<RenderSettings>().pause_when_covered)),
                change: |w, step| {
                    flip(&mut w.resource_mut::<RenderSettings>().pause_when_covered, step);
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-pause-battery"),
                help: |l| fl!(l, "help-pause-battery"),
                show: |w, l| Some(switch(l, w.resource::<RenderSettings>().pause_on_battery)),
                change: |w, step| {
                    flip(&mut w.resource_mut::<RenderSettings>().pause_on_battery, step);
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
            .picker()
        ],
    )
}

// None, following the system's languages, comes first.
fn languages_of(languages: &Languages) -> Vec<Option<String>> {
    std::iter::once(None).chain(languages.0.iter().map(|l| Some(l.tag.clone()))).collect()
}

// One row per connected monitor, left to right as they stand, and one for each chosen monitor
// that is unplugged, so it can be switched off while it is away. Made again only when the rows
// themselves change, or for a list still empty; giving one another stage only relabels its
// button.
fn fill_monitors(
    mut commands: Commands,
    screen: Res<ScreenSettings>,
    monitors: Res<Monitors>,
    locale: Res<Locale>,
    lists: Query<(Entity, Option<&Children>), With<MonitorList>>,
    mut made: Local<Vec<(String, String)>>,
) {
    let missing = fl!(locale, "monitor-missing");
    let connected =
        monitors.0.iter().map(|m| (m.id.clone(), format!("{} · {}×{}", m.name, m.width, m.height)));
    let unplugged = screen
        .monitors
        .iter()
        .filter(|chosen| monitors.get(&chosen.id).is_none())
        .map(|chosen| (chosen.id.clone(), format!("{} · {missing}", chosen.name)));
    let rows: Vec<(String, String)> = connected.chain(unplugged).collect();
    let changed = *made != rows;
    for (list, children) in &lists {
        let filled = children.is_some_and(|c| !c.is_empty());
        if filled && !changed {
            continue;
        }
        commands.entity(list).despawn_children().with_children(|list| {
            for (id, label_text) in &rows {
                list.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        height: Val::Px(32.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                    children![
                        (
                            Node { flex_grow: 1.0, min_width: Val::ZERO, ..default() },
                            Pickable::IGNORE,
                            children![label(label_text, BODY_SIZE, TEXT_LABEL)],
                        ),
                        button("-")
                            .node(|n| n.width = Val::Px(56.0))
                            .build(MonitorButton(id.clone())),
                    ],
                ));
            }
        });
    }
    *made = rows;
}

fn show_monitors(
    screen: Res<ScreenSettings>,
    monitors: Res<Monitors>,
    locale: Res<Locale>,
    buttons: Query<(&MonitorButton, &Children)>,
    mut texts: Query<&mut Text>,
) {
    for (button, children) in &buttons {
        let text = match shown_stage(&screen, &monitors, &button.0) {
            Some(stage) => stage.to_string(),
            None => on_off(&locale, false),
        };
        for &child in &**children {
            if let Ok(mut label) = texts.get_mut(child) {
                label.0.clone_from(&text);
            }
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

// A click moves the monitor on to the next stage, then Off, then round to A. There are as many
// stages to pick from as monitors in the list, enough for each to show its own.
fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&MonitorButton>,
    monitors: Res<Monitors>,
    mut screen: ResMut<ScreenSettings>,
) {
    let Ok(MonitorButton(id)) = buttons.get(event.entity) else {
        return;
    };
    // Nothing chosen yet means the monitors showing it now, so the first click builds on them.
    let mut chosen: Vec<ChosenMonitor> = match screen.monitors.is_empty() {
        true => monitors
            .0
            .iter()
            .filter_map(|m| {
                let stage = m.stage?;
                Some(ChosenMonitor { id: m.id.clone(), name: m.name.clone(), stage })
            })
            .collect(),
        false => screen.monitors.clone(),
    };
    let unplugged = chosen.iter().filter(|c| monitors.get(&c.id).is_none()).count();
    let count = (monitors.0.len() + unplugged).min(StageId::MAX);
    let options: Vec<Option<StageId>> =
        std::iter::once(None).chain((0..count).map(StageId::new)).collect();
    let current = shown_stage(&screen, &monitors, id);
    let at = options.iter().position(|option| *option == current).unwrap_or(0);
    let mut next = options[(at + 1) % options.len()];
    // The last connected monitor showing it can't go Off: a wallpaper on no monitor is none.
    let others_on = chosen.iter().any(|c| &c.id != id && monitors.get(&c.id).is_some());
    if next.is_none() && monitors.get(id).is_some() && !others_on {
        next = options[1];
    }

    chosen.retain(|c| &c.id != id);
    if let Some(stage) = next {
        let name = monitors.get(id).map(|m| m.name.clone());
        let name =
            name.or_else(|| screen.monitors.iter().find(|c| &c.id == id).map(|c| c.name.clone()));
        chosen.push(ChosenMonitor { id: id.clone(), name: name.unwrap_or_default(), stage });
    }
    screen.monitors = chosen;
}
