use std::sync::Arc;

use charpaper_anim::Actor;
use charpaper_anim::Clip;
use charpaper_anim::Event;
use charpaper_anim::Interpolation;
use charpaper_anim::Mode;
use charpaper_anim::NodeTracks;
use charpaper_anim::Rig;
use charpaper_anim::Track;
use charpaper_anim::Transform;
use glam::Quat;
use glam::Vec3;

const FADE: f32 = 0.25;

fn rig() -> Rig {
    let rest = |x| Transform { translation: Vec3::new(x, 0.0, 0.0), ..Default::default() };
    Rig {
        nodes: vec![("arm".into(), rest(1.0)), ("leg".into(), rest(2.0))],
        shape_keys: vec![("Smile".into(), 0.2), ("Blink".into(), 0.0)],
    }
}

// Moves `nodes` along y from 0 at its start to `duration` at its end.
fn clip(duration: f32, nodes: &[&str], keys: &[(&str, f32)]) -> Arc<Clip> {
    let ramp = |from: Vec3, to: Vec3| {
        Track::new(vec![0.0, duration], vec![from, to], Interpolation::Linear).unwrap()
    };
    let nodes = nodes
        .iter()
        .map(|name| NodeTracks {
            name: name.to_string(),
            translation: Some(ramp(Vec3::ZERO, Vec3::new(0.0, duration, 0.0))),
            ..Default::default()
        })
        .collect();
    let shape_keys = keys
        .iter()
        .map(|(name, value)| {
            let track = Track::new(vec![0.0], vec![*value], Interpolation::Linear).unwrap();
            (name.to_string(), track)
        })
        .collect();
    Arc::new(Clip { duration, nodes, shape_keys, ..Default::default() })
}

fn y(actor: &Actor, node: usize) -> f32 {
    actor.pose().nodes[node].translation.y
}

fn settle(actor: &mut Actor, timeline: f64) -> Vec<Event> {
    let mut events = Vec::new();
    actor.update(timeline, 1.0, &mut events);
    events
}

