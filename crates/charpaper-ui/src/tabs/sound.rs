use bevy::prelude::*;
use charpaper_settings::Volumes;
use i18n_embed_fl::fl;

use crate::row::percent;
use crate::row::*;
use crate::section::Section;
use crate::section::section;

const VOLUME: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };

// How loud the app is. Every stage shares it.
pub(crate) fn page() -> impl Bundle {
    children![section(
        |l| fl!(l, "section-sound"),
        Section::Sound,
        children![
            Setting {
                label: |l| fl!(l, "label-master-volume"),
                help: |l| fl!(l, "help-master-volume"),
                show: |w, _| Some(volume(w.resource::<Volumes>().master)),
                change: |w, step| {
                    let mut volumes = w.resource_mut::<Volumes>();
                    volumes.master = VOLUME.nudge(volumes.master, step);
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-music-volume"),
                help: |l| fl!(l, "help-music-volume"),
                show: |w, _| Some(volume(w.resource::<Volumes>().music)),
                change: |w, step| {
                    let mut volumes = w.resource_mut::<Volumes>();
                    volumes.music = VOLUME.nudge(volumes.music, step);
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-ambience-volume"),
                help: |l| fl!(l, "help-ambience-volume"),
                show: |w, _| Some(volume(w.resource::<Volumes>().ambience)),
                change: |w, step| {
                    let mut volumes = w.resource_mut::<Volumes>();
                    volumes.ambience = VOLUME.nudge(volumes.ambience, step);
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-character-volume"),
                help: |l| fl!(l, "help-character-volume"),
                show: |w, _| Some(volume(w.resource::<Volumes>().character)),
                change: |w, step| {
                    let mut volumes = w.resource_mut::<Volumes>();
                    volumes.character = VOLUME.nudge(volumes.character, step);
                },
            }
            .stepper(),
        ],
    )]
}

fn volume(volume: f32) -> Shown {
    VOLUME.number(percent(volume), volume)
}
