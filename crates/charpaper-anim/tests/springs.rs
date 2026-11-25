use charpaper_anim::Transform;
use charpaper_anim::springs::Capsule;
use charpaper_anim::springs::Chain;
use charpaper_anim::springs::Collider;
use charpaper_anim::springs::Forces;
use charpaper_anim::springs::Joint;
use charpaper_anim::springs::Settings;
use charpaper_anim::springs::Wind;
use charpaper_anim::surface::Surface;
use glam::Affine3A;
use glam::Quat;
use glam::Vec3;

const DOWN: Forces = Forces { gravity: Vec3::new(0.0, -9.81, 0.0), wind: Vec3::ZERO };

// Three bones of length 1 from the origin along +X: the first turned from +Y, the way bones
// point, to +X; the others straight on from it.
fn sideways() -> Vec<Transform> {
    let turn = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    let mut locals = vec![Transform { rotation: turn, ..Default::default() }];
    locals.extend((0..2).map(|_| Transform { translation: Vec3::Y, ..Default::default() }));
    locals
}

fn chain(settings: Settings) -> Chain {
    Chain::new(vec![Joint { length: 1.0, radius: 0.1 }; 3], settings)
}

// Each joint's head and the last one's tail, in the world.
fn points(locals: &[Transform]) -> Vec<Vec3> {
    let mut world = Affine3A::IDENTITY;
    let mut points = Vec::new();
    for local in locals {
        world *= Affine3A::from_scale_rotation_translation(
            local.scale,
            local.rotation,
            local.translation,
        );
        points.push(world.translation.into());
    }
    points.push(world.transform_point3(Vec3::Y));
    points
}

fn run(
    chain: &mut Chain,
    animated: &[Transform],
    colliders: &[Collider],
    secs: f32,
    dt: f32,
) -> Vec<Transform> {
    let mut locals = animated.to_vec();
    for _ in 0..(secs / dt).round() as usize {
        locals = animated.to_vec();
        chain.update(Affine3A::IDENTITY, &mut locals, colliders, DOWN, dt);
    }
    locals
}

#[test]
fn a_stiff_chain_follows_the_animation() {
    let animated = sideways();
    let mut stiff = chain(Settings { stiffness: 1.0, drag: 1.0, gravity: 0.0 });
    let locals = run(&mut stiff, &animated, &[], 1.0, 1.0 / 60.0);
    for (got, want) in locals.iter().zip(&animated) {
        assert!(got.rotation.angle_between(want.rotation) < 1e-3);
    }
}

#[test]
fn a_loose_chain_hangs_down_and_keeps_its_length() {
    let animated = sideways();
    let mut loose = chain(Settings { stiffness: 0.0, drag: 0.1, gravity: 1.0 });
    let points = points(&run(&mut loose, &animated, &[], 5.0, 1.0 / 60.0));
    assert!(points[3].y < -2.8, "tail at {}", points[3]);
    for pair in points.windows(2) {
        assert!((pair[0].distance(pair[1]) - 1.0).abs() < 1e-3);
    }
}

#[test]
fn stiffness_holds_against_gravity() {
    let animated = sideways();
    let sag = |stiffness| {
        let mut springy = chain(Settings { stiffness, drag: 0.2, gravity: 1.0 });
        points(&run(&mut springy, &animated, &[], 3.0, 1.0 / 60.0))[3].y
    };
    let (loose, firm, stiff) = (sag(0.0), sag(0.05), sag(0.5));
    assert!(loose < firm && firm < stiff && stiff < 0.0, "{loose} {firm} {stiff}");
}

#[test]
fn a_collider_holds_the_chain_off_it() {
    let animated = sideways();
    // A log under the chain, across it.
    let log = Capsule { a: Vec3::new(1.5, -0.6, -2.0), b: Vec3::new(1.5, -0.6, 2.0), radius: 0.4 };
    let mut loose = chain(Settings { stiffness: 0.0, drag: 0.2, gravity: 1.0 });
    let points = points(&run(&mut loose, &animated, &[log.into()], 3.0, 1.0 / 60.0));
    for tail in &points[1..] {
        let off_axis = Vec3::new(tail.x - 1.5, tail.y + 0.6, 0.0).length();
        // A hair's breadth in is fine: a pushed tail is pulled back to its length after.
        assert!(off_axis >= 0.5 - 2e-3, "tail {tail} inside the log by {}", 0.5 - off_axis);
    }
}

#[test]
fn it_moves_the_same_at_any_frame_rate() {
    let animated = sideways();
    let settings = Settings { stiffness: 0.05, drag: 0.1, gravity: 1.0 };
    let slow = run(&mut chain(settings), &animated, &[], 1.0, 1.0 / 30.0);
    let fast = run(&mut chain(settings), &animated, &[], 1.0, 1.0 / 120.0);
    for (slow, fast) in points(&slow).iter().zip(points(&fast)) {
        assert!(slow.distance(fast) < 1e-2, "{slow} vs {fast}");
    }
}

