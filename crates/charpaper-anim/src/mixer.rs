use std::sync::Arc;

use glam::Quat;
use glam::Vec3;
use glam::Vec4;

use crate::clip::Clip;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }
}

/// A light's strength and linear color as multiples of those it was loaded with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub strength: f32,
    pub color: Vec3,
}

impl Default for Light {
    fn default() -> Self {
        Self { strength: 1.0, color: Vec3::ONE }
    }
}

/// What an actor can move, and where each part rests when nothing keys it: nodes by name with
/// their rest transforms, shape keys by name with their slider values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rig {
    pub nodes: Vec<(String, Transform)>,
    pub shape_keys: Vec<(String, f32)>,
}

/// The actor as the mix leaves it, in the rig's order.
///
/// A shape key's rest can differ from mesh to mesh, so each also says how much of its rest
/// still shows through the clips, from 1 (none keys it) to 0: on a mesh whose slider sits at
/// `rest`, it is `value + rest * open`. `value` already counts the rig's own rest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pose {
    pub nodes: Vec<Transform>,
    pub shape_keys: Vec<f32>,
    pub open: Vec<f32>,
    pub light: Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Shows the timeline's moment, wrapped to the clip's length.
    Timeline,
    /// Plays from its start, then fades out.
    Once,
    /// Holds its first frame.
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClipId(usize);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// A clip moved from `from` to `to`, in its own seconds, and everything keyed in
    /// between is due. `from` is None on a Once clip's first frame, so keys at its very start
    /// count; a Timeline clip that came round has `to` below `from`. A clip that is fading
    /// out sends none.
    Passed { slot: usize, clip: ClipId, from: Option<f32>, to: f32 },
    /// A Once clip reached its end and began to fade out.
    Finished { slot: usize, clip: ClipId },
}

impl Event {
    /// Whether a mark at `at` seconds into the clip came due with this event.
    pub fn passes(&self, at: f32) -> bool {
        let Event::Passed { from, to, .. } = *self else {
            return false;
        };
        match from {
            None => at <= to,
            Some(from) if from <= to => from < at && at <= to,
            Some(from) => at > from || at <= to,
        }
    }
}

struct Bound {
    clip: Arc<Clip>,
    // (rig index, index in the clip) for what the rig has.
    nodes: Vec<(usize, usize)>,
    shape_keys: Vec<(usize, usize)>,
}

struct Entry {
    clip: ClipId,
    mode: Mode,
    time: f32,
    weight: f32,
    fresh: bool,
    leaving: bool,
}

#[derive(Default)]
struct Slot {
    // Oldest first; all but the last are leaving.
    entries: Vec<Entry>,
}

impl Slot {
    fn current(&self) -> Option<&Entry> {
        self.entries.last().filter(|entry| !entry.leaving)
    }

    fn leave(&mut self) {
        self.entries.iter_mut().for_each(|entry| entry.leaving = true);
    }
}

// Weighted sums for one slot, cleared before each.
#[derive(Default)]
struct Sums {
    translation: Vec<(Vec3, f32)>,
    rotation: Vec<(Vec4, f32)>,
    scale: Vec<(Vec3, f32)>,
    shape_keys: Vec<(f32, f32)>,
    strength: (f32, f32),
    color: (Vec3, f32),
}

impl Sums {
    fn clear(&mut self, nodes: usize, shape_keys: usize) {
        reset(&mut self.translation, nodes, (Vec3::ZERO, 0.0));
        reset(&mut self.rotation, nodes, (Vec4::ZERO, 0.0));
        reset(&mut self.scale, nodes, (Vec3::ZERO, 0.0));
        reset(&mut self.shape_keys, shape_keys, (0.0, 0.0));
        self.strength = (0.0, 0.0);
        self.color = (Vec3::ZERO, 0.0);
    }
}

fn reset<T: Clone>(sums: &mut Vec<T>, len: usize, zero: T) {
    sums.clear();
    sums.resize(len, zero);
}

/// One actor's clips, in slots stacked from the bottom up, and the pose they mix to.
pub struct Actor {
    rig: Rig,
    clips: Vec<Bound>,
    slots: Vec<Slot>,
    fade: f32,
    pose: Pose,
    sums: Sums,
}

impl Actor {
    /// `fade` is how long a clip takes to come in or go, in seconds.
    pub fn new(rig: Rig, slots: usize, fade: f32) -> Self {
        let pose = rest(&rig);
        let slots = (0..slots).map(|_| Slot::default()).collect();
        Self { rig, clips: Vec::new(), slots, fade, pose, sums: Sums::default() }
    }

    pub fn rig(&self) -> &Rig {
        &self.rig
    }

