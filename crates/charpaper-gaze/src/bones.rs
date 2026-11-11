use glam::Quat;

// Rotations of bones in a chain. "In the armature's space" is a bone's rotation with all its
// parents' counted in; a bone's own rotation is the one against its parent, which is what an
// animation keys and what gaze writes.

/// How far a bone has turned from its rest pose, as the armature sees it. Both rotations in
/// the armature's space.
pub fn turn_from_rest(now: Quat, rest: Quat) -> Quat {
    (now * rest.inverse()).normalize()
}

/// A turn worked out as a bone stood in its rest pose, as the armature sees it now that the
/// bone has turned by `turned` since.
pub fn carried(turned: Quat, turn: Quat) -> Quat {
    turned * turn * turned.inverse()
}

/// A bone's own rotation once it is turned by `by`, a turn in the armature's space, about its
/// own joint. `parent` is its parent's rotation in the armature's space.
pub fn turned_by(own: Quat, by: Quat, parent: Quat) -> Quat {
    (parent.inverse() * by * parent * own).normalize()
}

/// A bone's own rotation that leaves it turned by `turn` from its rest pose, as the armature
/// sees it, whatever it was before. `rest` and `parent` in the armature's space.
pub fn turned_from_rest(turn: Quat, rest: Quat, parent: Quat) -> Quat {
    (parent.inverse() * turn * rest).normalize()
}

#[cfg(test)]
mod tests {
    use glam::EulerRot;
    use glam::Vec2;
    use glam::Vec3;

    use super::*;
    use crate::Eyes;
    use crate::Frame;
    use crate::Setup;

    fn euler(x: f32, y: f32, z: f32) -> Quat {
        Quat::from_euler(EulerRot::XYZ, x.to_radians(), y.to_radians(), z.to_radians())
    }

    fn degrees(yaw: f32, pitch: f32) -> Vec2 {
        Vec2::new(yaw.to_radians(), pitch.to_radians())
    }

    // Chest, neck, head and one eye, each hanging from the one before. Their own rest
    // rotations are whatever the rig's bone rolls made them; the clip turns each a little,
    // and the chest a lot.
    struct Chain {
        rest: [Quat; 4],
        own: [Quat; 4],
    }

    impl Chain {
        fn new(chest_clip: Quat) -> Self {
            let rest = [
                euler(10.0, 80.0, -30.0),
                euler(-5.0, 0.0, 95.0),
                euler(40.0, -90.0, 3.0),
                euler(90.0, 0.0, 45.0),
            ];
            let clip =
                [chest_clip, euler(4.0, -6.0, 2.0), euler(-8.0, 12.0, 5.0), euler(3.0, 20.0, 0.0)];
            Self { rest, own: std::array::from_fn(|i| rest[i] * clip[i]) }
        }

        // In the armature's space.
        fn rotation(own: &[Quat; 4], bone: usize) -> Quat {
            own[..=bone].iter().fold(Quat::IDENTITY, |parents, own| parents * *own)
        }

        fn now(&self, bone: usize) -> Quat {
            Self::rotation(&self.own, bone)
        }

        fn rest(&self, bone: usize) -> Quat {
            Self::rotation(&self.rest, bone)
        }

        fn turn(&self, bone: usize) -> Quat {
            turn_from_rest(self.now(bone), self.rest(bone))
        }
    }

    const CHEST: usize = 0;
    const NECK: usize = 1;
    const HEAD: usize = 2;
    const EYE: usize = 3;

    // The app's steps, in its order, on a chain of plain rotations. Returns where the head and
    // the eye end up pointing, in the armature's space.
    fn look(chain: &mut Chain, frame: &Frame, setup: &Setup, direction: Vec3) -> (Vec3, Vec3) {
        let chest = chain.turn(CHEST);
        let target = frame.angles(chest.inverse() * direction);
        let clip = frame.angles(chest.inverse() * chain.turn(HEAD) * frame.front);
        let head_at = setup.head(target, clip);
        let neck_at = setup.neck(clip, head_at);

        let by = carried(chest, frame.turn(clip, neck_at));
        chain.own[NECK] = turned_by(chain.own[NECK], by, chain.now(CHEST));
        let by = carried(chest, frame.turn(neck_at, head_at));
        chain.own[HEAD] = turned_by(chain.own[HEAD], by, chain.now(NECK));

        if let Some(eyes) = setup.eyes {
            let socket = chain.turn(HEAD);
            let look = eyes.look(frame.angles(socket.inverse() * direction));
            let aimed = socket * frame.toward(eyes.turn(look));
            chain.own[EYE] = turned_from_rest(aimed, chain.rest(EYE), chain.now(HEAD));
        }
        (chain.turn(HEAD) * frame.front, chain.turn(EYE) * frame.front)
    }

    fn person(head_turn: f32, eyes: Option<Eyes>) -> Setup {
        Setup {
            turn_limit: degrees(75.0, 40.0),
            neck_limit: degrees(25.0, 15.0),
            head_share: 0.7,
            head_turn,
            eyes,
        }
    }

    #[test]
    fn the_head_ends_up_pointing_where_it_was_sent() {
        let frame = Frame::facing(Vec3::new(0.2, 0.0, 1.0), Vec3::Y).unwrap();
        // Standing, leaning, and lying on her back.
        for chest in [euler(2.0, -5.0, 1.0), euler(25.0, 30.0, -10.0), euler(-90.0, 15.0, 0.0)] {
            let mut chain = Chain::new(chest);
            let direction = chain.turn(CHEST) * frame.direction(degrees(-50.0, 30.0));
            let (head, _) = look(&mut chain, &frame, &person(1.0, None), direction);
            assert!(head.angle_between(direction) < 1e-3, "{head} is not {direction}");
        }
    }

    #[test]
    fn the_eyes_cover_what_the_head_leaves() {
        let frame = Frame::default();
        let eyes = Eyes { limit: degrees(30.0, 20.0), reach: degrees(30.0, 20.0) };
        for chest in [euler(2.0, -5.0, 1.0), euler(-90.0, 15.0, 0.0)] {
            let mut chain = Chain::new(chest);
            let direction = chain.turn(CHEST) * frame.direction(degrees(40.0, -15.0));
            let (head, eye) = look(&mut chain, &frame, &person(0.6, Some(eyes)), direction);
            assert!(head.angle_between(direction).to_degrees() > 10.0);
            assert!(eye.angle_between(direction) < 1e-3, "{eye} is not {direction}");
        }
    }

    #[test]
    fn eyes_past_their_limit_stop_there_whatever_the_clip_did_to_them() {
        let frame = Frame::default();
        let eyes = Eyes { limit: degrees(30.0, 20.0), reach: degrees(30.0, 20.0) };
        let mut chain = Chain::new(euler(0.0, 0.0, 0.0));
        let setup = Setup { eyes: Some(eyes), ..Setup::default() };
        let direction = frame.direction(degrees(80.0, 0.0));
        let clip_head = chain.turn(HEAD) * frame.front;
        let (head, eye) = look(&mut chain, &frame, &setup, direction);
        // No head or neck to turn, so the head stays where the clip has it.
        assert!(head.angle_between(clip_head) < 1e-4);
        let wanted = frame.angles(chain.turn(HEAD).inverse() * direction);
        let from_head = frame.angles(chain.turn(HEAD).inverse() * eye);
        assert!(wanted.x.to_degrees() > 45.0);
        assert!((from_head.x.to_degrees() - 30.0).abs() < 0.01);
        assert!((from_head.y - wanted.y.clamp(-eyes.limit.y, eyes.limit.y)).abs() < 1e-3);
    }
}
