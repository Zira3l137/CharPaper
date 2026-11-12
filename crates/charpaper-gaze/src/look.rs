use std::f32::consts::PI;

use glam::Vec2;
use glam::Vec3;

/// How the picture shows the character: `toward` runs from its eyes to the viewer, and `right`
/// and `up` are the picture's own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub toward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

// How far aside the character looks for a cursor half a screen height from its eyes, while
// the cursor is near.
const GAIN: f32 = 45.0 * PI / 180.0;
// A look never asks for more than this, however far away the cursor is.
const MOST: f32 = 80.0 * PI / 180.0;

/// The direction the character looks in for a cursor `offset` from its eyes in the picture:
/// right and up, in half screen heights. With the cursor on its eyes it looks at the viewer.
/// Further away it looks further aside, the way the cursor lies, more slowly the further the
/// cursor goes and never past a limit. So the look depends on the picture alone, not on the
/// lens or how far the camera is. `strength` scales how readily it turns: 1 is normal.
pub fn look_direction(view: &View, offset: Vec2, strength: f32) -> Vec3 {
    let toward = view.toward.normalize_or(Vec3::Z);
    let across = view.right * offset.x + view.up * offset.y;
    let Some(across) = (across - toward * across.dot(toward)).try_normalize() else {
        return toward;
    };
    let angle = MOST * (1.0 - (-offset.length() * GAIN * strength.max(0.0) / MOST).exp());
    toward * angle.cos() + across * angle.sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The viewer stands in front of the character, as Blender's front view does: its left is
    // on the right.
    const VIEW: View = View { toward: Vec3::Z, right: Vec3::X, up: Vec3::Y };

    fn aside(offset: Vec2) -> f32 {
        look_direction(&VIEW, offset, 1.0).angle_between(Vec3::Z).to_degrees()
    }

    #[test]
    fn the_cursor_on_the_eyes_is_a_look_at_the_viewer() {
        assert_eq!(look_direction(&VIEW, Vec2::ZERO, 1.0), Vec3::Z);
    }

    #[test]
    fn the_look_goes_the_way_the_cursor_lies() {
        let look = look_direction(&VIEW, Vec2::new(0.5, 0.5), 1.0);
        assert!(look.x > 0.1 && (look.x - look.y).abs() < 1e-5);
        let look = look_direction(&VIEW, Vec2::new(-1.0, 0.0), 1.0);
        assert!(look.x < -0.1 && look.y.abs() < 1e-5);
    }

    #[test]
    fn further_is_more_but_never_past_the_limit() {
        // The top of the screen above a centred face, the side edge of a 16:9 one, and beyond.
        assert!((aside(Vec2::new(0.0, 1.0)) - 34.4).abs() < 0.2);
        assert!((aside(Vec2::new(16.0 / 9.0, 0.0)) - 50.6).abs() < 0.2);
        let mut last = 0.0;
        for step in 1..200 {
            let now = aside(Vec2::new(step as f32 * 0.1, 0.0));
            assert!(now > last && now < 80.0);
            last = now;
        }
    }

    #[test]
    fn strength_scales_the_turn() {
        let weak = look_direction(&VIEW, Vec2::X, 0.5).angle_between(Vec3::Z);
        let strong = look_direction(&VIEW, Vec2::X, 1.5).angle_between(Vec3::Z);
        assert!(weak < aside(Vec2::X).to_radians() && aside(Vec2::X).to_radians() < strong);
        assert_eq!(look_direction(&VIEW, Vec2::X, 0.0), Vec3::Z);
    }

    #[test]
    fn a_view_from_the_side_still_tilts_across_the_line_of_sight() {
        let view = View { toward: Vec3::new(1.0, 0.0, 1.0), right: Vec3::X, up: Vec3::Y };
        let look = look_direction(&view, Vec2::new(1.0, 0.0), 1.0);
        assert!((look.length() - 1.0).abs() < 1e-5);
        assert!((look.angle_between(view.toward).to_degrees() - 34.4).abs() < 0.2);
    }
}
