use std::f32::consts::PI;

use glam::Vec2;

/// Eyes that follow the look, by bones, by shape keys or both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eyes {
    /// How far the eye bones turn at a full look, in radians.
    pub limit: Vec2,
    /// How far aside the target is, seen from the sockets, when the look is full. The same as
    /// `limit` aims a round eyeball exactly. More than `limit` moves the eye in proportion,
    /// for eyes that are no balls in sockets and would overshoot if aimed.
    pub reach: Vec2,
}

impl Eyes {
    /// The look for a target at these angles from the sockets: -1 to 1 each way, left and up
    /// positive.
    pub fn look(&self, target: Vec2) -> Vec2 {
        (target / self.reach.max(Vec2::splat(1e-3))).clamp(Vec2::NEG_ONE, Vec2::ONE)
    }

    pub fn turn(&self, look: Vec2) -> Vec2 {
        look * self.limit
    }
}

/// How a look is shared out. Angles in radians, yaw then pitch.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Setup {
    /// How far the head may point from the chest, by the neck's and its own turn together.
    pub turn_limit: Vec2,
    /// How much of that the neck may add. Zero without a neck.
    pub neck_limit: Vec2,
    /// The head's share of a turn, from 0 to 1. The neck takes the rest.
    pub head_share: f32,
    /// How much of the way to the target the head and neck go, from 0 to 1, leaving the rest
    /// to the eyes. Without eyes they go all the way.
    pub head_turn: f32,
    pub eyes: Option<Eyes>,
}

// The eyes are counted on for this much of their reach; the head makes up the rest, so a look
// far aside doesn't leave them pressed against their limit.
const EYE_COMFORT: f32 = 0.9;
// The character follows a target up to this far past what it can turn to, looking as far as
// it can, and then lets go. It picks the target up again a little earlier than that, so one
// on the edge doesn't make it start and stop.
const PAST_REACH: f32 = 20.0 * PI / 180.0;
const BACK_IN_REACH: f32 = 15.0 * PI / 180.0;
// However little of the character turns, it follows what is beside it; however much, not
// what is right behind it, where left and right would swap.
const LEAST_REACH: f32 = 90.0 * PI / 180.0;
const MOST_REACH: f32 = 140.0 * PI / 180.0;

impl Setup {
    /// Whether the character follows a target at these angles from its chest, given whether
    /// it did a moment ago.
    pub fn in_reach(&self, target: Vec2, was: bool) -> bool {
        let eyes = self.eyes.map_or(0.0, |eyes| eyes.reach.x);
        let reach = (self.turn_limit.x + eyes).clamp(LEAST_REACH, MOST_REACH) + PAST_REACH;
        target.x.abs() < if was { reach } else { reach - BACK_IN_REACH }
    }

    /// Where the head points for a target at these angles from the chest, when the clip
    /// playing points it at `clip`. Part of the way, further when the eyes can't cover the
    /// rest, and never past its limit, except as far as the clip itself takes it.
    pub fn head(&self, target: Vec2, clip: Vec2) -> Vec2 {
        let (share, reach) = match self.eyes {
            Some(eyes) => (self.head_turn.clamp(0.0, 1.0), eyes.reach * EYE_COMFORT),
            None => (1.0, Vec2::ZERO),
        };
        let mut head = clip + (target - clip) * share;
        let left = target - head;
        head += left - left.clamp(-reach, reach);
        head.clamp((-self.turn_limit).min(clip), self.turn_limit.max(clip))
    }

