//! Secondary motion on bone chains: hair, ribbons, skirts, tails.
//!
//! Each joint's tail is a point mass, kept at the bone's length from its head and pulled toward
//! where the animation puts it. The animation stays in charge: it is the pose the springs follow,
//! so a keyed swing of a ponytail swings it, and the springs add the follow-through. Tails are
//! pushed out of capsules, each around a bone of the body, by the joint's own radius.
//!
//! It steps at a fixed rate whatever the frame rate, which keeps it the same at 30 and at 144
//! frames a second, and shows a blend of the last two steps in between, so it moves smoothly.
//! Joints are worked out from the root down: a joint swung by the springs carries its children
//! with it, as a bone does.

use glam::Affine3A;
use glam::Quat;
use glam::Vec3;

use crate::mixer::Transform;

/// The rate the springs step at, in steps a second.
pub const RATE: f32 = 60.0;
const STEP: f32 = 1.0 / RATE;
// Past this, a slow frame is let go of rather than caught up with: a wallpaper nobody looks at
// may draw a frame a second, and a quarter of a second of motion is plenty to catch up on.
const MOST_BEHIND: f32 = 0.25;

/// How a chain moves. All in `0..=1` but gravity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// How much of the way back to the animated pose a tail goes each step: 0 hangs loose,
    /// 1 follows the animation exactly.
    pub stiffness: f32,
    /// How much of its speed a tail loses each step: 0 swings on, 1 stops dead.
    pub drag: f32,
    /// Gravity, as a multiple of Earth's.
    pub gravity: f32,
}

/// One bone of a chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    /// From its head to its tail, in its own units. Only the last joint's is used: the others
    /// end where the next begins.
    pub length: f32,
    /// How far its tail keeps from colliders, in its own units.
    pub radius: f32,
}

/// A capsule a chain stays out of, in world space: the segment from `a` to `b`, grown by `radius`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capsule {
    pub a: Vec3,
    pub b: Vec3,
    pub radius: f32,
}

impl Capsule {
    /// A capsule around a bone: from its head along its +Y, as Blender's bones point, scaled
    /// with it.
    pub fn around(bone: Affine3A, length: f32, radius: f32) -> Self {
        let a = bone.translation.into();
        let b = bone.transform_point3(Vec3::Y * length);
        let scale = bone.matrix3.x_axis.length();
        Self { a, b, radius: radius * scale }
    }

    // `point` moved out of it, grown by `margin`, along the shortest way.
    fn push(&self, point: Vec3, margin: f32) -> Vec3 {
        let axis = self.b - self.a;
        let along = (point - self.a).dot(axis) / axis.length_squared().max(f32::EPSILON);
        let closest = self.a + axis * along.clamp(0.0, 1.0);
        let away = point - closest;
        let reach = self.radius + margin;
        if away.length_squared() >= reach * reach {
            return point;
        }
        // Dead on the axis: out sideways, any way will do.
        let out = away.try_normalize().unwrap_or_else(|| axis.any_orthonormal_vector());
        closest + out * reach
    }
}

/// What pushes on every chain besides its springs, in world space.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Forces {
    /// Earth's gravity, down. Each chain scales it by its own `gravity`.
    pub gravity: Vec3,
}

/// One chain of joints, from a root to a leaf, each the child of the one before.
#[derive(Debug, Clone)]
pub struct Chain {
    joints: Vec<Joint>,
    settings: Settings,
    state: Option<State>,
}

// Each joint's tail now and a step before: the difference is its speed, and the time since
// the last step blends between them for show.
#[derive(Debug, Clone)]
struct State {
    tails: Vec<Vec3>,
    before: Vec<Vec3>,
    // Time not yet stepped through, under a step.
    spare: f32,
}

impl Chain {
    pub fn new(joints: Vec<Joint>, settings: Settings) -> Self {
        Self { joints, settings, state: None }
    }

    pub fn settings(&self) -> Settings {
        self.settings
    }

    /// Starts again from the animated pose, for a jump the springs shouldn't swing through:
    /// another skin, a cut to another animation.
    pub fn reset(&mut self) {
        self.state = None;
    }

