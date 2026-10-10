use std::time::Duration;

use bevy::prelude::*;
use rodio::Player;

use crate::AudioSource;
use crate::FadeOut;
use crate::Sound;
use crate::SoundFailed;
use crate::Volumes;
use crate::envelope::Envelope;
use crate::output::Output;

// A sound that is playing.
#[derive(Component)]
pub(crate) struct Voice {
    player: Player,
    envelope: Envelope,
    // See Output::generation.
    generation: u32,
    // Where in the file the player began. A player counts its position from there.
    offset: Duration,
}

// Waits while there is no device, so a sound asked for meanwhile starts once one is back.
pub(crate) fn start_sounds(
    mut commands: Commands,
    output: Res<Output>,
    assets: Res<AssetServer>,
    sources: Res<Assets<AudioSource>>,
    volumes: Res<Volumes>,
    queued: Query<(Entity, &Sound), (Without<Voice>, Without<FadeOut>)>,
) {
    let Some(device) = output.device() else {
        return;
    };
    for (entity, sound) in &queued {
        let Some(source) = sources.get(&sound.source) else {
            // The asset server has said why already.
            if assets.load_state(sound.source.id()).is_failed() {
                fail(&mut commands, entity);
            }
            continue;
        };
        let envelope = Envelope::fading_in(sound.fade_in);
        match device.play(source.clone(), sound.looped, Duration::ZERO, &envelope) {
            Ok(player) => {
                player.set_volume(volumes.of(sound.layer));
                commands.entity(entity).insert(Voice {
                    player,
                    envelope,
                    generation: output.generation,
                    offset: Duration::ZERO,
                });
            }
            Err(err) => {
                warn!("cannot play {}: {err}", describe(sound));
                fail(&mut commands, entity);
            }
        }
    }
}

// A loop starts over; anything else picks up where it was.
pub(crate) fn move_to_new_device(
    mut commands: Commands,
    output: Res<Output>,
    sources: Res<Assets<AudioSource>>,
    volumes: Res<Volumes>,
    mut voices: Query<(Entity, &Sound, &mut Voice)>,
) {
    let Some(device) = output.device() else {
        return;
    };
    for (entity, sound, mut voice) in &mut voices {
        if voice.generation == output.generation {
            continue;
        }
        let from = match sound.looped {
            true => Duration::ZERO,
            false => voice.offset + voice.player.get_pos(),
        };
        let played = sources
            .get(&sound.source)
            .map(|source| device.play(source.clone(), sound.looped, from, &voice.envelope));
        match played {
            Some(Ok(player)) => {
                player.set_volume(volumes.of(sound.layer));
                voice.player = player;
                voice.generation = output.generation;
                voice.offset = from;
            }
            _ => fail(&mut commands, entity),
        }
    }
}

pub(crate) fn apply_volumes(
    volumes: Res<Volumes>,
    output: Res<Output>,
    voices: Query<(&Sound, &Voice)>,
) {
    if let Some(device) = output.device() {
        device.master.set_volume(volumes.master());
    }
    for (sound, voice) in &voices {
        voice.player.set_volume(volumes.of(sound.layer));
    }
}

pub(crate) fn fade_out(
    mut commands: Commands,
    fading: Query<(Entity, Ref<FadeOut>, Option<&Voice>)>,
) {
    for (entity, fade, voice) in &fading {
        match voice {
            None => commands.entity(entity).despawn(),
            Some(voice) if fade.is_added() => voice.envelope.fade_to(0.0, fade.0),
            Some(voice) if voice.envelope.level() == 0.0 => commands.entity(entity).despawn(),
            Some(_) => {}
        }
    }
}

pub(crate) fn despawn_finished(
    mut commands: Commands,
    voices: Query<(Entity, &Sound, &Voice), Without<FadeOut>>,
) {
    for (entity, sound, voice) in &voices {
        if !sound.looped && voice.player.empty() {
            commands.entity(entity).despawn();
        }
    }
}

fn fail(commands: &mut Commands, entity: Entity) {
    commands.trigger(SoundFailed { entity });
    commands.entity(entity).despawn();
}

fn describe(sound: &Sound) -> String {
    match sound.source.path() {
        Some(path) => path.to_string(),
        None => "a sound".to_string(),
    }
}