    /// Ties the clip's parts to the rig's by name. Nodes the rig lacks are left out. Shape keys
    /// it lacks join it, resting at 0: which shape keys there are depends on what is worn.
    pub fn add_clip(&mut self, clip: Arc<Clip>) -> ClipId {
        let node_at = |name: &str| self.rig.nodes.iter().position(|(own, _)| own == name);
        let nodes = (clip.nodes.iter().enumerate())
            .filter_map(|(at, node)| Some((node_at(&node.name)?, at)))
            .collect();
        let mut shape_keys = Vec::with_capacity(clip.shape_keys.len());
        for (at, (name, _)) in clip.shape_keys.iter().enumerate() {
            let target = match self.rig.shape_keys.iter().position(|(own, _)| own == name) {
                Some(target) => target,
                None => {
                    self.rig.shape_keys.push((name.clone(), 0.0));
                    self.pose.shape_keys.push(0.0);
                    self.pose.open.push(1.0);
                    self.rig.shape_keys.len() - 1
                }
            };
            shape_keys.push((target, at));
        }
        self.clips.push(Bound { clip, nodes, shape_keys });
        ClipId(self.clips.len() - 1)
    }

    pub fn clip(&self, id: ClipId) -> &Clip {
        &self.clips[id.0].clip
    }

    /// Starts `clip` in `slot`, fading out what played there. The clip already playing there
    /// in the same mode carries on, unless the mode is Once: that starts over.
    pub fn play(&mut self, slot: usize, clip: ClipId, mode: Mode) {
        let slot = &mut self.slots[slot];
        let same = slot.current().is_some_and(|e| e.clip == clip && e.mode == mode);
        if same && mode != Mode::Once {
            return;
        }
        slot.leave();
        let weight = if self.fade > 0.0 { 0.0 } else { 1.0 };
        slot.entries.push(Entry { clip, mode, time: 0.0, weight, fresh: true, leaving: false });
    }

    /// Starts `clip` in `slot` at once, dropping what played there without a fade: for a switch
    /// a slot above hides while it fades out itself.
    pub fn cut(&mut self, slot: usize, clip: ClipId, mode: Mode) {
        let slot = &mut self.slots[slot];
        let same = slot.current().is_some_and(|e| e.clip == clip && e.mode == mode);
        if same && mode != Mode::Once {
            return;
        }
        slot.entries.clear();
        slot.entries.push(Entry {
            clip,
            mode,
            time: 0.0,
            weight: 1.0,
            fresh: true,
            leaving: false,
        });
    }

    /// Fades out what plays in `slot`.
    pub fn stop(&mut self, slot: usize) {
        self.slots[slot].leave();
    }

    /// Where a clip laid over others should play among `slots`: where a clip moving any of the
    /// same parts plays, so it takes them over; else in a slot with nothing playing; else in the
    /// first.
    pub fn slot_for(&self, slots: std::ops::Range<usize>, clip: ClipId) -> usize {
        let parts = |id: ClipId| {
            let bound = &self.clips[id.0];
            let nodes = bound.nodes.iter().map(|&(target, _)| (false, target));
            nodes.chain(bound.shape_keys.iter().map(|&(target, _)| (true, target)))
        };
        let shares = |id: ClipId| parts(id).any(|part| parts(clip).any(|own| own == part));
        let current = |slot: usize| self.playing(slot).map(|(id, _)| id);
        (slots.clone().find(|&slot| current(slot).is_some_and(shares)))
            .or_else(|| slots.clone().find(|&slot| current(slot).is_none()))
            .unwrap_or(slots.start)
    }

    pub fn playing(&self, slot: usize) -> Option<(ClipId, Mode)> {
        self.slots[slot].current().map(|entry| (entry.clip, entry.mode))
    }

    pub fn pose(&self) -> &Pose {
        &self.pose
    }

    /// Moves every clip on by `dt` seconds, or to the `timeline`'s moment, and mixes the pose.
    pub fn update(&mut self, timeline: f64, dt: f32, events: &mut Vec<Event>) {
        self.advance(timeline, dt, events);
        self.mix();
    }

