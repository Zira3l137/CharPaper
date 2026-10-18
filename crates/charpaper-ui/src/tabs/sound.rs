use bevy::prelude::*;
use charpaper_settings::Volumes;
use i18n_embed_fl::fl;

use crate::elements::Cycler;
use crate::elements::Section;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiContainer;
use crate::panel::new_panel;
use crate::widgets::percent;
use crate::widgets::*;

const VOLUME_STEP: f32 = 0.05;

// How loud the app is, in the Sound tab. Every stage shares it.
pub(crate) struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_step).add_systems(
            Update,
            show_volumes.run_if(resource_changed::<Volumes>.or_eager(new_panel)),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![shown_section(
        |l| fl!(l, "section-sound"),
        Section::Sound,
        rows(children![
            stepper(|l| fl!(l, "label-master-volume"), Cycler::MasterVolume),
            stepper(|l| fl!(l, "label-music-volume"), Cycler::MusicVolume),
            stepper(|l| fl!(l, "label-ambience-volume"), Cycler::AmbienceVolume),
            stepper(|l| fl!(l, "label-character-volume"), Cycler::CharacterVolume),
        ]),
    )]
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

fn on_step(event: On<Step>, mut volumes: ResMut<Volumes>) {
    let volume = match event.cycler {
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
