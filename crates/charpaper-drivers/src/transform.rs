// The Transform Channel and Distance variables, worked out as Blender works them out
// (dvar_eval_transChan and dvar_eval_locDiff in fcurve_driver.cc, with the matrix decompositions
// of math_rotation_c.cc), on matrices in Blender's own axes.
//
// The app holds the bones in glTF's axes, so a matrix is first turned back into Blender's.
// glTF's axes are Blender's with +Y up: (x, y, z) becomes (x, z, -y). Besides, the glTF
// exporter may give each bone axes of its own; rather than assume how, `Axes` works it out
// from one rotation the exporter writes down, the bone's rest rotation as Blender has it.

use std::f32::consts::FRAC_PI_2;

use glam::Affine3A;
use glam::Mat3A;
use glam::Quat;
use serde::Deserialize;

/// Which number a Transform Channel variable reads.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    LocX,
    LocY,
    LocZ,
    RotW,
    RotX,
    RotY,
    RotZ,
    ScaleX,
    ScaleY,
    ScaleZ,
    ScaleAvg,
}

/// How a rotation becomes numbers. An Euler order names the axes in the order they turn.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RotationMode {
    Xyz,
    Xzy,
    Yxz,
    Yzx,
    Zxy,
    Zyx,
    Quaternion,
    SwingTwistX,
    SwingTwistY,
    SwingTwistZ,
}

/// World space is the bone in the scene; local space is the bone's pose against its rest pose,
/// along its own rest axes, which Blender's Transform Space and Local Space both are while no
/// constraint moves the bone.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Space {
    World,
    Local,
}

/// How one bone's matrices in the app relate to Blender's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axes {
    // The turn from Blender's axes of the bone to the app's.
    own: Quat,
}

impl Axes {
    /// From the bone's rest rotation as Blender has it in the world, in Blender's axes, and as
    /// the app has it in the armature's space.
    pub fn new(blender_rest: Quat, app_rest: Quat) -> Self {
        let blender_in_gltf = to_blender().inverse() * blender_rest;
        Self { own: (blender_in_gltf.inverse() * app_rest).normalize() }
    }

    /// The bone's matrix in the world, as Blender would have it, from the app's matrix in the
    /// armature's space.
    pub fn world(&self, app: Affine3A) -> Affine3A {
        let to_blender = to_blender();
        Affine3A {
            matrix3: Mat3A::from_quat(to_blender)
                * app.matrix3
                * Mat3A::from_quat(self.own.inverse()),
            translation: to_blender * app.translation,
        }
    }

    /// The bone's pose against its rest, as Blender would have it, from the app's.
    pub fn local(&self, app: Affine3A) -> Affine3A {
        Affine3A {
            matrix3: Mat3A::from_quat(self.own)
                * app.matrix3
                * Mat3A::from_quat(self.own.inverse()),
            translation: self.own * app.translation,
        }
    }
}

// Turns glTF's axes into Blender's, undoing (x, y, z) becoming (x, z, -y).
fn to_blender() -> Quat {
    Quat::from_rotation_x(FRAC_PI_2)
}

/// A Transform Channel variable's value from a matrix in Blender's axes.
pub fn channel(matrix: Affine3A, channel: Channel, mode: RotationMode) -> f32 {
    let column = |i: usize| matrix.matrix3.col(i);
    match channel {
        Channel::LocX => matrix.translation.x,
        Channel::LocY => matrix.translation.y,
        Channel::LocZ => matrix.translation.z,
        Channel::ScaleX => column(0).length(),
        Channel::ScaleY => column(1).length(),
        Channel::ScaleZ => column(2).length(),
        Channel::ScaleAvg => matrix.matrix3.determinant().cbrt(),
        Channel::RotW => rotation(matrix, 0, mode),
        Channel::RotX => rotation(matrix, 1, mode),
        Channel::RotY => rotation(matrix, 2, mode),
        Channel::RotZ => rotation(matrix, 3, mode),
    }
}

