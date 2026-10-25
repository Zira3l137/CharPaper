use std::f32::consts::PI;
use std::f32::consts::TAU;

use glam::Quat;

/// Blender's Rotational Difference: the angle of the turn from one rotation to the other, from
/// 0 to π. Both rotations in the same space; turning both alike, or each about its own axes
/// alike, leaves it unchanged.
pub fn rotation_difference(a: Quat, b: Quat) -> f32 {
    let between = (a.inverse() * b).normalize();
    let angle = (2.0 * between.w.clamp(-1.0, 1.0).acos()).abs();
    if angle > PI { TAU - angle } else { angle }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use glam::Vec3;

    use super::*;

    #[test]
    fn same_rotation_is_zero() {
        let q = Quat::from_euler(glam::EulerRot::XYZ, 0.3, -1.1, 2.0);
        assert!(rotation_difference(q, q).abs() < 1e-3);
    }

    #[test]
    fn is_the_angle_between() {
        let a = Quat::from_rotation_y(0.4);
        let b = a * Quat::from_axis_angle(Vec3::new(1.0, 2.0, -0.5).normalize(), 1.2);
        assert!((rotation_difference(a, b) - 1.2).abs() < 1e-4);
        assert!((rotation_difference(b, a) - 1.2).abs() < 1e-4);
    }

    #[test]
    fn folds_past_half_a_turn() {
        let a = Quat::IDENTITY;
        let b = Quat::from_rotation_x(3.0 * FRAC_PI_2);
        assert!((rotation_difference(a, b) - FRAC_PI_2).abs() < 1e-4);
    }

    // The app's bones differ from Blender's by the change to glTF's axes, and maybe by how the
    // exporter turns each bone's own axes; neither may change the angle.
    #[test]
    fn ignores_a_change_of_axes() {
        let a = Quat::from_euler(glam::EulerRot::XYZ, 0.2, 0.9, -0.4);
        let b = Quat::from_euler(glam::EulerRot::XYZ, -1.0, 0.1, 0.7);
        let world = Quat::from_rotation_x(-FRAC_PI_2);
        let own = Quat::from_euler(glam::EulerRot::XYZ, 0.5, -0.3, 1.4);
        let expected = rotation_difference(a, b);
        let changed = rotation_difference(world * a * own, world * b * own);
        assert!((expected - changed).abs() < 1e-4);
    }
}
