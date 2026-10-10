use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use charpaper_audio::FadeOut;
use charpaper_audio::Layer;
use charpaper_audio::Sound;
use charpaper_audio::SoundFailed;

use crate::SceneSet;
use crate::assets::asset_path;
use crate::state::CharacterState;
use crate::suite::ActiveSuite;

// Music and ambience belong to the shown environment and crossfade when it changes. Music
// plays its tracks in order and starts over after the last one; ambience loops.
const CROSSFADE: Duration = Duration::from_secs(2);

pub(crate) struct SoundscapePlugin;

impl Plugin for SoundscapePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Soundscape>()
            .add_observer(skip_broken_track)
            .add_systems(Update, (follow_environment, next_track).chain().in_set(SceneSet::Run));
    }
}

#[derive(Resource, Default)]
struct Soundscape {
    // The suite's folder and the environment's name.
    shown: Option<(String, String)>,
    music: Vec<PathBuf>,
    next: usize,
    // The track playing, and its place in `music`.
    track: Option<(Entity, usize)>,
}

#[derive(Component)]
struct EnvironmentSound;

fn follow_environment(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    state: Res<CharacterState>,
    assets: Res<AssetServer>,
    playing: Query<Entity, (With<EnvironmentSound>, Without<FadeOut>)>,
    mut scape: ResMut<Soundscape>,
) {
    let wanted = suite.as_ref().zip(state.environment.as_ref());
    let wanted = wanted.map(|(suite, name)| (suite.folder(), name.clone()));
    if scape.shown == wanted {
        return;
    }
    for entity in &playing {
        commands.entity(entity).insert(FadeOut(CROSSFADE));
    }
    *scape = Soundscape { shown: wanted, ..default() };

    let (Some(suite), Some(name)) = (suite, &state.environment) else {
        return;
    };
    let Some(environment) = suite.environments.iter().find(|e| &e.name == name) else {
        return;
    };
    if let Some(ambience) = &environment.ambience {
        commands.spawn((
            EnvironmentSound,
            Sound {
                source: assets.load(asset_path(&suite, ambience)),
                layer: Layer::Ambience,
                looped: true,
                fade_in: CROSSFADE,
            },
        ));
    }
    scape.music = environment.music.clone();
}

fn next_track(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
    sounds: Query<(), With<Sound>>,
    mut scape: ResMut<Soundscape>,
) {
    let Some(suite) = suite else {
        return;
    };
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
            Sound {
                source: assets.load(asset_path(&suite, path)),
                layer: Layer::Music,
                looped: false,
                fade_in,
            },
        ))
        .id();
    scape.track = Some((track, index));
}

// Dropped for as long as the environment is shown, so a folder of broken files doesn't
// retry them forever.
fn skip_broken_track(failed: On<SoundFailed>, mut scape: ResMut<Soundscape>) {
    let Some((track, index)) = scape.track else {
        return;
    };
    if failed.entity == track {
        scape.music.remove(index);
        scape.next = index;
    }
}