// As BKE_driver_target_matrix_to_rot_channels fills its four numbers: W first, then X, Y, Z.
fn rotation(matrix: Affine3A, index: usize, mode: RotationMode) -> f32 {
    let normalized = Mat3A::from_cols(
        matrix.matrix3.x_axis.normalize_or_zero(),
        matrix.matrix3.y_axis.normalize_or_zero(),
        matrix.matrix3.z_axis.normalize_or_zero(),
    );
    let order = match mode {
        RotationMode::Xyz => Some(([0, 1, 2], false)),
        RotationMode::Xzy => Some(([0, 2, 1], true)),
        RotationMode::Yxz => Some(([1, 0, 2], true)),
        RotationMode::Yzx => Some(([1, 2, 0], false)),
        RotationMode::Zxy => Some(([2, 0, 1], false)),
        RotationMode::Zyx => Some(([2, 1, 0], true)),
        _ => None,
    };
    if let Some((axes, parity)) = order {
        // An Euler mode leaves W at 0.
        return if index == 0 { 0.0 } else { euler(&normalized, axes, parity)[index - 1] };
    }
    let quat = canonical(Quat::from_mat3a(&normalized));
    let wxyz = |q: Quat| [q.w, q.x, q.y, q.z];
    let axis = match mode {
        RotationMode::SwingTwistX => 0,
        RotationMode::SwingTwistY => 1,
        RotationMode::SwingTwistZ => 2,
        _ => return wxyz(quat)[index],
    };
    // quat_split_swing_and_twist, then quaternion_to_angles on the swing.
    let half_twist = wxyz(quat)[axis + 1].atan2(quat.w);
    if index == axis + 1 {
        return 2.0 * half_twist;
    }
    let mut untwist = [half_twist.cos(), 0.0, 0.0, 0.0];
    untwist[axis + 1] = -half_twist.sin();
    let untwist = Quat::from_xyzw(untwist[1], untwist[2], untwist[3], untwist[0]);
    let swing = wxyz(quat * untwist)[index].clamp(-1.0, 1.0);
    if index == 0 { 2.0 * swing.acos() } else { 2.0 * swing.asin() }
}

// Blender keeps W at or above 0.
fn canonical(q: Quat) -> Quat {
    if q.w < 0.0 { -q } else { q }
}

