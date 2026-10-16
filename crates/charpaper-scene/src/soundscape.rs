use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use charpaper_audio::FadeOut;
use charpaper_audio::Fader;
use charpaper_audio::Layer;
use charpaper_audio::Sound;
use charpaper_audio::SoundFailed;

use crate::SceneSet;
use crate::assets::asset_path;
use crate::screens::ShowsStage;
use crate::screens::Unseen;
use crate::stage::FocusedStage;
use crate::stage::StageKey;
use crate::state::CharacterState;
use crate::suite::ActiveSuite;

// Music and ambience belong to the shown environment and crossfade when it changes. Music
// plays its tracks in order and starts over after the last one; ambience loops.
const CROSSFADE: Duration = Duration::from_secs(2);

// Only the stage under the cursor is heard. The cursor has to stay on its monitors this long
// before the sound goes over to it, so sweeping across a monitor on the way to another changes
// nothing; a stage left behind keeps playing, silently, and is where it would be on coming back.
const LINGER: Duration = Duration::from_millis(300);
const HANDOVER: Duration = Duration::from_millis(1500);

pub(crate) struct SoundscapePlugin;

impl Plugin for SoundscapePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(skip_broken_track)
            .add_systems(Update, (follow_environment, next_track).chain().in_set(SceneSet::Run))
            .add_systems(Update, listen.in_set(SceneSet::Run));
    }
}

#[derive(Component, Default)]
pub(crate) struct Soundscape {
    // The suite's folder and the environment's name.
    shown: Option<(String, String)>,
    music: Vec<PathBuf>,
    next: usize,
    // The track playing, and its place in `music`.
    track: Option<(Entity, usize)>,
}

// A sound of a stage's environment, as a child of that stage.
#[derive(Component)]
struct EnvironmentSound;

fn follow_environment(
    mut commands: Commands,
    mut stages: Query<(Entity, Option<&ActiveSuite>, &CharacterState, &mut Soundscape)>,
    assets: Res<AssetServer>,
    playing: Query<(Entity, &ChildOf), (With<EnvironmentSound>, Without<FadeOut>)>,
) {
    for (stage, suite, state, mut scape) in &mut stages {
        let wanted = suite.zip(state.environment.as_ref());
        let wanted = wanted.map(|(suite, name)| (suite.folder(), name.clone()));
        if scape.shown == wanted {
            continue;
        }
        for (entity, parent) in &playing {
            if parent.parent() == stage {
                commands.entity(entity).insert(FadeOut(CROSSFADE));
            }
        }
        *scape = Soundscape { shown: wanted, ..default() };

        let (Some(suite), Some(name)) = (suite, &state.environment) else {
            continue;
        };
        let Some(environment) = suite.environments.iter().find(|e| &e.name == name) else {
            continue;
        };
        if let Some(ambience) = &environment.ambience {
            commands.spawn((
                EnvironmentSound,
                ChildOf(stage),
                Sound {
                    source: assets.load(asset_path(suite, ambience)),
                    layer: Layer::Ambience,
                    looped: true,
                    fade_in: CROSSFADE,
                },
            ));
        }
        scape.music = environment.music.clone();
    }
}

fn next_track(
    mut commands: Commands,
    mut stages: Query<(Entity, &ActiveSuite, &mut Soundscape)>,
    assets: Res<AssetServer>,
    sounds: Query<(), With<Sound>>,
) {
    for (stage, suite, mut scape) in &mut stages {
        play_next(&mut commands, stage, suite, &assets, &sounds, &mut scape);
    }
}

fn play_next(
    commands: &mut Commands,
    stage: Entity,
    suite: &ActiveSuite,
    assets: &AssetServer,
    sounds: &Query<(), With<Sound>>,
    scape: &mut Soundscape,
) {
    if scape.music.is_empty() || scape.track.is_some_and(|(track, _)| sounds.contains(track)) {
        return;
    }
    let index = scape.next % scape.music.len();
    scape.next = index + 1;
    // Only the first track fades in.
    let fade_in = if scape.track.is_none() { CROSSFADE } else { Duration::ZERO };
    let path = &scape.music[index];
    debug!("music: {}", path.display());
    let track = commands
        .spawn((
            EnvironmentSound,
            ChildOf(stage),
            Sound {
                source: assets.load(asset_path(suite, path)),
                layer: Layer::Music,
                looped: false,
                fade_in,
            },
        ))
        .id();
    scape.track = Some((track, index));
}

#[derive(Default)]
struct Listening {
    heard: Option<Entity>,
    // The stage the cursor moved to, and when.
    coming: Option<(Entity, Duration)>,
}

// A stage nobody can see, with every monitor showing it covered, is silent too.
fn listen(
    time: Res<Time<Real>>,
    focused: Res<FocusedStage>,
    mut listening: Local<Listening>,
    mut stages: Query<(Entity, &StageKey, &mut Fader)>,
    screens: Query<(&ShowsStage, Has<Unseen>)>,
) {
    let now = time.elapsed();
    let heard = listening.heard.filter(|&stage| stages.contains(stage));
    let next = match (focused.0, heard) {
        (Some(focused), Some(heard)) if focused != heard => {
            let since = match listening.coming {
                Some((coming, since)) if coming == focused => since,
                _ => now,
            };
            listening.coming = Some((focused, since));
            if now.saturating_sub(since) < LINGER { Some(heard) } else { Some(focused) }
        }
        // The first stage is heard at once, and so is the next when the one heard goes away.
        (focused, None) => focused,
        (_, heard) => {
            listening.coming = None;
            heard
        }
    };
    listening.heard = next;

    for (stage, key, mut fader) in &mut stages {
        let seen = screens.iter().any(|(shows, unseen)| shows.0 == key.0 && !unseen);
        let level = if Some(stage) == next && seen { 1.0 } else { 0.0 };
        if fader.level != level {
            *fader = Fader { level, over: HANDOVER };
        }
    }
}

// Dropped for as long as the environment is shown, so a folder of broken files doesn't
// retry them forever.
fn skip_broken_track(failed: On<SoundFailed>, mut stages: Query<&mut Soundscape>) {
    for mut scape in &mut stages {
        let Some((track, index)) = scape.track else {
            continue;
        };
        if failed.entity == track {
            scape.music.remove(index);
            scape.next = index;
        }
    }
}