#[test]
fn unkeyed_parts_rest() {
    let mut actor = Actor::new(rig(), 1, FADE);
    let arm = actor.add_clip(clip(2.0, &["arm"], &[]));
    actor.play(0, arm, Mode::Hold);
    settle(&mut actor, 0.0);
    assert_eq!(actor.pose().nodes[1].translation, Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(actor.pose().shape_keys, vec![0.2, 0.0]);
}

#[test]
fn timeline_clips_wrap_to_their_length() {
    let mut actor = Actor::new(rig(), 1, FADE);
    let loop_ = actor.add_clip(clip(2.0, &["arm"], &[]));
    actor.play(0, loop_, Mode::Timeline);
    settle(&mut actor, 3.5);
    assert!((y(&actor, 0) - 1.5).abs() < 1e-5);
}

#[test]
fn a_slot_above_owns_only_what_it_keys() {
    let mut actor = Actor::new(rig(), 2, FADE);
    let body = actor.add_clip(clip(4.0, &["arm", "leg"], &[]));
    let wave = actor.add_clip(clip(1.0, &["arm"], &[]));
    actor.play(0, body, Mode::Hold);
    actor.play(1, wave, Mode::Timeline);
    settle(&mut actor, 0.5);
    assert!((y(&actor, 0) - 0.5).abs() < 1e-5);
    assert_eq!(y(&actor, 1), 0.0);
    assert_eq!(actor.pose().nodes[1].translation.x, 0.0);
}

#[test]
fn a_crossfade_averages_shared_parts_and_fades_the_rest() {
    let mut actor = Actor::new(rig(), 1, 1.0);
    let both = actor.add_clip(clip(1.0, &["arm", "leg"], &[]));
    let arm = actor.add_clip(clip(3.0, &["arm"], &[]));
    actor.play(0, both, Mode::Hold);
    settle(&mut actor, 0.0);
    actor.play(0, arm, Mode::Hold);
    actor.update(0.0, 0.5, &mut Vec::new());
    // Both hold their first frame, at y = 0; x shows how much of the rest pose is left.
    assert_eq!(actor.pose().nodes[0].translation.x, 0.0);
    assert!((actor.pose().nodes[1].translation.x - 1.0).abs() < 1e-5);
}

#[test]
fn once_plays_through_then_hands_back() {
    let mut actor = Actor::new(rig(), 2, FADE);
    let idle = actor.add_clip(clip(10.0, &["arm"], &[]));
    let wave = actor.add_clip(clip(0.5, &["arm"], &[]));
    actor.play(0, idle, Mode::Timeline);
    actor.play(1, wave, Mode::Once);
    let mut events = Vec::new();
    actor.update(5.0, 0.25, &mut events);
    assert_eq!(events, vec![Event::Passed { slot: 1, clip: wave, from: None, to: 0.0 }]);
    assert_eq!(y(&actor, 0), 0.0);

    events.clear();
    actor.update(5.25, 0.25, &mut events);
    actor.update(5.5, 0.25, &mut events);
    assert!(events.contains(&Event::Finished { slot: 1, clip: wave }));
    assert_eq!(actor.playing(1), None);

    actor.update(5.75, 0.25, &mut events);
    assert!((y(&actor, 0) - 5.75).abs() < 1e-4);
}

#[test]
fn timeline_clips_report_coming_round() {
    let mut actor = Actor::new(rig(), 1, FADE);
    let loop_ = actor.add_clip(clip(2.0, &["arm"], &[]));
    actor.play(0, loop_, Mode::Timeline);
    let mut events = Vec::new();
    actor.update(1.5, 0.0, &mut events);
    assert!(events.is_empty(), "joining mid-loop passes nothing");
    actor.update(2.5, 1.0, &mut events);
    assert_eq!(events, vec![Event::Passed { slot: 0, clip: loop_, from: Some(1.5), to: 0.5 }]);
}

#[test]
fn playing_the_same_loop_again_changes_nothing() {
    let mut actor = Actor::new(rig(), 1, FADE);
    let idle = actor.add_clip(clip(2.0, &["arm"], &[]));
    actor.play(0, idle, Mode::Timeline);
    settle(&mut actor, 1.0);
    actor.play(0, idle, Mode::Timeline);
    actor.update(1.0, 0.0, &mut Vec::new());
    assert!((y(&actor, 0) - 1.0).abs() < 1e-5);
    assert_eq!(actor.pose().nodes[0].translation.x, 0.0);
}

#[test]
fn shape_keys_mix_by_name_and_rest_at_their_sliders() {
    let mut actor = Actor::new(rig(), 2, FADE);
    let happy = actor.add_clip(clip(1.0, &[], &[("Smile", 1.0), ("Frown", 1.0)]));
    let blink = actor.add_clip(clip(1.0, &[], &[("Blink", 1.0)]));
    actor.play(0, happy, Mode::Timeline);
    actor.play(1, blink, Mode::Hold);
    settle(&mut actor, 0.0);
    assert_eq!(actor.pose().shape_keys, vec![1.0, 1.0, 1.0]);
    assert_eq!(actor.rig().shape_keys[2], ("Frown".to_string(), 0.0), "joined the rig");
    actor.stop(0);
    settle(&mut actor, 0.0);
    assert_eq!(actor.pose().shape_keys, vec![0.2, 1.0, 0.0]);
    assert_eq!(actor.pose().open, vec![1.0, 0.0, 1.0]);
}

#[test]
fn a_fading_shape_key_lets_its_mesh_rest_through() {
    let mut actor = Actor::new(Rig::default(), 1, 1.0);
    let smile = actor.add_clip(clip(1.0, &[], &[("Smile", 1.0)]));
    actor.play(0, smile, Mode::Hold);
    actor.update(0.0, 0.25, &mut Vec::new());
    let (value, open) = (actor.pose().shape_keys[0], actor.pose().open[0]);
    // On a mesh whose Smile slider sits at 0.4.
    assert!((value + 0.4 * open - (0.25 + 0.4 * 0.75)).abs() < 1e-6);
}

#[test]
fn rotations_from_opposite_hemispheres_blend_the_short_way() {
    let rig = Rig { nodes: vec![("arm".into(), Transform::default())], shape_keys: Vec::new() };
    let turn = |q: Quat| {
        let rotation = Some(Track::new(vec![0.0], vec![q], Interpolation::Linear).unwrap());
        let node = NodeTracks { name: "arm".into(), rotation, ..Default::default() };
        Arc::new(Clip { duration: 1.0, nodes: vec![node], ..Default::default() })
    };
    let quarter = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let mut actor = Actor::new(rig, 1, 1.0);
    let a = actor.add_clip(turn(quarter));
    let b = actor.add_clip(turn(-quarter));
    actor.play(0, a, Mode::Hold);
    settle(&mut actor, 0.0);
    actor.play(0, b, Mode::Hold);
    actor.update(0.0, 0.5, &mut Vec::new());
    assert!(actor.pose().nodes[0].rotation.angle_between(quarter) < 1e-4);
}

#[test]
fn no_fade_switches_at_once() {
    let mut actor = Actor::new(rig(), 1, 0.0);
    let first = actor.add_clip(clip(1.0, &["arm"], &[]));
    let second = actor.add_clip(clip(1.0, &["leg"], &[]));
    actor.play(0, first, Mode::Hold);
    actor.update(0.0, 0.0, &mut Vec::new());
    actor.play(0, second, Mode::Hold);
    actor.update(0.0, 0.0, &mut Vec::new());
    assert_eq!(actor.pose().nodes[0].translation.x, 1.0);
    assert_eq!(actor.pose().nodes[1].translation.x, 0.0);
}

#[test]
fn marks_come_due_once_each_including_round_the_loop() {
    let mut actor = Actor::new(rig(), 1, FADE);
    let clip = actor.add_clip(clip(2.0, &["arm"], &[]));
    let passed = |from, to| Event::Passed { slot: 0, clip, from, to };
    assert!(passed(None, 0.5).passes(0.0), "a Once clip's start counts");
    assert!(passed(Some(0.5), 1.0).passes(1.0));
    assert!(!passed(Some(0.5), 1.0).passes(0.5), "already due the frame before");
    assert!(passed(Some(1.5), 0.5).passes(1.8) && passed(Some(1.5), 0.5).passes(0.0));
    assert!(!passed(Some(1.5), 0.5).passes(1.0));
    assert!(!Event::Finished { slot: 0, clip }.passes(0.0));
}

#[test]
fn a_layer_takes_the_slot_of_one_moving_the_same_parts_or_a_free_one() {
    let mut actor = Actor::new(rig(), 4, 0.0);
    let reach = actor.add_clip(clip(1.0, &["arm"], &[]));
    let wave = actor.add_clip(clip(1.0, &["arm"], &[]));
    let blink = actor.add_clip(clip(1.0, &[], &[("blink", 1.0)]));
    let smile = actor.add_clip(clip(1.0, &[], &[("smile", 1.0)]));
    let layers = 1..4;

    assert_eq!(actor.slot_for(layers.clone(), reach), 1);
    actor.play(1, reach, Mode::Once);
    assert_eq!(actor.slot_for(layers.clone(), wave), 1, "both move arm");
    assert_eq!(actor.slot_for(layers.clone(), blink), 2, "moves nothing the first does");
    actor.play(2, blink, Mode::Once);
    actor.play(3, smile, Mode::Once);
    let both = actor.add_clip(clip(1.0, &[], &[("smile", 1.0), ("blink", 1.0)]));
    assert_eq!(actor.slot_for(layers.clone(), both), 2, "the first slot it shares parts with");
    let other = actor.add_clip(clip(1.0, &[], &[("frown", 1.0)]));
    assert_eq!(actor.slot_for(layers, other), 1, "none free: the first");
}

#[test]
fn a_cut_under_a_fading_slot_crossfades_from_the_slot_to_the_new_clip() {
    let mut actor = Actor::new(rig(), 2, 1.0);
    let sitting = actor.add_clip(clip(1.0, &[], &[("Smile", 1.0)]));
    let standing = actor.add_clip(clip(1.0, &[], &[("Smile", 0.0)]));
    let reaction = actor.add_clip(clip(1.0, &[], &[("Smile", 0.5)]));
    actor.play(0, sitting, Mode::Hold);
    actor.play(1, reaction, Mode::Hold);
    settle(&mut actor, 0.0);

    actor.cut(0, standing, Mode::Hold);
    actor.stop(1);
    assert_eq!(actor.playing(0), Some((standing, Mode::Hold)));
    actor.update(0.0, 0.5, &mut Vec::new());
    // Half the reaction over all of standing: nothing of sitting shows.
    assert!((actor.pose().shape_keys[0] - 0.25).abs() < 1e-5);
}