#[test]
fn it_lags_behind_a_quick_move() {
    let animated = sideways();
    let mut springy = chain(Settings { stiffness: 0.1, drag: 0.1, gravity: 0.0 });
    run(&mut springy, &animated, &[], 1.0, 1.0 / 60.0);
    let mut locals = animated.clone();
    let moved = Affine3A::from_translation(Vec3::new(0.0, 0.0, 0.5));
    springy.update(moved, &mut locals, &[], DOWN, 1.0 / 60.0);
    let tail = moved.transform_point3(points(&locals)[3]);
    assert!(tail.z < 0.45, "tail kept up at {tail}");
}

#[test]
fn a_long_pause_is_let_go_of_not_caught_up_with() {
    let animated = sideways();
    let mut loose = chain(Settings { stiffness: 0.0, drag: 0.0, gravity: 1.0 });
    let locals = run(&mut loose, &animated, &[], 10.0, 10.0);
    // A quarter of a second of falling at most: barely under where it started.
    let tail = points(&locals)[3];
    assert!(tail.is_finite() && tail.y > -2.0, "tail at {tail}");
}

#[test]
fn reset_starts_again_from_the_animation() {
    let animated = sideways();
    let mut loose = chain(Settings { stiffness: 0.0, drag: 0.1, gravity: 1.0 });
    run(&mut loose, &animated, &[], 2.0, 1.0 / 60.0);
    loose.reset();
    let mut locals = animated.clone();
    loose.update(Affine3A::IDENTITY, &mut locals, &[], DOWN, 0.0);
    for (got, want) in locals.iter().zip(&animated) {
        assert!(got.rotation.angle_between(want.rotation) < 1e-4);
    }
}

#[test]
fn a_capsule_around_a_bone_follows_its_scale() {
    let bone = Affine3A::from_scale_rotation_translation(Vec3::splat(2.0), Quat::IDENTITY, Vec3::X);
    let capsule = Capsule::around(bone, 0.5, 0.1);
    assert_eq!((capsule.a, capsule.b), (Vec3::X, Vec3::new(1.0, 1.0, 0.0)));
    assert!((capsule.radius - 0.2).abs() < 1e-6);
}

// Two triangles making a floor at y = -0.6 under the chain, facing up, in a bone's space with
// the bone moved down by 0.6.
fn floor() -> Surface {
    let corner = |x: f32, z: f32| Vec3::new(x, 0.0, z);
    Surface::new([
        [corner(-5.0, -5.0), corner(-5.0, 5.0), corner(5.0, 5.0)],
        [corner(-5.0, -5.0), corner(5.0, 5.0), corner(5.0, -5.0)],
    ])
}

#[test]
fn a_surface_holds_the_chain_off_it_where_its_bone_is() {
    let animated = sideways();
    let floor = floor();
    let bone = Affine3A::from_translation(Vec3::new(0.0, -0.6, 0.0));
    let mut loose = chain(Settings { stiffness: 0.0, drag: 0.2, gravity: 1.0 });
    let points =
        points(&run(&mut loose, &animated, &[Collider::surface(&floor, bone)], 3.0, 1.0 / 60.0));
    for tail in &points[1..] {
        assert!(tail.y >= -0.5 - 2e-3, "tail {tail} under the floor");
    }
    // It did fall onto it.
    assert!(points[3].y < -0.3, "{points:?}");
}

#[test]
fn wind_blows_a_loose_chain_its_way() {
    let animated = sideways();
    let mut loose = chain(Settings { stiffness: 0.02, drag: 0.2, gravity: 1.0 });
    let mut locals = animated.clone();
    let forces = Forces { wind: Vec3::new(0.0, 0.0, 9.81), ..DOWN };
    for _ in 0..300 {
        locals = animated.clone();
        loose.update(Affine3A::IDENTITY, &mut locals, &[], forces, 1.0 / 60.0);
    }
    // As strong as gravity, it takes the chain as far its way as gravity takes it down.
    let tip = points(&locals)[3];
    assert!(tip.z > 0.5 && (tip.z + tip.y).abs() < 1e-3, "{tip}");
}

#[test]
fn gusts_come_and_go_and_roll_along_the_wind() {
    let wind = Wind { direction: Vec3::new(2.0, 0.0, 0.0), strength: 4.0, gusts: 1.0 };
    let strengths: Vec<f32> =
        (0..400).map(|i| wind.at(Vec3::ZERO, i as f64 * 0.05).length()).collect();
    let (calmest, strongest) =
        strengths.iter().fold((f32::MAX, 0.0f32), |(lo, hi), s| (lo.min(*s), hi.max(*s)));
    assert!(calmest < 1.5 && strongest > 6.5 && strongest <= 8.0 + 1e-4, "{calmest} {strongest}");
    // A point 1.5 m downwind feels what the origin felt a second before.
    let later = wind.at(Vec3::new(1.5, 0.0, 0.0), 11.0);
    assert!((later - wind.at(Vec3::ZERO, 10.0)).length() < 1e-4, "{later}");
    let steady = Wind { gusts: 0.0, ..wind };
    assert!((steady.at(Vec3::ONE, 123.4) - Vec3::new(4.0, 0.0, 0.0)).length() < 1e-5);
    // Still precise after days of running.
    let days = 3.0 * 86_400.0;
    let step = (wind.at(Vec3::ZERO, days + 0.01) - wind.at(Vec3::ZERO, days)).length();
    assert!(step < 0.2, "{step}");
}
