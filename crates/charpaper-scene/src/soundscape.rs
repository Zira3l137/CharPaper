use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use charpaper_audio::FadeOut;
use charpaper_audio::Fader;
use charpaper_audio::Layer;
use charpaper_audio::Pause;
use charpaper_audio::Sound;
use charpaper_audio::SoundFailed;
use charpaper_settings::MusicOrder;
use charpaper_settings::MusicSettings;

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
            .add_observer(play_track)
            .add_systems(
                Update,
                (follow_environment, next_track, follow_pause).chain().in_set(SceneSet::Run),
            )
            .add_systems(Update, listen.in_set(SceneSet::Run));
    }
}

#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct PlayTrack {
    pub entity: Entity,
    pub index: usize,
}

const SKIP_FADE: Duration = Duration::from_millis(500);

fn play_track(
    event: On<PlayTrack>,
    mut commands: Commands,
    mut stages: Query<(&mut Soundscape, &mut MusicSettings)>,
) {
    let Ok((mut scape, mut settings)) = stages.get_mut(event.entity) else {
        return;
    };
    if event.index >= scape.music.len() {
        return;
    }
    if let Some((track, _)) = scape.track.take() {
        commands.entity(track).insert(FadeOut(SKIP_FADE));
    }
    scape.requested = Some(event.index);
    if settings.paused {
        settings.paused = false;
    }
}

#[derive(Component, Default)]
pub struct Soundscape {
    // The suite's folder and the environment's name.
    shown: Option<(String, String)>,
    music: Vec<PathBuf>,
    next: usize,
    // The track playing, and its place in `music`.
    track: Option<(Entity, usize)>,
    requested: Option<usize>,
}

impl Soundscape {
    pub fn tracks(&self) -> Vec<String> {
        let name = |p: &PathBuf| p.file_stem().map(|s| s.to_string_lossy().into_owned());
        self.music.iter().map(|p| name(p).unwrap_or_default()).collect()
    }

    pub fn playing(&self) -> Option<usize> {
        self.track.map(|(_, index)| index)
    }
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
    mut stages: Query<(Entity, &ActiveSuite, &MusicSettings, &mut Soundscape)>,
    assets: Res<AssetServer>,
    sounds: Query<(), With<Sound>>,
) {
    for (stage, suite, settings, mut scape) in &mut stages {
        // Read first: touching `scape` mutably marks it changed, and the panel redraws then.
        let playing = scape.track.is_some_and(|(track, _)| sounds.contains(track));
        if scape.music.is_empty() || playing || settings.paused {
            continue;
        }
        play_next(&mut commands, stage, suite, &assets, settings.order, &mut scape);
    }
}

fn play_next(
    commands: &mut Commands,
    stage: Entity,
    suite: &ActiveSuite,
    assets: &AssetServer,
    order: MusicOrder,
    scape: &mut Soundscape,
) {
    let len = scape.music.len();
    let last = scape.track.map(|(_, index)| index);
    let index = match (scape.requested.take(), order) {
        (Some(index), _) => index,
        (None, MusicOrder::InOrder) => scape.next,
        (None, MusicOrder::RepeatOne) => last.unwrap_or(scape.next),
        (None, MusicOrder::Shuffle) => shuffled(len, last),
    } % len;
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

fn shuffled(len: usize, last: Option<usize>) -> usize {
    use std::hash::BuildHasher;
    let roll = std::hash::RandomState::new().hash_one(()) as usize;
    match last {
        Some(last) if len > 1 => (last + 1 + roll % (len - 1)) % len,
        _ => roll % len,
    }
}

fn follow_pause(
    mut commands: Commands,
    stages: Query<(&Soundscape, &MusicSettings)>,
    tracks: Query<Has<Pause>, With<Sound>>,
) {
    for (scape, settings) in &stages {
        let Some((track, _)) = scape.track else {
            continue;
        };
        match (settings.paused, tracks.get(track)) {
            (true, Ok(false)) => {
                commands.entity(track).insert(Pause);
            }
            (false, Ok(true)) => {
                commands.entity(track).remove::<Pause>();
            }
            _ => {}
        }
    }
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
