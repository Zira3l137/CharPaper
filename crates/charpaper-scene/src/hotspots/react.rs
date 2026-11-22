// The four steps hotspots take each frame, after `mouse`: keep up with what plays, notice
// what the mouse sets off or lets go of, switch, and go off.

use bevy::prelude::*;
use charpaper_anim::Mode;
use charpaper_settings::GazeSettings;
use charpaper_suite::Trigger;

use super::Hotspots;
use super::Hover;
use super::Phase;
use super::Spot;
use super::actors::Actors;
use super::actors::Slot;
use super::areas::distance;
use super::areas::find_bones;
use super::mouse::Mouse;
use crate::CharacterState;
use crate::character::CharacterClips;

// How long the cursor rests on a hover hotspot before it goes off: a cursor passing over
// shouldn't set it off.
const HOVER_SECS: f64 = 0.35;
// How far the cursor may move between press and release for a click, in logical pixels.
const CLICK_SLOP: f32 = 6.0;
// How soon after it can go off again it has to, for that to count as in a row.
const STREAK_SECS: f64 = 3.0;

// Drops what has ended, finds bones, and stops hotspots whose actors no longer play what they
// wait for: a reaction meant for another animation would look wrong over this one.
pub(super) fn keep_up(
    time: Res<Time>,
    globals: Query<&GlobalTransform>,
    mut stages: Query<(Entity, &mut Hotspots)>,
    mut actors: Actors,
) {
    let now = time.elapsed_secs_f64();
    for (stage, mut hotspots) in &mut stages {
        let hotspots = &mut *hotspots;
        end_finished(stage, now, hotspots, &mut actors);
        let environment = actors.environment(stage);
        if hotspots.environment != environment {
            hotspots.environment = environment;
            hotspots.spots.iter_mut().for_each(|spot| spot.searched.fill(false));
        }
        for spot in &mut hotspots.spots {
            find_bones(stage, spot, &globals, &mut actors);
            spot.allowed = actors.allows(stage, &spot.hotspot.during);
            if !spot.allowed {
                stop(spot, now, &mut actors);
            }
        }
    }
}

// Turns what the mouse did into hotspots due to go off, and lets go of those held.
pub(super) fn notice(
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    mouse: Res<Mouse>,
    globals: Query<&GlobalTransform>,
    mut stages: Query<(Entity, &mut Hotspots)>,
    mut actors: Actors,
) {
    let now = time.elapsed_secs_f64();
    for (stage, mut hotspots) in &mut stages {
        let hotspots = &mut *hotspots;
        let seen = mouse.sight.filter(|(on, ..)| *on == stage);
        let front = |trigger: Trigger| {
            let (_, ray, _) = seen?;
            let spots = hotspots.spots.iter().enumerate();
            let hits = spots.filter(|(_, s)| s.allowed && s.hotspot.trigger == trigger);
            hits.filter_map(|(at, spot)| Some((at, distance(spot, ray, &globals)?)))
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(at, _)| at)
        };
        let (clicked, hovered) = (front(Trigger::Click), front(Trigger::Hover));

        // A click hotspot with clips to hold goes off as it's pressed, any other as it's let go.
        let position = seen.map(|(.., position)| position);
        let held = |at: usize| !hotspots.spots[at].hotspot.meanwhile.is_empty();
        if buttons.just_pressed(MouseButton::Left) {
            hotspots.pressed = clicked.zip(position);
            if let Some(at) = clicked.filter(|&at| held(at)) {
                hotspots.due.push(at);
            }
        }
        if buttons.just_released(MouseButton::Left)
            && let Some((at, from)) = hotspots.pressed.take()
            && clicked == Some(at)
            && position.is_some_and(|to| to.distance(from) <= CLICK_SLOP)
            && !held(at)
        {
            hotspots.due.push(at);
        }

        let came_back = mouse.came_back_after;
        for (at, spot) in hotspots.spots.iter_mut().enumerate() {
            let let_go = match spot.hotspot.trigger {
                Trigger::Click => !buttons.pressed(MouseButton::Left),
                Trigger::Hover => hovered != Some(at),
                Trigger::Idle => came_back.is_some(),
                Trigger::Welcome => false,
            };
            if let_go {
                release(spot, now, &mut actors);
            }
            let after = f64::from(spot.hotspot.after_secs());
            let due = match spot.hotspot.trigger {
                Trigger::Click => false,
                Trigger::Hover => hover(&mut spot.hover, hovered == Some(at), now),
                Trigger::Idle => {
                    spot.allowed
                        && idle_due(after, mouse.still(), mouse.real_now, spot.history.idled)
                }
                Trigger::Welcome => spot.allowed && came_back.is_some_and(|gap| gap >= after),
            };
            if due {
                hotspots.due.push(at);
            }
        }
    }
}