    /// Where the neck's part of the turn alone leaves the head pointing, on its way from
    /// `clip` to `head`.
    pub fn neck(&self, clip: Vec2, head: Vec2) -> Vec2 {
        let part = (head - clip) * (1.0 - self.head_share.clamp(0.0, 1.0));
        clip + part.clamp(-self.neck_limit, self.neck_limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn degrees(yaw: f32, pitch: f32) -> Vec2 {
        Vec2::new(yaw.to_radians(), pitch.to_radians())
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    fn eyeballs() -> Eyes {
        Eyes { limit: degrees(30.0, 20.0), reach: degrees(30.0, 20.0) }
    }

    fn person() -> Setup {
        Setup {
            turn_limit: degrees(75.0, 40.0),
            neck_limit: degrees(25.0, 15.0),
            head_share: 0.7,
            head_turn: 0.6,
            eyes: Some(eyeballs()),
        }
    }

    #[test]
    fn the_head_goes_part_of_the_way_and_the_eyes_the_rest() {
        let head = person().head(degrees(50.0, 10.0), Vec2::ZERO);
        assert!(close(head, degrees(30.0, 6.0)));
    }

    #[test]
    fn the_head_makes_up_for_what_the_eyes_cannot_cover() {
        // 100 from the chest, clip at 30: 60% of the way is 72, leaving 28 to eyes good for 27.
        let head = person().head(degrees(100.0, 0.0), degrees(30.0, 0.0));
        assert!(close(head, degrees(73.0, 0.0)));
    }

    #[test]
    fn the_limit_holds_for_clip_and_gaze_together() {
        let setup = person();
        for clip in [0.0, 30.0, 45.0, 70.0] {
            let head = setup.head(degrees(130.0, 0.0), degrees(clip, 0.0));
            assert!(close(head, degrees(75.0, 0.0)));
        }
        // A clip that goes past the limit by itself is left where it is, not pulled back.
        let head = setup.head(degrees(130.0, 0.0), degrees(85.0, 0.0));
        assert!(close(head, degrees(85.0, 0.0)));
    }

    #[test]
    fn without_eyes_the_head_goes_all_the_way() {
        let setup = Setup { eyes: None, ..person() };
        assert!(close(setup.head(degrees(40.0, -20.0), Vec2::ZERO), degrees(40.0, -20.0)));
    }

    #[test]
    fn the_neck_takes_its_share_up_to_its_limit() {
        let setup = person();
        assert!(close(setup.neck(Vec2::ZERO, degrees(30.0, 10.0)), degrees(9.0, 3.0)));
        assert!(close(setup.neck(degrees(-75.0, 0.0), degrees(75.0, 0.0)), degrees(-50.0, 0.0)));
        let no_neck = Setup { neck_limit: Vec2::ZERO, ..setup };
        assert!(close(no_neck.neck(degrees(5.0, 0.0), degrees(30.0, 10.0)), degrees(5.0, 0.0)));
    }

    #[test]
    fn a_target_behind_is_let_go_of_and_never_swaps_sides() {
        let setup = person();
        // 75 of head and neck and 30 of eyes reach 105, followed to 125, picked up at 110.
        assert!(setup.in_reach(degrees(120.0, 0.0), true));
        assert!(!setup.in_reach(degrees(126.0, 0.0), true));
        assert!(!setup.in_reach(degrees(-120.0, 0.0), false));
        assert!(setup.in_reach(degrees(-109.0, 0.0), false));
        for yaw in [170.0, 179.0, -179.0, -170.0] {
            assert!(!setup.in_reach(degrees(yaw, 0.0), true));
        }
    }

    #[test]
    fn eyes_alone_still_follow_a_target_to_the_side() {
        let setup = Setup { eyes: Some(eyeballs()), ..Setup::default() };
        assert!(setup.in_reach(degrees(80.0, 0.0), false));
        assert!(!setup.in_reach(degrees(115.0, 0.0), true));
    }

    #[test]
    fn eyeballs_are_aimed_and_other_eyes_move_in_proportion() {
        let balls = eyeballs();
        assert!(close(balls.turn(balls.look(degrees(12.0, -8.0))), degrees(12.0, -8.0)));
        assert!(close(balls.turn(balls.look(degrees(50.0, 0.0))), degrees(30.0, 0.0)));
        let drawn = Eyes { limit: degrees(8.0, 6.0), reach: degrees(35.0, 25.0) };
        assert!(close(drawn.look(degrees(17.5, 0.0)), Vec2::new(0.5, 0.0)));
        assert!(close(drawn.turn(drawn.look(degrees(17.5, 0.0))), degrees(4.0, 0.0)));
        let keys_only = Eyes { limit: Vec2::ZERO, reach: degrees(35.0, 25.0) };
        assert!(close(keys_only.look(degrees(-70.0, 25.0)), Vec2::new(-1.0, 1.0)));
    }
}
