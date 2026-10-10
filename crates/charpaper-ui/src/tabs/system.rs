use bevy::prelude::*;
use charpaper_audio::Volumes;
use charpaper_scene::Monitors;
use charpaper_scene::ScreenSettings;
use i18n_embed_fl::fl;

use crate::UiState;
use crate::elements::Cycler;
use crate::elements::Section;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiContainer;
use crate::locale::Languages;
use crate::locale::Locale;
use crate::widgets::percent;
use crate::widgets::*;

const VOLUME_STEP: f32 = 0.05;

// Settings of the app itself rather than of what it shows: which monitor it is on, how loud
// it is, and the panel's language.
pub(crate) struct SystemTabPlugin;

impl Plugin for SystemTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_step).add_systems(
            Update,
            (
                show_language
                    .run_if(resource_changed::<UiState>.or_eager(resource_changed::<Locale>)),
                show_volumes.run_if(resource_changed::<Volumes>),
                show_monitor.run_if(
                    resource_changed::<ScreenSettings>
                        .or_eager(resource_changed::<Monitors>)
                        .or_eager(resource_changed::<Locale>),
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
            picker(|l| fl!(l, "label-monitor"), Cycler::Monitor),
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

fn show_monitor(
    screen: Res<ScreenSettings>,
    monitors: Res<Monitors>,
    locale: Res<Locale>,
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    let label = |option: &Option<String>| match option {
        None => fl!(locale, "monitor-primary"),
        Some(id) => match monitors.get(id) {
            Some(m) => format!("{} · {}×{}", m.name, m.width, m.height),
            None => fl!(locale, "monitor-missing"),
        },
    };
    let options = monitor_options(&monitors, &screen);
    show(&mut rows, Cycler::Monitor, choice(&options, &screen.monitor, label));
}

// The primary monitor first, then every connected one. A chosen monitor that is unplugged
// comes last, so the row still shows it was chosen.
fn monitor_options(monitors: &Monitors, screen: &ScreenSettings) -> Vec<Option<String>> {
    let connected = monitors.0.iter().map(|m| Some(m.id.clone()));
    let missing = screen.monitor.clone().filter(|id| monitors.get(id).is_none());
    std::iter::once(None).chain(connected).chain(missing.map(Some)).collect()
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
    monitors: Res<Monitors>,
    mut screen: ResMut<ScreenSettings>,
    mut ui: ResMut<UiState>,
    mut volumes: ResMut<Volumes>,
) {
    let volume = match event.cycler {
        Cycler::Monitor => {
            let options = monitor_options(&monitors, &screen);
            if let Some(next) = step(&options, &screen.monitor, &event) {
                screen.set_if_neq(ScreenSettings { monitor: next });
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
