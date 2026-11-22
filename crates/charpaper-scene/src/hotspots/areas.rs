// Where hotspots are: capsules around bones, found by name and followed as they move.

use bevy::prelude::*;

use super::Spot;
use super::actors::Actors;

// A bone found earlier stays found until its entity goes: the skin or environment it was in
// switched. An environment is searched once for the bones it lacks, not every frame.
pub(super) fn find_bones(
    stage: Entity,
    spot: &mut Spot,
    globals: &Query<&GlobalTransform>,
    actors: &mut Actors,
) {
    let areas = spot.bones.iter_mut().zip(&mut spot.searched).zip(&spot.hotspot.areas);
    for ((bone, searched), area) in areas {
        if bone.is_some_and(|entity| globals.contains(entity)) {
            continue;
        }
        *bone = None;
        if area.actor.is_some() && *searched {
            continue;
        }
        *bone = actors.bone(stage, area.actor.as_deref(), &area.bone);
        *searched = area.actor.is_some() && actors.environment(stage).is_some();
    }
}

// How far along the ray the hotspot is, at the nearest of its capsules.
pub(super) fn distance(spot: &Spot, ray: Ray3d, globals: &Query<&GlobalTransform>) -> Option<f32> {
    let reaches = spot.bones.iter().zip(&spot.hotspot.areas).filter_map(|(bone, area)| {
        let global = globals.get((*bone)?).ok()?;
        let head = global.translation();
        let tail = global.transform_point(Vec3::Y * area.length);
        let affine = global.affine();
        let scale =
            (affine.x_axis.length() + affine.y_axis.length() + affine.z_axis.length()) / 3.0;
        reach(ray.origin, *ray.direction, head, tail, area.radius * scale)
    });
    reaches.min_by(f32::total_cmp)
}

// How far along the ray (origin, unit direction) it enters the capsule around a→b, or None
// if it misses or the capsule is behind it. Found from where the ray passes closest to the
// bone: exact for a sphere and near enough for a capsule to tell what is in front.
fn reach(origin: Vec3, direction: Vec3, a: Vec3, b: Vec3, radius: f32) -> Option<f32> {
    let ab = b - a;
    let r = origin - a;
    let (e, f, c, along) = (ab.dot(ab), ab.dot(r), direction.dot(r), direction.dot(ab));
    let s_for = |t: f32| if e > f32::EPSILON { ((along * t + f) / e).clamp(0.0, 1.0) } else { 0.0 };
    let parallel = e - along * along <= f32::EPSILON * e.max(1.0);
    let t = if parallel { 0.0 } else { ((along * f - c * e) / (e - along * along)).max(0.0) };
    let t = (along * s_for(t) - c).max(0.0);
    let s = s_for(t);
    let gap = (origin + direction * t - (a + ab * s)).length_squared();
    let inside = radius * radius - gap;
    (inside >= 0.0).then(|| (t - inside.sqrt()).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_enters_a_capsule_where_it_meets_its_surface() {
        let (a, b) = (Vec3::ZERO, Vec3::Y);
        let hit = reach(Vec3::new(0.0, 0.5, 5.0), Vec3::NEG_Z, a, b, 0.5);
        assert!(hit.is_some_and(|t| (t - 4.5).abs() < 1e-4));
        // Past the tail, the rounded end still counts.
        assert!(reach(Vec3::new(0.0, 1.3, 5.0), Vec3::NEG_Z, a, b, 0.5).is_some());
        assert_eq!(reach(Vec3::new(2.0, 0.5, 5.0), Vec3::NEG_Z, a, b, 0.5), None);
        assert_eq!(reach(Vec3::new(0.0, 0.5, 5.0), Vec3::Z, a, b, 0.5), None);
    }
}
