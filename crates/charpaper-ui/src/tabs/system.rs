use bevy::prelude::*;
use charpaper_audio::Volumes;
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

// Settings of the app itself rather than of what it shows: how loud it is, and the panel's
// language.
pub(crate) struct SystemTabPlugin;

impl Plugin for SystemTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_step).add_systems(
            Update,
            (
                show_language
                    .run_if(resource_changed::<UiState>.or_eager(resource_changed::<Locale>)),
                show_volumes.run_if(resource_changed::<Volumes>),
            ),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![
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
    mut volumes: ResMut<Volumes>,
) {
    let volume = match event.cycler {
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
