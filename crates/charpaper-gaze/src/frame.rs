use glam::Quat;
use glam::Vec2;
use glam::Vec3;

/// The character's own left, up and front in the rest pose, in the armature's space. Angles
/// are measured against it, in radians: yaw, in `x`, turns towards the left, and pitch, in
/// `y`, upwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub side: Vec3,
    pub up: Vec3,
    pub front: Vec3,
}

// Blender's front view as glTF has it: +Y up, facing +Z.
impl Default for Frame {
    fn default() -> Self {
        Self { side: Vec3::X, up: Vec3::Y, front: Vec3::Z }
    }
}

impl Frame {
    /// Facing `front`, made level against `up`. None when `front` points along `up`.
    pub fn facing(front: Vec3, up: Vec3) -> Option<Self> {
        let up = up.try_normalize()?;
        let front = (front - up * front.dot(up)).try_normalize()?;
        Some(Self { side: up.cross(front), up, front })
    }

    pub fn angles(&self, direction: Vec3) -> Vec2 {
        let d = direction.normalize_or(self.front);
        Vec2::new(d.dot(self.side).atan2(d.dot(self.front)), d.dot(self.up).clamp(-1.0, 1.0).asin())
    }

    /// The turn that points `front` along the angles, without rolling.
    pub fn toward(&self, angles: Vec2) -> Quat {
        Quat::from_axis_angle(self.up, angles.x) * Quat::from_axis_angle(self.side, -angles.y)
    }

    pub fn direction(&self, angles: Vec2) -> Vec3 {
        self.toward(angles) * self.front
    }

    /// The turn that takes what points along `from` to point along `to`. Whatever roll it has
    /// goes along unchanged.
    pub fn turn(&self, from: Vec2, to: Vec2) -> Quat {
        (self.toward(to) * self.toward(from).inverse()).normalize()
    }
}

// Eyes this close to straight above the pivot, as a share of their distance from it, don't say
// which way the character faces.
const LEAST_AHEAD: f32 = 0.05;

/// Which way the character faces, from where the eyes sit in the rest pose: level, square to
/// the line between them, on the side of `pivot`, the head's joint, that they are on. With one
/// eye,
/// straight from the pivot to it. None when the bones don't say.
pub fn front_from_eyes(pivot: Vec3, eyes: &[Vec3], up: Vec3) -> Option<Vec3> {
    let (first, last) = (*eyes.first()?, *eyes.last()?);
    let level = |v: Vec3| v - up * v.dot(up);
    let middle = eyes.iter().sum::<Vec3>() / eyes.len() as f32;
    let least = (middle - pivot).length() * LEAST_AHEAD;
    let ahead = level(middle - pivot);
    // The line between two eyes says it more exactly than the pivot, which only picks the side.
    match up.cross(level(last - first)).try_normalize() {
        Some(square) if ahead.dot(square).abs() > least => {
            Some(square * ahead.dot(square).signum())
        }
        Some(_) => None,
        None if ahead.length() > least => Some(ahead.normalize()),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn angles_and_directions_agree() {
        let frame = Frame::facing(Vec3::new(0.3, 0.4, -1.0), Vec3::Y).unwrap();
        for angles in [Vec2::ZERO, Vec2::new(0.7, 0.2), Vec2::new(-2.0, -0.9), Vec2::new(3.0, 1.2)]
        {
            assert!((frame.angles(frame.direction(angles)) - angles).length() < 1e-4);
        }
    }

    #[test]
    fn positive_yaw_is_to_the_left_and_positive_pitch_is_up() {
        let frame = Frame::default();
        assert!(close(frame.direction(Vec2::new(FRAC_PI_2, 0.0)), Vec3::X));
        assert!(close(frame.direction(Vec2::new(0.0, FRAC_PI_2)), Vec3::Y));
    }

    #[test]
    fn a_turn_lands_on_its_target_without_rolling() {
        let frame = Frame::default();
        let (from, to) = (Vec2::new(0.3, -0.2), Vec2::new(-0.9, 0.5));
        let turn = frame.turn(from, to);
        assert!(close(turn * frame.direction(from), frame.direction(to)));
        // The side axis stays level: no roll came in.
        let side = turn * (frame.toward(from) * frame.side);
        assert!(side.dot(frame.up).abs() < 1e-4);
    }

    #[test]
    fn a_turn_in_two_steps_is_the_turn_in_one() {
        let frame = Frame::default();
        let (clip, neck, head) = (Vec2::new(0.1, 0.0), Vec2::new(0.4, 0.2), Vec2::new(0.9, 0.5));
        let stepped = frame.turn(neck, head) * frame.turn(clip, neck);
        assert!(stepped.angle_between(frame.turn(clip, head)) < 1e-4);
    }

    #[test]
    fn front_comes_from_the_eyes() {
        let pivot = Vec3::new(0.0, 1.5, 0.0);
        let eyes = [Vec3::new(0.03, 1.6, 0.08), Vec3::new(-0.03, 1.6, 0.08)];
        assert!(close(front_from_eyes(pivot, &eyes, Vec3::Y).unwrap(), Vec3::Z));
        // Turned a quarter, with one eye bone a little off: the line between the eyes decides.
        let eyes = [Vec3::new(0.08, 1.6, -0.03), Vec3::new(0.08, 1.61, 0.03)];
        assert!(close(front_from_eyes(pivot, &eyes, Vec3::Y).unwrap(), Vec3::X));
        let one = [Vec3::new(0.0, 1.6, -0.08)];
        assert!(close(front_from_eyes(pivot, &one, Vec3::Y).unwrap(), Vec3::NEG_Z));
    }

    #[test]
    fn eyes_straight_above_the_pivot_say_nothing() {
        let pivot = Vec3::new(0.0, 1.5, 0.0);
        let eyes = [Vec3::new(0.03, 1.6, 0.0), Vec3::new(-0.03, 1.6, 0.0)];
        assert_eq!(front_from_eyes(pivot, &eyes, Vec3::Y), None);
        assert_eq!(front_from_eyes(pivot, &[], Vec3::Y), None);
    }
}
