use charpaper_gltf::Clip;
use charpaper_gltf::ClipNode;
use charpaper_gltf::Trs;
use charpaper_gltf::blender::Bones;
use charpaper_gltf::blender::Y_UP;
use charpaper_gltf::blender::object_local;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;

fn trs(x: f32, turn: f32) -> Trs {
    Trs {
        translation: Vec3::new(x, 0.0, 0.0),
        rotation: Quat::from_rotation_y(turn),
        scale: Vec3::ONE,
    }
}

fn json(frames: usize, nodes: &[&str]) -> Vec<u8> {
    serde_json::json!({ "format": 1, "fps": 24.0, "frames": frames, "nodes": nodes })
        .to_string()
        .into_bytes()
}

#[test]
fn the_app_reads_what_it_writes() {
    let moving = ClipNode {
        name: "Arm".into(),
        samples: (0..24).map(|i| trs(i as f32, 0.1 * i as f32)).collect(),
    };
    let still = ClipNode { name: "Head".into(), samples: vec![trs(2.0, 0.5); 24] };
    let clip = Clip {
        name: "Wave".into(),
        step_seconds: 1.0 / 24.0,
        optimize: true,
        nodes: vec![moving, still],
    };
    let glb = clip.to_glb();

    let loaded = charpaper_anim::Clip::load(&json(24, &["Arm", "Head"]), Some(&glb)).unwrap();
    assert_eq!(loaded.nodes.len(), 2);

    let gltf = gltf::Gltf::from_slice(&glb).unwrap();
    let animation = gltf.animations().next().unwrap();
    let keys: Vec<(String, usize)> = animation
        .channels()
        .map(|c| (c.target().node().name().unwrap().to_string(), c.sampler().input().count()))
        .collect();
    // The still head keeps its first and last keys only, as Blender's Optimize keys does.
    assert!(keys.iter().filter(|(n, _)| n == "Arm").all(|(_, k)| *k == 24 || *k == 2));
    assert!(keys.iter().filter(|(n, _)| n == "Head").all(|(_, k)| *k == 2));
    assert_eq!(keys.iter().filter(|(n, k)| n == "Arm" && *k == 24).count(), 2, "{keys:?}");
}

#[test]
fn a_held_clip_has_one_key() {
    let clip = Clip {
        name: "Sit".into(),
        step_seconds: 1.0 / 24.0,
        optimize: true,
        nodes: vec![ClipNode { name: "Hips".into(), samples: vec![trs(1.0, 0.2)] }],
    };
    charpaper_anim::Clip::load(&json(1, &["Hips"]), Some(&clip.to_glb())).unwrap();
}

#[test]
fn rotations_keep_to_one_side() {
    let flipped = |i: usize| {
        let q = Quat::from_rotation_z(0.01 * i as f32);
        if i.is_multiple_of(2) { q } else { -q }
    };
    let samples = (0..10)
        .map(|i| Trs { translation: Vec3::ZERO, rotation: flipped(i), scale: Vec3::ONE })
        .collect();
    let clip = Clip {
        name: "Spin".into(),
        step_seconds: 0.1,
        optimize: false,
        nodes: vec![ClipNode { name: "Top".into(), samples }],
    };
    let glb = clip.to_glb();
    let gltf = gltf::Gltf::from_slice(&glb).unwrap();
    let blob = gltf.blob.as_deref();
    let animation = gltf.animations().next().unwrap();
    let channel = animation.channels().nth(1).unwrap();
    let reader = channel.reader(|_| blob);
    let Some(gltf::animation::util::ReadOutputs::Rotations(rotations)) = reader.read_outputs()
    else {
        panic!("no rotations")
    };
    let rotations: Vec<[f32; 4]> = rotations.into_f32().collect();
    assert!(rotations.windows(2).all(|w| Quat::from_array(w[0]).dot(Quat::from_array(w[1])) > 0.0));
}

#[test]
fn a_bone_at_rest_is_its_rest_transform() {
    // A root bone standing up from the origin, and a child one unit up it.
    let root_rest = Mat4::IDENTITY;
    let child_rest = Mat4::from_translation(Vec3::Z);
    let bones = Bones {
        rest: vec![root_rest, child_rest],
        parents: vec![None, Some(0)],
        has_parent: vec![false, true],
    };
    let at_rest = bones.locals(&[root_rest, child_rest]);
    assert!(at_rest[0].translation.abs_diff_eq(Vec3::ZERO, 1e-6));
    // The root bone carries the turn from Blender's Z-up to glTF's Y-up.
    let up = at_rest[0].rotation * Vec3::Y;
    assert!(up.abs_diff_eq(Y_UP.transform_vector3(Vec3::Y), 1e-6), "{up}");
    assert!(at_rest[1].translation.abs_diff_eq(Vec3::Z, 1e-6));
    assert!(at_rest[1].rotation.abs_diff_eq(Quat::IDENTITY, 1e-6));

    // Turned 90° about its own axis, the child's local rotation is that turn.
    let turned = child_rest * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let posed = bones.locals(&[root_rest, turned]);
    assert!(
        posed[1].rotation.abs_diff_eq(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2), 1e-5)
    );
}

#[test]
fn objects_turn_to_y_up() {
    let local = object_local(Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)));
    assert_eq!(local.translation, Vec3::new(1.0, 3.0, -2.0));
    let turned = object_local(Mat4::from_rotation_z(0.5));
    // Blender's Z, up, is glTF's Y.
    assert!(turned.rotation.abs_diff_eq(Quat::from_rotation_y(0.5), 1e-6));
}
