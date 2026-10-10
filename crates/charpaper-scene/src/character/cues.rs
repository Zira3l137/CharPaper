use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use charpaper_audio::AudioSource;
use charpaper_audio::FadeOut;
use charpaper_audio::Hush;
use charpaper_audio::Layer;
use charpaper_audio::Sound;

use crate::assets::asset_path;
use crate::character::CharacterClips;
use crate::character::animation::Playing;
use crate::config::SceneConfig;
use crate::suite::ActiveSuite;

// The clip being followed, and how far it had got when last looked at.
#[derive(Resource, Default)]
pub(crate) struct CueClock {
    clip: Option<String>,
    // Its seek time and completed loops, or None before its first frame.
    last: Option<(f32, u32)>,
    // Its cues' files, loaded as soon as it starts so the first cue isn't late.
    sounds: BTreeMap<PathBuf, Handle<AudioSource>>,
}

#[derive(Component)]
pub(crate) struct CueSound;

// Follows the selected clip itself rather than using Bevy's animation events: those fire for
// a clip fading out too, which would double every footstep during a crossfade.
pub(crate) fn play_cues(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    clips: Option<Res<CharacterClips>>,
    armature: Query<(&AnimationPlayer, &Playing)>,
    config: Res<SceneConfig>,
    hush: Res<Hush>,
    assets: Res<AssetServer>,
    playing: Query<Entity, (With<CueSound>, Without<FadeOut>)>,
    mut clock: ResMut<CueClock>,
) {
    let armature = armature.single().ok();
    let current = armature.and_then(|(_, Playing(name))| name.clone());
    let clip = current.as_deref().and_then(|name| clips.as_ref()?.get(name));

    if clock.clip != current {
        // Every sound stops with the clip that played it, fading out with the animation.
        let fade = Duration::from_secs_f32(config.animation_crossfade_secs.max(0.0));
        for entity in &playing {
            commands.entity(entity).insert(FadeOut(fade));
        }
        *clock = CueClock { clip: current, ..default() };
        if let (Some(clip), Some(suite)) = (clip, &suite) {
            for cue in &clip.cues {
                let handle = assets.load(asset_path(suite, &cue.sound));
                clock.sounds.insert(cue.sound.clone(), handle);
            }
        }
    }

    let (Some(clip), Some((player, _))) = (clip, armature) else {
        return;
    };
    let Some(active) = player.animation(clip.node) else {
        return;
    };
    let (now, loops) = (active.seek_time(), active.completions());
    let passed = |at: f32| match clock.last {
        None => at <= now,
        // A loop came round: the rest of the last one, then the start of this one.
        Some((last, before)) if loops > before && !active.is_finished() => at > last || at <= now,
        Some((last, _)) => at > last && at <= now,
    };
    // While hushed nothing is heard, and whatever was started would all play at once later.
    if !hush.0 {
        for cue in clip.cues.iter().filter(|cue| passed(cue.at)) {
            let Some(source) = clock.sounds.get(&cue.sound) else {
                continue;
            };
            commands.spawn((
                CueSound,
                Sound {
                    source: source.clone(),
                    layer: Layer::Character,
                    looped: false,
                    fade_in: Duration::ZERO,
                },
            ));
        }
    }
    clock.last = Some((now, loops));
}
