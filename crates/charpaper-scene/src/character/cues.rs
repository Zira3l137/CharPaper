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
use crate::character::CROSSFADE;
use crate::character::CharacterClips;
use crate::character::animation::Playing;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

// The clip a stage's character plays, and how far it had got when last looked at.
#[derive(Component, Default)]
pub(crate) struct CueClock {
    clip: Option<String>,
    // Its seek time and completed loops, or None before its first frame.
    last: Option<(f32, u32)>,
    // Its cues' files, loaded as soon as it starts so the first cue isn't late.
    sounds: BTreeMap<PathBuf, Handle<AudioSource>>,
}

// A sound one of a stage's clips played, as a child of that stage.
#[derive(Component)]
pub(crate) struct CueSound;

// Follows the selected clip itself rather than using Bevy's animation events: those fire for
// a clip fading out too, which would double every footstep during a crossfade.
pub(crate) fn play_cues(
    mut commands: Commands,
    mut stages: Query<(Entity, Option<&ActiveSuite>, Option<&CharacterClips>, &mut CueClock)>,
    armatures: Query<(Entity, &AnimationPlayer, &Playing)>,
    stage_of: StageOf,
    hush: Res<Hush>,
    assets: Res<AssetServer>,
    playing: Query<(Entity, &ChildOf), (With<CueSound>, Without<FadeOut>)>,
) {
    for (stage, suite, clips, mut clock) in &mut stages {
        let armature =
            armatures.iter().find(|(armature, ..)| stage_of.get(*armature) == Some(stage));
        let armature = armature.map(|(_, player, playing)| (player, playing));
        play_stage_cues(
            &mut commands,
            stage,
            (suite, clips, armature),
            &hush,
            &assets,
            &playing,
            &mut clock,
        );
    }
}

fn play_stage_cues(
    commands: &mut Commands,
    stage: Entity,
    (suite, clips, armature): (
        Option<&ActiveSuite>,
        Option<&CharacterClips>,
        Option<(&AnimationPlayer, &Playing)>,
    ),
    hush: &Hush,
    assets: &AssetServer,
    playing: &Query<(Entity, &ChildOf), (With<CueSound>, Without<FadeOut>)>,
    clock: &mut CueClock,
) {
    let current = armature.and_then(|(_, Playing(name))| name.clone());
    let clip = current.as_deref().and_then(|name| clips?.get(name));

    if clock.clip != current {
        // Every sound stops with the clip that played it, fading out with the animation.
        for (entity, parent) in playing {
            if parent.parent() == stage {
                commands.entity(entity).insert(FadeOut(CROSSFADE));
            }
        }
        *clock = CueClock { clip: current, ..default() };
        if let (Some(clip), Some(suite)) = (clip, suite) {
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
                ChildOf(stage),
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