    fn advance(&mut self, timeline: f64, dt: f32, events: &mut Vec<Event>) {
        let step = if self.fade > 0.0 { dt / self.fade } else { f32::INFINITY };
        for (at, slot) in self.slots.iter_mut().enumerate() {
            for entry in &mut slot.entries {
                let duration = self.clips[entry.clip.0].clip.duration;
                let now = match entry.mode {
                    Mode::Timeline if duration > 0.0 => {
                        timeline.rem_euclid(f64::from(duration)) as f32
                    }
                    Mode::Once if !entry.fresh => (entry.time + dt).min(duration),
                    Mode::Timeline | Mode::Once | Mode::Hold => 0.0,
                };
                let clip = entry.clip;
                if !entry.leaving {
                    match entry.mode {
                        Mode::Timeline if !entry.fresh && now != entry.time => {
                            events.push(Event::Passed {
                                slot: at,
                                clip,
                                from: Some(entry.time),
                                to: now,
                            });
                        }
                        Mode::Once => {
                            let from = (!entry.fresh).then_some(entry.time);
                            if from != Some(now) {
                                events.push(Event::Passed { slot: at, clip, from, to: now });
                            }
                            if now >= duration {
                                events.push(Event::Finished { slot: at, clip });
                                entry.leaving = true;
                            }
                        }
                        Mode::Timeline | Mode::Hold => {}
                    }
                }
                entry.weight = if entry.leaving {
                    (entry.weight - step).max(0.0)
                } else {
                    (entry.weight + step).min(1.0)
                };
                entry.time = now;
                entry.fresh = false;
            }
            slot.entries.retain(|entry| !(entry.leaving && entry.weight <= 0.0));
        }
    }

    fn mix(&mut self) {
        let (nodes, keys) = (self.rig.nodes.len(), self.rig.shape_keys.len());
        let pose = &mut self.pose;
        for (node, (_, rest)) in pose.nodes.iter_mut().zip(&self.rig.nodes) {
            *node = *rest;
        }
        for (value, (_, rest)) in pose.shape_keys.iter_mut().zip(&self.rig.shape_keys) {
            *value = *rest;
        }
        pose.open.fill(1.0);
        pose.light = Light::default();
        for slot in &self.slots {
            let sums = &mut self.sums;
            sums.clear(nodes, keys);
            for entry in slot.entries.iter().filter(|entry| entry.weight > 0.0) {
                let (bound, w, t) = (&self.clips[entry.clip.0], entry.weight, entry.time);
                for &(target, at) in &bound.nodes {
                    let node = &bound.clip.nodes[at];
                    if let Some(track) = &node.translation {
                        add(&mut sums.translation[target], track.sample(t), w);
                    }
                    if let Some(track) = &node.rotation {
                        // Same hemisphere as the pose below, or the average would cancel out.
                        let q = track.sample(t);
                        let q = if q.dot(pose.nodes[target].rotation) < 0.0 { -q } else { q };
                        add(&mut sums.rotation[target], Vec4::from(q), w);
                    }
                    if let Some(track) = &node.scale {
                        add(&mut sums.scale[target], track.sample(t), w);
                    }
                }
                for &(target, at) in &bound.shape_keys {
                    add(&mut sums.shape_keys[target], bound.clip.shape_keys[at].1.sample(t), w);
                }
                if let Some(track) = &bound.clip.light.strength {
                    add(&mut sums.strength, track.sample(t), w);
                }
                if let Some(track) = &bound.clip.light.color {
                    add(&mut sums.color, track.sample(t), w);
                }
            }

            for (node, sums) in pose
                .nodes
                .iter_mut()
                .zip(sums.translation.iter().zip(&sums.rotation).zip(&sums.scale))
            {
                let ((&translation, &rotation), &scale) = sums;
                lay(&mut node.translation, translation, Vec3::lerp);
                lay(&mut node.scale, scale, Vec3::lerp);
                let (sum, weight) = rotation;
                if weight > 0.0 && sum.length_squared() > f32::EPSILON {
                    let average = Quat::from_vec4(sum).normalize();
                    node.rotation = node.rotation.slerp(average, weight.min(1.0));
                }
            }
            for ((value, open), &sum) in
                pose.shape_keys.iter_mut().zip(&mut pose.open).zip(&sums.shape_keys)
            {
                lay(value, sum, |a, b, t| a + (b - a) * t);
                *open *= 1.0 - sum.1.min(1.0);
            }
            lay(&mut pose.light.strength, sums.strength, |a, b, t| a + (b - a) * t);
            lay(&mut pose.light.color, sums.color, Vec3::lerp);
        }
    }
}

fn rest(rig: &Rig) -> Pose {
    Pose {
        nodes: rig.nodes.iter().map(|(_, rest)| *rest).collect(),
        shape_keys: rig.shape_keys.iter().map(|(_, rest)| *rest).collect(),
        open: vec![1.0; rig.shape_keys.len()],
        light: Light::default(),
    }
}

fn add<T: std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T> + Copy>(
    sum: &mut (T, f32),
    value: T,
    weight: f32,
) {
    sum.0 = sum.0 + value * weight;
    sum.1 += weight;
}

// Lays a slot's weighted average over the value below, by as much as its weights add up to.
fn lay<T: std::ops::Mul<f32, Output = T> + Copy>(
    below: &mut T,
    (sum, weight): (T, f32),
    lerp: impl Fn(T, T, f32) -> T,
) {
    if weight > 0.0 {
        *below = lerp(*below, sum * (1.0 / weight), weight.min(1.0));
    }
}