    /// Moves the chain on by `dt` seconds. `parent` is where the first joint's parent is in the
    /// world; `locals` are the joints' animated transforms relative to their parents, in order,
    /// whose rotations are turned to the springs' pose.
    pub fn update(
        &mut self,
        parent: Affine3A,
        locals: &mut [Transform],
        colliders: &[Capsule],
        forces: Forces,
        dt: f32,
    ) {
        assert_eq!(locals.len(), self.joints.len(), "a transform for every joint");
        let state = self.state.get_or_insert_with(|| {
            let tails = targets(parent, locals, &self.joints);
            State { before: tails.clone(), tails, spare: 0.0 }
        });
        let pull = forces.gravity * self.settings.gravity * STEP * STEP;
        state.spare += dt.clamp(0.0, MOST_BEHIND);
        while state.spare >= STEP {
            state.spare -= STEP;
            let older = std::mem::replace(&mut state.before, state.tails.clone());
            let tails = (state.tails.as_mut_slice(), older.as_slice());
            step(parent, locals, &self.joints, self.settings, tails, pull, colliders);
        }
        let blend = state.spare / STEP;
        let tails: Vec<Vec3> =
            state.before.iter().zip(&state.tails).map(|(from, to)| from.lerp(*to, blend)).collect();
        pose(parent, locals, &self.joints, &tails);
    }
}

// Where each tail is in the animated pose.
fn targets(parent: Affine3A, locals: &[Transform], joints: &[Joint]) -> Vec<Vec3> {
    let mut parent = parent;
    (0..joints.len())
        .map(|at| {
            let world = parent * affine(locals[at]);
            parent = world;
            world.transform_point3(tail_offset(locals, joints, at))
        })
        .collect()
}

// One step: each tail keeps its speed less its drag, goes back toward the animated pose by the
// stiffness, falls, then is kept at its length from its head and outside the colliders.
fn step(
    parent: Affine3A,
    locals: &[Transform],
    joints: &[Joint],
    settings: Settings,
    (tails, previous): (&mut [Vec3], &[Vec3]),
    pull: Vec3,
    colliders: &[Capsule],
) {
    let mut parent = parent;
    for at in 0..joints.len() {
        let world = parent * affine(locals[at]);
        let head: Vec3 = world.translation.into();
        let target = world.transform_point3(tail_offset(locals, joints, at));
        let length = (target - head).length();
        let tail = tails[at];
        let mut next = tail
            + (tail - previous[at]) * (1.0 - settings.drag)
            + (target - tail) * settings.stiffness
            + pull;
        next = keep_length(head, next, target, length);
        // Pushed out, then back to its length, which can take it a little in again: twice
        // leaves it a hair's breadth in at most.
        let margin = joints[at].radius * world.matrix3.x_axis.length();
        for _ in 0..2 {
            for collider in colliders {
                next = collider.push(next, margin);
            }
            next = keep_length(head, next, target, length);
        }
        tails[at] = next;
        parent = swung(world, head, target, next);
    }
}

// Turns the joints' rotations to point at the tails, from the root down.
fn pose(parent: Affine3A, locals: &mut [Transform], joints: &[Joint], tails: &[Vec3]) {
    let mut parent = parent;
    for at in 0..joints.len() {
        let world = parent * affine(locals[at]);
        let head: Vec3 = world.translation.into();
        let target = world.transform_point3(tail_offset(locals, joints, at));
        let world = swung(world, head, target, tails[at]);
        let (_, rotation, _) = (parent.inverse() * world).to_scale_rotation_translation();
        locals[at].rotation = rotation.normalize();
        parent = world;
    }
}

// The joint's world transform, turned about its head so that its tail is at `tail`.
fn swung(world: Affine3A, head: Vec3, target: Vec3, tail: Vec3) -> Affine3A {
    let (Some(from), Some(to)) = ((target - head).try_normalize(), (tail - head).try_normalize())
    else {
        return world;
    };
    let turn = Quat::from_rotation_arc(from, to);
    Affine3A::from_translation(head)
        * Affine3A::from_quat(turn)
        * Affine3A::from_translation(-head)
        * world
}

fn keep_length(head: Vec3, tail: Vec3, target: Vec3, length: f32) -> Vec3 {
    let along = (tail - head).try_normalize().or((target - head).try_normalize());
    along.map_or(target, |along| head + along * length)
}

// Where its tail is in its own space: the next joint's head, or for the last, along its +Y.
fn tail_offset(locals: &[Transform], joints: &[Joint], at: usize) -> Vec3 {
    match locals.get(at + 1) {
        Some(next) => next.translation,
        None => Vec3::Y * joints[at].length,
    }
}

fn affine(transform: Transform) -> Affine3A {
    Affine3A::from_scale_rotation_translation(
        transform.scale,
        transform.rotation,
        transform.translation,
    )
}