// Carries out the switches of hotspots that let go, or whose clips ended.
pub(super) fn switch(
    mut stages: Query<(
        Entity,
        &mut Hotspots,
        Option<&CharacterClips>,
        Option<&mut CharacterState>,
    )>,
    mut actors: Actors,
) {
    for (stage, mut hotspots, clips, mut state) in &mut stages {
        for spot in &mut hotspots.spots {
            if spot.phase == Phase::Switching {
                actors.switch(stage, &spot.hotspot.then, state.as_deref_mut(), clips);
                spot.phase = if spot.playing.is_empty() { Phase::Ready } else { Phase::Ending };
            }
        }
    }
}

pub(super) fn go_off(
    time: Res<Time>,
    mouse: Res<Mouse>,
    mut stages: Query<(Entity, &GazeSettings, &mut Hotspots, Option<&CharacterClips>)>,
    mut actors: Actors,
) {
    let now = time.elapsed_secs_f64();
    for (stage, settings, mut hotspots, clips) in &mut stages {
        let hotspots = &mut *hotspots;
        let due = std::mem::take(&mut hotspots.due);
        if !settings.reactions || hotspots.switching() {
            continue;
        }
        for at in due {
            let spot = &hotspots.spots[at];
            let in_row = match spot.hotspot.trigger {
                Trigger::Idle => spot.history.idled.is_some_and(|idled| {
                    // Not moved since it last went off.
                    idled >= mouse.real_now - mouse.still()
                }),
                Trigger::Welcome => false,
                Trigger::Click | Trigger::Hover => {
                    let cooldown = f64::from(spot.hotspot.cooldown.unwrap_or(0.0));
                    clicks_in_row(&spot.history, cooldown, now)
                }
            };
            if !fire(stage, at, (now, in_row), hotspots, clips, &mut actors) {
                continue;
            }
            let spot = &mut hotspots.spots[at];
            if spot.hotspot.trigger == Trigger::Idle {
                spot.history.idled = Some(mouse.real_now);
            }
            // Others may wait for what it switches to: they go off only once it has.
            if spot.switches_later() {
                break;
            }
        }
    }
}

// Whether a hover hotspot is due: the cursor has rested on it long enough, and hasn't since
// it last went off.
fn hover(state: &mut Hover, on: bool, now: f64) -> bool {
    match (&*state, on) {
        (_, false) => *state = Hover::Off,
        (Hover::Off, true) => *state = Hover::Since(now),
        (Hover::Since(since), true) if now - since >= HOVER_SECS => {
            *state = Hover::WentOff;
            return true;
        }
        _ => {}
    }
    false
}

// Whether an idle hotspot is due: the mouse has kept still `after` seconds, and it hasn't gone
// off since it last moved, or did `after` seconds ago.
fn idle_due(after: f64, still: f64, now: f64, idled: Option<f64>) -> bool {
    still >= after && idled.is_none_or(|idled| idled < now - still || now - idled >= after)
}

