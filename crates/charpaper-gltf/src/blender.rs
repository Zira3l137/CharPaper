//! Blender's space to glTF's, as Blender's glTF exporter converts it with +Y up. Blender is
//! Z-up, glTF Y-up: (x, y, z) in Blender is (x, z, -y) in glTF.

use glam::Mat3;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glam::Vec4;

use crate::clip::Trs;

/// The exporter's `axis_basis_change`: Blender's axes in glTF's.
pub const Y_UP: Mat4 = Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, -1.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::W,
);

/// An armature's bones as Blender has them, in the armature's own order.
#[derive(Debug, Clone)]
pub struct Bones {
    /// Each bone's rest matrix in armature space, Blender's `bone.matrix_local`.
    pub rest: Vec<Mat4>,
    /// Each bone's parent among those exported, which with *Deform bones only* is its nearest
    /// deform ancestor, not always its own parent.
    pub parents: Vec<Option<usize>>,
    /// Whether a bone without an exported parent has a parent at all in Blender.
    pub has_parent: Vec<bool>,
}

impl Bones {
    /// Each bone's local transform in glTF, from its pose in armature space, Blender's
    /// `pose_bone.matrix`, given for every bone.
    pub fn locals(&self, poses: &[Mat4]) -> Vec<Trs> {
        (0..self.rest.len()).map(|bone| self.local(bone, poses)).collect()
    }

    // The exporter's armature_caching and the correction its sampler applies after.
    fn local(&self, bone: usize, poses: &[Mat4]) -> Trs {
        let rest = self.rest[bone];
        let (matrix, correction) = match self.parents[bone] {
            Some(parent) => {
                let rest_local = self.rest[parent].inverse() * rest;
                let matrix = rest_local.inverse() * poses[parent].inverse() * poses[bone];
                (matrix, rest_local)
            }
            None if !self.has_parent[bone] => (rest.inverse() * poses[bone], Y_UP * rest),
            None => (poses[bone], Y_UP),
        };
        let (scale, rotation, translation) = decompose(matrix);
        Trs {
            translation: correction.transform_point3(translation),
            rotation: quat_of(Mat3::from_mat4(correction) * Mat3::from_quat(rotation)),
            scale: scale_of(Mat3::from_mat4(correction) * Mat3::from_diagonal(scale)),
        }
    }
}

/// An object's local transform in glTF, from its matrix relative to its parent in Blender.
pub fn object_local(matrix: Mat4) -> Trs {
    let (scale, rotation, translation) = decompose(matrix);
    Trs {
        translation: Vec3::new(translation.x, translation.z, -translation.y),
        rotation: Quat::from_xyzw(rotation.x, rotation.z, -rotation.y, rotation.w),
        scale: Vec3::new(scale.x, scale.z, scale.y),
    }
}

// As mathutils' Matrix.decompose: column lengths for the scale, the rest for the rotation, and
// a mirrored matrix's scale negated, so its rotation is a real one.
fn decompose(matrix: Mat4) -> (Vec3, Quat, Vec3) {
    let basis = Mat3::from_mat4(matrix);
    let rotation = if basis.determinant() < 0.0 { quat_of(-basis) } else { quat_of(basis) };
    (scale_of(basis), rotation, matrix.w_axis.truncate())
}

fn scale_of(basis: Mat3) -> Vec3 {
    let scale = Vec3::new(basis.x_axis.length(), basis.y_axis.length(), basis.z_axis.length());
    if basis.determinant() < 0.0 { -scale } else { scale }
}

// With w at 0 or above, as mathutils turns a matrix into a quaternion.
fn quat_of(basis: Mat3) -> Quat {
    let unit = |axis: Vec3| axis.try_normalize().unwrap_or(Vec3::ZERO);
    let normalized = Mat3::from_cols(unit(basis.x_axis), unit(basis.y_axis), unit(basis.z_axis));
    let q = Quat::from_mat3(&normalized).normalize();
    if q.w < 0.0 { -q } else { q }
}
