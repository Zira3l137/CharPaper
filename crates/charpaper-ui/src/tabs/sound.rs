use bevy::prelude::*;
use charpaper_scene::FocusedStage;
use charpaper_scene::PlayTrack;
use charpaper_scene::Soundscape;
use charpaper_settings::MusicOrder;
use charpaper_settings::MusicSettings;
use charpaper_settings::Volumes;
use i18n_embed_fl::fl;

use crate::row::percent;
use crate::row::*;
use crate::section::Section;
use crate::section::section;

const VOLUME: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };

// How loud the app is. Every stage shares it.
pub(crate) fn page() -> impl Bundle {
    children![
        section(
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
        ),
        section(
            |l| fl!(l, "section-music"),
            Section::Music,
            children![
                Setting {
                    label: |l| fl!(l, "label-track"),
                    help: |l| fl!(l, "help-track"),
                    show: |w, _| {
                        let music = w.focused::<Soundscape>()?;
                        let names = music.tracks();
                        let options: Vec<usize> = (0..names.len()).collect();
                        let current = music.playing().unwrap_or(usize::MAX);
                        let shown = choice(&options, &current, |&i| names[i].clone());
                        Some(shown.when(!names.is_empty()))
                    },
                    change: |w, step| {
                        let (Some(stage), Some(music)) =
                            (w.resource::<FocusedStage>().0, w.focused::<Soundscape>())
                        else {
                            return;
                        };
                        let options: Vec<usize> = (0..music.tracks().len()).collect();
                        let current = music.playing().unwrap_or(usize::MAX);
                        if let Some(index) = stepped(&options, &current, step) {
                            w.trigger(PlayTrack { entity: stage, index });
                        }
                    },
                }
                .picker(),
                Setting {
                    label: |l| fl!(l, "label-playback"),
                    help: |l| fl!(l, "help-playback"),
                    show: |w, l| {
                        let has_music = !w.focused::<Soundscape>()?.tracks().is_empty();
                        let paused = w.focused::<MusicSettings>()?.paused;
                        let label = |&paused: &bool| match paused {
                            false => fl!(l, "value-play"),
                            true => fl!(l, "value-pause"),
                        };
                        Some(choice(&[false, true], &paused, label).when(has_music))
                    },
                    change: |w, step| w.edit(|m: &mut MusicSettings| flip(&mut m.paused, step)),
                }
                .segments(),
                Setting {
                    label: |l| fl!(l, "label-order"),
                    help: |l| fl!(l, "help-order"),
                    show: |w, l| {
                        let has_music = !w.focused::<Soundscape>()?.tracks().is_empty();
                        let order = w.focused::<MusicSettings>()?.order;
                        let label = |order: &MusicOrder| match order {
                            MusicOrder::InOrder => fl!(l, "value-in-order"),
                            MusicOrder::Shuffle => fl!(l, "value-shuffle"),
                            MusicOrder::RepeatOne => fl!(l, "value-repeat-one"),
                        };
                        Some(choice(&MusicOrder::ALL, &order, label).when(has_music))
                    },
                    change: |w, step| {
                        w.edit(|m: &mut MusicSettings| pick(&MusicOrder::ALL, &mut m.order, step));
                    },
                }
                .segments(),
            ],
        ),
    ]
}

fn volume(volume: f32) -> Shown {
    VOLUME.number(percent(volume), volume)
}