// Whether going off now counts as in a row: within a moment of when it could again.
fn clicks_in_row(history: &super::History, cooldown: f64, now: f64) -> bool {
    history.ended.is_some_and(|ended| {
        let free = history.fired.map_or(ended, |fired| ended.max(fired + cooldown));
        now - free <= STREAK_SECS
    })
}

// Whether it went off: it may be cooling down, still playing, or have nothing loaded to play.
fn fire(
    stage: Entity,
    at: usize,
    (now, in_row): (f64, bool),
    hotspots: &mut Hotspots,
    clips: Option<&CharacterClips>,
    actors: &mut Actors,
) -> bool {
    let spot = &hotspots.spots[at];
    let cooldown = f64::from(spot.hotspot.cooldown.unwrap_or(0.0));
    if spot.phase != Phase::Ready || spot.history.fired.is_some_and(|f| now - f < cooldown) {
        return false;
    }
    let streak = if in_row { spot.history.streak + 1 } else { 1 };
    let layer = spot.hotspot.layer;
    // The `while` clips first, so the clips it plays once play over them.
    let mut played: Vec<_> = (spot.hotspot.meanwhile.iter())
        .filter_map(|wanted| {
            let actor = wanted.actor.as_deref();
            actors.start(stage, actor, &wanted.clip, Slot::Loop(layer), clips)
        })
        .collect();
    let held = !played.is_empty();
    let once = if held { Slot::Top } else { Slot::Once(layer) };
    let mut waiting = false;
    for at_pick in 0..spot.picks.len() {
        let pick = &hotspots.spots[at].picks[at_pick];
        let choices = pick.tier(streak);
        if choices.is_empty() {
            waiting = true;
            continue;
        }
        let last = pick.last.and_then(|last| choices.iter().position(|&i| i == last));
        let choice = choices[random_other(&mut hotspots.seed, choices.len(), last)];
        let actor = pick.actor.clone();
        let name = pick.clips[choice].0.clone();
        if let Some(started) = actors.start(stage, actor.as_deref(), &name, once, clips) {
            hotspots.spots[at].picks[at_pick].last = Some(choice);
            played.push(started);
        }
    }
    // Nothing it plays has loaded, or its actors are in another environment. Clips that wait for
    // a later time in a row still count this one.
    if played.is_empty() && !waiting {
        return false;
    }
    let spot = &mut hotspots.spots[at];
    spot.playing = played;
    spot.history.fired = Some(now);
    spot.history.streak = streak;
    if held {
        spot.phase = Phase::Held;
    } else if spot.playing.is_empty() {
        spot.history.ended = Some(now);
        spot.done();
    } else {
        spot.phase = Phase::Playing;
    }
    if let Some(expression) = &spot.hotspot.expression
        && let Some((_, mut character)) = actors.character(stage)
    {
        character.reaction_face(Some(expression));
        hotspots.face = Some(at);
    }
    true
}

// Any of `count`, but not `last` while there are others.
fn random_other(seed: &mut u64, count: usize, last: Option<usize>) -> usize {
    // xorshift64: plenty for picking a clip.
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    match last.filter(|_| count > 1) {
        Some(last) => (last + 1 + (*seed % (count as u64 - 1)) as usize) % count,
        None => (*seed % count.max(1) as u64) as usize,
    }
}

// Lets go of a held hotspot's `while` clips; what it plays once plays on.
fn release(spot: &mut Spot, now: f64, actors: &mut Actors) {
    if spot.phase != Phase::Held {
        return;
    }
    spot.playing.retain(|played| {
        let looping = played.mode != Mode::Once;
        if looping {
            actors.stop(played);
        }
        !looping
    });
    spot.history.ended = Some(now);
    spot.done();
}

// Stops all it plays, at once Ready, without switching: what was switched away from no longer
// is.
fn stop(spot: &mut Spot, now: f64, actors: &mut Actors) {
    if spot.phase == Phase::Ready {
        return;
    }
    for played in spot.playing.drain(..) {
        actors.stop(&played);
    }
    spot.history.ended = Some(now);
    spot.phase = Phase::Ready;
}

