use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::Justify;
use bevy::text::LineBreak;
use charpaper_scene::Focused;
use charpaper_scene::FocusedStage;
use charpaper_scene::PlayTrack;
use charpaper_scene::Soundscape;
use charpaper_scene::focused_changed;
use charpaper_settings::MusicOrder;
use charpaper_settings::MusicSettings;
use charpaper_settings::Volumes;
use i18n_embed_fl::fl;

use crate::icons::Icon;
use crate::locale::Locale;
use crate::locale::Tr;
use crate::panel::Hint;
use crate::panel::new_panel;
use crate::row::percent;
use crate::row::*;
use crate::section::Content;
use crate::section::Section;
use crate::section::section;
use crate::theme::*;
use crate::widgets::*;

const VOLUME: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };

// What's playing, and the music's buttons under it: the order it plays in, the track before,
// play or pause, and the track after. They act on the focused stage's music, as rows do.
pub(crate) struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            show_controls.run_if(
                focused_changed::<MusicSettings>
                    .or_eager(focused_changed::<Soundscape>)
                    .or_eager(resource_changed::<Locale>)
                    .or_eager(new_panel),
            ),
        );
    }
}

#[derive(Component)]
#[require(Content)]
struct Player;

// The track's name, and its place among the environment's tracks with whether it's paused.
#[derive(Component, Clone, Copy, PartialEq)]
enum Playing {
    Name,
    Place,
}

#[derive(Component, Clone, Copy, PartialEq)]
enum MusicButton {
    // Goes on from in order to shuffle to repeating one, and round.
    Order,
    Previous,
    PlayPause,
    Next,
}

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
                player(),
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
            ],
        ),
    ]
}

// Hidden while the environment has no music.
fn player() -> impl Bundle {
    let centered = TextLayout::new(Justify::Center, LineBreak::WordBoundary);
    let buttons = (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            margin: UiRect::top(Val::Px(10.0)),
            ..default()
        },
        Pickable::IGNORE,
        children![
            button(MusicButton::Order, Icon::Repeat, |l| fl!(l, "help-order")),
            button(MusicButton::Previous, Icon::Previous, |l| fl!(l, "help-previous")),
            button(MusicButton::PlayPause, Icon::Pause, |l| fl!(l, "help-playback")),
            button(MusicButton::Next, Icon::Next, |l| fl!(l, "help-next")),
        ],
    );
    (
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            padding: UiRect::new(Val::Px(12.0), Val::Px(12.0), Val::Px(12.0), Val::Px(14.0)),
            margin: UiRect::vertical(Val::Px(6.0)),
            border_radius: BorderRadius::all(RADIUS),
            ..default()
        },
        Fill(WELL_BG),
        Pickable::IGNORE,
        Player,
        children![
            translated(|l| fl!(l, "now-playing"), SMALL_SIZE, TEXT_DIM),
            (label("", TITLE_SIZE, TEXT), centered, Playing::Name),
            (label("", SMALL_SIZE, TEXT_DIM), centered, Playing::Place),
            buttons,
        ],
    )
}

// Round. Play and pause is the big one, filled in the text's color so it stands out in any
// theme.
fn button(kind: MusicButton, icon: Icon, hint: Tr) -> impl Bundle {
    let main = kind == MusicButton::PlayPause;
    let size = Val::Px(if main { 44.0 } else { 36.0 });
    (
        ButtonBuilder::icon(icon)
            .text_color(if main { CARD_BG } else { TEXT_LABEL })
            .fill(if main { TEXT } else { CARD_BG })
            .node(|n| {
                n.width = size;
                n.height = size;
                n.justify_content = JustifyContent::Center;
                n.align_items = AlignItems::Center;
                n.border_radius = BorderRadius::MAX;
            })
            .build(kind),
        Hint(hint),
        Hovered::default(),
    )
}

// Play shows while paused and pause while playing; the order button shows the order, in the
// accent unless it's the usual one.
fn show_controls(
    locale: Res<Locale>,
    music: Focused<MusicSettings>,
    soundscape: Focused<Soundscape>,
    mut players: Query<&mut Node, With<Player>>,
    buttons: Query<(&MusicButton, &Children)>,
    mut texts: Query<(&mut Text, &mut Ink), Without<Playing>>,
    mut lines: Query<(&mut Text, &Playing)>,
) {
    let (Some(music), Some(soundscape)) = (music.get(), soundscape.get()) else {
        return;
    };
    let tracks = soundscape.tracks();
    let shown = display(!tracks.is_empty());
    for mut node in &mut players {
        if node.display != shown {
            node.display = shown;
        }
    }
    let playing = soundscape.playing();
    let name = match playing.and_then(|index| tracks.get(index)) {
        Some(name) => name.clone(),
        None => fl!(locale, "nothing-playing"),
    };
    let place = playing.map(|index| {
        let (number, count) = (index + 1, tracks.len());
        let place = fl!(locale, "track-of", number = number, count = count);
        match music.paused {
            true => fl!(locale, "music-paused", place = place),
            false => place,
        }
    });
    for (mut text, line) in &mut lines {
        let wanted = match line {
            Playing::Name => name.as_str(),
            Playing::Place => place.as_deref().unwrap_or_default(),
        };
        if text.0 != wanted {
            wanted.clone_into(&mut text.0);
        }
    }
    for (button, children) in &buttons {
        let (icon, ink) = match button {
            MusicButton::PlayPause if music.paused => (Icon::Play, CARD_BG),
            MusicButton::PlayPause => (Icon::Pause, CARD_BG),
            MusicButton::Order => match music.order {
                MusicOrder::InOrder => (Icon::Repeat, TEXT_LABEL),
                MusicOrder::Shuffle => (Icon::Shuffle, ACCENT),
                MusicOrder::RepeatOne => (Icon::RepeatOne, ACCENT),
            },
            MusicButton::Previous | MusicButton::Next => continue,
        };
        let glyph = icon.glyph().to_string();
        for &child in &**children {
            if let Ok((mut text, mut color)) = texts.get_mut(child) {
                if text.0 != glyph {
                    text.0.clone_from(&glyph);
                }
                color.set_if_neq(Ink(ink));
            }
        }
    }
}

fn on_click(event: On<Pointer<Click>>, mut commands: Commands, buttons: Query<&MusicButton>) {
    if let Ok(&button) = buttons.get(event.entity) {
        commands.queue(move |world: &mut World| press(world, button));
    }
}

fn press(world: &mut World, button: MusicButton) {
    let by = match button {
        MusicButton::Order => {
            let step = Step { change: Change::By(1), continuous: false };
            world.edit(|m: &mut MusicSettings| pick(&MusicOrder::ALL, &mut m.order, &step));
            return;
        }
        MusicButton::PlayPause => {
            world.edit(|m: &mut MusicSettings| m.paused = !m.paused);
            return;
        }
        MusicButton::Previous => -1,
        MusicButton::Next => 1,
    };
    let (Some(stage), Some(music)) =
        (world.resource::<FocusedStage>().0, world.focused::<Soundscape>())
    else {
        return;
    };
    let count = music.tracks().len() as i64;
    if count == 0 {
        return;
    }
    // From nothing playing, the next track is the first and the one before it the last.
    let index = match music.playing() {
        Some(playing) => (playing as i64 + by).rem_euclid(count),
        None if by > 0 => 0,
        None => count - 1,
    };
    world.trigger(PlayTrack { entity: stage, index: index as usize });
}

fn volume(volume: f32) -> Shown {
    VOLUME.number(percent(volume), volume)
}