// mat3_normalized_to_eulo2 and the choice mat3_normalized_to_eulO makes between its two
// answers. Blender indexes a matrix by column, then row: m[i][j] is column i, row j.
fn euler(m: &Mat3A, [i, j, k]: [usize; 3], parity: bool) -> [f32; 3] {
    const HYPOT_EPSILON: f32 = 0.0000375;
    let at = |column: usize, row: usize| m.col(column)[row];
    let cy = at(i, i).hypot(at(i, j));
    let mut first = [0.0; 3];
    let mut second = [0.0; 3];
    if cy > HYPOT_EPSILON {
        first[i] = at(j, k).atan2(at(k, k));
        first[j] = (-at(i, k)).atan2(cy);
        first[k] = at(i, j).atan2(at(i, i));
        second[i] = (-at(j, k)).atan2(-at(k, k));
        second[j] = (-at(i, k)).atan2(-cy);
        second[k] = (-at(i, j)).atan2(-at(i, i));
    } else {
        first[i] = (-at(k, j)).atan2(at(j, j));
        first[j] = (-at(i, k)).atan2(cy);
        second = first;
    }
    if parity {
        first = first.map(|a| -a);
        second = second.map(|a| -a);
    }
    let size = |e: &[f32; 3]| e.iter().map(|a| a.abs()).sum::<f32>();
    if size(&first) > size(&second) { second } else { first }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    fn turn(axis: usize, angle: f32) -> Quat {
        match axis {
            0 => Quat::from_rotation_x(angle),
            1 => Quat::from_rotation_y(angle),
            _ => Quat::from_rotation_z(angle),
        }
    }

    // Blender's XYZ turns about X first, then Y, then Z.
    #[test]
    fn euler_orders_round_trip() {
        let angles = [0.3_f32, -0.7, 1.1];
        let orders = [
            (RotationMode::Xyz, [0, 1, 2]),
            (RotationMode::Xzy, [0, 2, 1]),
            (RotationMode::Yxz, [1, 0, 2]),
            (RotationMode::Yzx, [1, 2, 0]),
            (RotationMode::Zxy, [2, 0, 1]),
            (RotationMode::Zyx, [2, 1, 0]),
        ];
        for (mode, sequence) in orders {
            let q = sequence.iter().fold(Quat::IDENTITY, |q, &axis| turn(axis, angles[axis]) * q);
            let matrix = Affine3A::from_quat(q);
            for (axis, channel) in
                [Channel::RotX, Channel::RotY, Channel::RotZ].into_iter().enumerate()
            {
                let value = super::channel(matrix, channel, mode);
                assert!((value - angles[axis]).abs() < 1e-4, "{mode:?} axis {axis}: {value}");
            }
            assert_eq!(super::channel(matrix, Channel::RotW, mode), 0.0);
        }
    }

    #[test]
    fn quaternion_is_canonical() {
        let q = Quat::from_rotation_y(0.8);
        let matrix = Affine3A::from_quat(-q);
        assert!((channel(matrix, Channel::RotW, RotationMode::Quaternion) - q.w).abs() < 1e-5);
        assert!((channel(matrix, Channel::RotY, RotationMode::Quaternion) - q.y).abs() < 1e-5);
    }

    #[test]
    fn swing_and_twist() {
        let twist = Quat::from_rotation_y(0.9);
        let swing = Quat::from_rotation_x(0.4);
        let matrix = Affine3A::from_quat(swing * twist);
        let mode = RotationMode::SwingTwistY;
        assert!((channel(matrix, Channel::RotY, mode) - 0.9).abs() < 1e-4);
        assert!((channel(matrix, Channel::RotX, mode) - 0.4).abs() < 1e-4);
        assert!(channel(matrix, Channel::RotZ, mode).abs() < 1e-4);
        assert!((channel(matrix, Channel::RotW, mode) - 0.4).abs() < 1e-4);
    }

    #[test]
    fn location_and_scale() {
        let matrix = Affine3A::from_scale_rotation_translation(
            Vec3::new(2.0, 3.0, 4.0),
            Quat::from_rotation_z(0.5),
            Vec3::new(1.0, -2.0, 0.5),
        );
        let mode = RotationMode::Xyz;
        assert_eq!(channel(matrix, Channel::LocY, mode), -2.0);
        assert!((channel(matrix, Channel::ScaleY, mode) - 3.0).abs() < 1e-5);
        assert!((channel(matrix, Channel::ScaleAvg, mode) - 24f32.cbrt()).abs() < 1e-4);
        assert!((channel(matrix, Channel::RotZ, mode) - 0.5).abs() < 1e-5);
    }

    // The app's bone is Blender's in glTF's axes, along axes of its own (`own`), whatever the
    // exporter chose; both conversions must give Blender's matrices back.
    #[test]
    fn axes_give_blenders_matrices() {
        let to_gltf = to_blender().inverse();
        let own = Quat::from_euler(glam::EulerRot::XYZ, FRAC_PI_2, 0.0, 0.0);
        let app = |blender: Affine3A| Affine3A {
            matrix3: Mat3A::from_quat(to_gltf) * blender.matrix3 * Mat3A::from_quat(own),
            translation: to_gltf * blender.translation,
        };
        let rest = Affine3A::from_rotation_translation(
            Quat::from_euler(glam::EulerRot::XYZ, 0.2, 0.4, -0.3),
            Vec3::new(0.1, 0.2, 1.4),
        );
        let pose = Affine3A::from_rotation_translation(
            Quat::from_rotation_x(0.6),
            Vec3::new(0.0, 0.05, 0.0),
        );
        let posed = rest * pose;
        let axes = Axes::new(Quat::from_mat3a(&rest.matrix3), Quat::from_mat3a(&app(rest).matrix3));

        assert!(axes.world(app(posed)).abs_diff_eq(posed, 1e-5));
        let app_pose = app(rest).inverse() * app(posed);
        assert!(axes.local(app_pose).abs_diff_eq(pose, 1e-5));
    }

    #[test]
    fn gltf_axes() {
        // Blender's +Z up is glTF's +Y; Blender's -Y front is glTF's +Z.
        let to_gltf = to_blender().inverse();
        assert!((to_gltf * Vec3::Z - Vec3::Y).length() < 1e-6);
        assert!((to_gltf * Vec3::NEG_Y - Vec3::Z).length() < 1e-6);
    }
}