// Drops what has stopped playing: run out, or taken over by another reaction. A hotspot whose
// clips have all gone is done with them; its expression goes with them.
fn end_finished(stage: Entity, now: f64, hotspots: &mut Hotspots, actors: &mut Actors) {
    for spot in &mut hotspots.spots {
        if spot.playing.is_empty() {
            continue;
        }
        spot.playing.retain(|played| actors.still_playing(played));
        if !spot.playing.is_empty() {
            continue;
        }
        spot.history.ended = Some(now);
        match spot.phase {
            Phase::Playing | Phase::Held => spot.done(),
            Phase::Ending => spot.phase = Phase::Ready,
            Phase::Ready | Phase::Switching => {}
        }
    }
    if let Some(owner) = hotspots.face
        && hotspots.spots[owner].playing.is_empty()
    {
        hotspots.face = None;
        if let Some((_, mut character)) = actors.character(stage) {
            character.reaction_face(None);
        }
    }
    let gaze_off = hotspots.spots.iter().flat_map(|s| &s.playing).any(|played| !played.gaze);
    if hotspots.gaze_off != gaze_off {
        hotspots.gaze_off = gaze_off;
    }
}

#[cfg(test)]
mod tests {
    use super::super::History;
    use super::super::Pick;
    use super::*;

    #[test]
    fn idle_goes_off_again_every_after_seconds_while_still() {
        assert!(!idle_due(60.0, 59.0, 100.0, None));
        assert!(idle_due(60.0, 60.0, 100.0, None));
        // Went off at 100, still since 40: not again until 160.
        assert!(!idle_due(60.0, 110.0, 150.0, Some(100.0)));
        assert!(idle_due(60.0, 120.0, 160.0, Some(100.0)));
        // Moved at 130 and kept still the 60 s since: due again.
        assert!(idle_due(60.0, 60.0, 190.0, Some(100.0)));
    }

    #[test]
    fn going_off_soon_after_it_could_counts_as_in_a_row() {
        let history = History { fired: Some(10.0), ended: Some(11.0), ..default() };
        assert!(clicks_in_row(&history, 0.0, 13.5));
        assert!(!clicks_in_row(&history, 0.0, 14.5));
        // A cooldown to 15 moves when it could: in a row up to 18.
        assert!(clicks_in_row(&history, 5.0, 17.5));
        assert!(!clicks_in_row(&History::default(), 0.0, 1.0));
    }

    #[test]
    fn clips_for_the_furthest_time_in_a_row_reached_replace_the_others() {
        let clip = |name: &str, from| (name.to_string(), from);
        let pick = Pick {
            actor: None,
            clips: vec![clip("giggle", 1), clip("laugh", 1), clip("annoyed", 3), clip("door", 5)],
            last: None,
        };
        assert_eq!(pick.tier(1), [0, 1]);
        assert_eq!(pick.tier(4), [2]);
        assert_eq!(pick.tier(9), [3]);
        let later = Pick { actor: None, clips: vec![clip("slam", 3)], last: None };
        assert!(later.tier(2).is_empty());
    }

    #[test]
    fn a_random_pick_never_repeats_the_last() {
        let mut seed = 7;
        for last in 0..3 {
            for _ in 0..50 {
                assert_ne!(random_other(&mut seed, 3, Some(last)), last);
            }
        }
        assert_eq!(random_other(&mut seed, 1, Some(0)), 0);
    }

    #[test]
    fn hover_goes_off_once_per_rest() {
        let mut state = Hover::Off;
        assert!(!hover(&mut state, true, 0.0));
        assert!(!hover(&mut state, true, 0.2));
        assert!(hover(&mut state, true, 0.4));
        assert!(!hover(&mut state, true, 5.0));
        assert!(!hover(&mut state, false, 5.1));
        assert!(!hover(&mut state, true, 5.2));
        assert!(hover(&mut state, true, 5.6));
    }
}
