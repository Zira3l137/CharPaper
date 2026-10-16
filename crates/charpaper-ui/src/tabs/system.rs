use bevy::prelude::*;
use charpaper_audio::Volumes;
use charpaper_scene::ChosenMonitor;
use charpaper_scene::Focused;
use charpaper_scene::Monitors;
use charpaper_scene::ScreenSettings;
use charpaper_scene::StageId;
use charpaper_scene::StageKey;
use charpaper_scene::focused_changed;
use i18n_embed_fl::fl;

use crate::UiState;
use crate::elements::Cycler;
use crate::elements::Section;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiButton;
use crate::elements::UiContainer;
use crate::locale::Languages;
use crate::locale::Locale;
use crate::panel::new_panel;
use crate::theme::*;
use crate::widgets::percent;
use crate::widgets::*;

const VOLUME_STEP: f32 = 0.05;

// Settings of the app itself rather than of what it shows: which monitors it is on and which
// stage each shows, how loud it is, and the panel's language.
pub(crate) struct SystemTabPlugin;

impl Plugin for SystemTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_step).add_observer(on_click).add_systems(
            Update,
            (
                show_language.run_if(
                    resource_changed::<UiState>
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(new_panel),
                ),
                show_volumes.run_if(resource_changed::<Volumes>.or_eager(new_panel)),
                (fill_monitors, show_monitors, show_layout).chain().run_if(
                    resource_changed::<ScreenSettings>
                        .or_eager(resource_changed::<Monitors>)
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(focused_changed::<StageKey>)
                        .or_eager(new_panel),
                ),
            ),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![
        shown_section(
            |l| fl!(l, "section-display"),
            Section::Display,
            rows(children![
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                    UiContainer::MonitorList,
                ),
                segments(|l| fl!(l, "label-layout"), Cycler::Layout),
            ]),
        ),
        shown_section(
            |l| fl!(l, "section-sound"),
            Section::Sound,
            rows(children![
                stepper(|l| fl!(l, "label-master-volume"), Cycler::MasterVolume),
                stepper(|l| fl!(l, "label-music-volume"), Cycler::MusicVolume),
                stepper(|l| fl!(l, "label-ambience-volume"), Cycler::AmbienceVolume),
                stepper(|l| fl!(l, "label-character-volume"), Cycler::CharacterVolume),
            ]),
        ),
        shown_section(
            |l| fl!(l, "section-interface"),
            Section::Interface,
            picker(|l| fl!(l, "label-language"), Cycler::Language),
        ),
    ]
}

fn show_volumes(volumes: Res<Volumes>, mut rows: Query<(&UiContainer, &mut Shown)>) {
    for (cycler, volume) in [
        (Cycler::MasterVolume, volumes.master),
        (Cycler::MusicVolume, volumes.music),
        (Cycler::AmbienceVolume, volumes.ambience),
        (Cycler::CharacterVolume, volumes.character),
    ] {
        show(&mut rows, cycler, number(percent(volume), volume, 0.0, 1.0));
    }
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
    lists: Query<(Entity, &UiContainer, Option<&Children>)>,
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
    for (list, container, children) in &lists {
        let filled = children.is_some_and(|c| !c.is_empty());
        if *container != UiContainer::MonitorList || (filled && !changed) {
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
                            .build(UiButton::Monitor(id.clone())),
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
    buttons: Query<(&UiButton, &Children)>,
    mut texts: Query<&mut Text>,
) {
    for (button, children) in &buttons {
        let UiButton::Monitor(id) = button else {
            continue;
        };
        let text = match shown_stage(&screen, &monitors, id) {
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

// For the stage the panel shows, with two monitors or more showing it.
fn show_layout(
    screen: Res<ScreenSettings>,
    monitors: Res<Monitors>,
    stage: Focused<StageKey>,
    locale: Res<Locale>,
    mut nodes: Query<(&UiContainer, &mut Node)>,
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    let stage = stage.get().map(|key| key.0);
    let several = monitors.0.iter().filter(|m| m.stage.is_some() && m.stage == stage).count() > 1;
    show_container(&mut nodes, UiContainer::Row(Cycler::Layout), several);
    let label = |span: &bool| match span {
        false => fl!(locale, "layout-mirror"),
        true => fl!(locale, "layout-span"),
    };
    let span = stage.is_some_and(|stage| screen.spanned.contains(&stage));
    show(&mut rows, Cycler::Layout, choice(&[false, true], &span, label));
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
    buttons: Query<&UiButton>,
    monitors: Res<Monitors>,
    mut screen: ResMut<ScreenSettings>,
) {
    let Ok(UiButton::Monitor(id)) = buttons.get(event.entity) else {
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

fn show_language(
    ui: Res<UiState>,
    locale: Res<Locale>,
    languages: Res<Languages>,
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    let label = |tag: &Option<String>| match tag {
        None => fl!(locale, "language-system"),
        Some(tag) => languages.name(tag).unwrap_or(tag).to_string(),
    };
    show(&mut rows, Cycler::Language, choice(&options(&languages), &ui.language, label));
}

// None, following the system's languages, comes first.
fn options(languages: &Languages) -> Vec<Option<String>> {
    std::iter::once(None).chain(languages.0.iter().map(|l| Some(l.tag.clone()))).collect()
}

fn on_step(
    event: On<Step>,
    languages: Res<Languages>,
    mut ui: ResMut<UiState>,
    mut screen: ResMut<ScreenSettings>,
    stage: Focused<StageKey>,
    mut volumes: ResMut<Volumes>,
) {
    let volume = match event.cycler {
        Cycler::Layout => {
            let Some(stage) = stage.get().map(|key| key.0) else {
                return;
            };
            if flip(screen.spanned.contains(&stage), &event) {
                screen.spanned.insert(stage);
            } else {
                screen.spanned.remove(&stage);
            }
            return;
        }
        Cycler::Language => {
            let options = options(&languages);
            if let Some(next) = step(&options, &ui.language, &event) {
                ui.language = next;
            }
            return;
        }
        Cycler::MasterVolume => &mut volumes.master,
        Cycler::MusicVolume => &mut volumes.music,
        Cycler::AmbienceVolume => &mut volumes.ambience,
        Cycler::CharacterVolume => &mut volumes.character,
        _ => return,
    };
    // Rounded so that repeated steps land on whole percents rather than drifting off them.
    let next = (*volume + event.by() as f32 * VOLUME_STEP).clamp(0.0, 1.0);
    *volume = (next * 100.0).round() / 100.0;
}
