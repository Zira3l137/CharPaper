use charpaper_gltf::Channel;
use charpaper_gltf::Clip;
use charpaper_gltf::ClipNode;
use charpaper_gltf::Scene;
use charpaper_gltf::Trs;
use charpaper_gltf::mesh::MeshData;
use charpaper_gltf::mesh::ShapeKey;
use charpaper_gltf::mesh::SkinData;
use charpaper_gltf::scene::Camera;
use charpaper_gltf::scene::Light;
use charpaper_gltf::scene::LightKind;
use charpaper_gltf::scene::Node;
use charpaper_gltf::scene::Projection;
use charpaper_gltf::scene::Skin;
use glam::Mat4;
use glam::Vec2;
use glam::Vec3;

/// A unit square on the ground, as two triangles over four vertices and six corners.
fn square(normals: Vec<Vec3>) -> MeshData {
    MeshData {
        name: "Square".into(),
        positions: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        corner_verts: vec![0, 1, 2, 0, 2, 3],
        normals,
        uvs: vec![vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ]],
        triangles: vec![[0, 1, 2], [3, 4, 5]],
        triangle_slots: vec![0, 0],
        ..Default::default()
    }
}

#[test]
fn corners_alike_share_a_vertex() {
    let primitives = square(vec![Vec3::Z; 6]).primitives();
    assert_eq!(primitives.len(), 1);
    let square = &primitives[0];
    assert_eq!(square.positions.len(), 4);
    assert_eq!(square.indices, [0, 1, 2, 0, 2, 3]);
    assert_eq!(square.positions[2], [1.0, 0.0, -1.0], "Blender's Y is glTF's -Z");
    assert_eq!(square.normals[0], [0.0, 1.0, 0.0], "Blender's Z is glTF's Y");
    assert_eq!(square.uvs[0][2], [1.0, 0.0], "glTF's V runs down");
}

#[test]
fn corners_that_differ_split_their_vertex() {
    let tilted = Vec3::new(0.0, 0.6, 0.8);
    let normals = vec![Vec3::Z, Vec3::Z, Vec3::Z, tilted, tilted, tilted];
    let primitives = square(normals).primitives();
    assert_eq!(primitives[0].positions.len(), 6);
}

#[test]
fn float_noise_does_not_split_a_vertex() {
    let noisy = Vec3::new(0.0, 0.00001, 1.0);
    let normals = vec![Vec3::Z, Vec3::Z, Vec3::Z, noisy, noisy, noisy];
    assert_eq!(square(normals).primitives()[0].positions.len(), 4);
}

#[test]
fn each_material_slot_is_a_primitive() {
    let mut mesh = square(vec![Vec3::Z; 6]);
    mesh.triangle_slots = vec![0, 2];
    let primitives = mesh.primitives();
    assert_eq!(primitives.iter().map(|p| p.slot).collect::<Vec<_>>(), [0, 2]);
    assert!(primitives.iter().all(|p| p.positions.len() == 3 && p.indices == [0, 1, 2]));
}

#[test]
fn a_vertex_keeps_its_four_strongest_joints() {
    let mut mesh = square(vec![Vec3::Z; 6]);
    let five = vec![(0, 0.1), (1, 0.4), (2, 0.2), (3, 0.05), (4, 0.25)];
    mesh.skin = Some(SkinData {
        influences: vec![five, vec![(0, 1.0)], vec![(0, 1.0)], vec![(0, 1.0)]],
        object_world: Mat4::IDENTITY,
        armature_world: Mat4::IDENTITY,
    });
    let primitive = &mesh.primitives()[0];
    assert_eq!(primitive.joints[0], [1, 4, 2, 0]);
    let sum: f32 = primitive.weights[0].iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
    assert!((primitive.weights[0][0] - 0.4 / 0.95).abs() < 1e-6);
}

#[test]
fn the_file_holds_the_mesh_its_shape_keys_and_its_skin() {
    let mut mesh = square(vec![Vec3::Z; 6]);
    let mut raised = mesh.positions.clone();
    raised[2].z += 0.5;
    mesh.shape_keys =
        vec![ShapeKey { name: "Raise".into(), weight: 0.25, positions: raised, normals: None }];
    mesh.skin = Some(SkinData {
        influences: vec![vec![(0, 1.0)]; 4],
        object_world: Mat4::IDENTITY,
        armature_world: Mat4::IDENTITY,
    });
    let node = |name: &str, children: Vec<usize>, mesh, skin| Node {
        name: name.into(),
        trs: Trs::IDENTITY,
        children,
        mesh,
        skin,
        light: None,
        camera: None,
    };
    let scene = Scene {
        name: "Test".into(),
        nodes: vec![
            node("Armature", vec![1, 2], None, None),
            node("Bone", vec![], None, None),
            node("Square", vec![], Some(0), Some(0)),
        ],
        roots: vec![0],
        meshes: vec![mesh],
        skins: vec![Skin {
            name: "Armature".into(),
            joints: vec![1],
            inverse_binds: vec![Mat4::IDENTITY],
        }],
        ..Default::default()
    };
    let glb = scene.to_glb().unwrap();

    let file = gltf::Gltf::from_slice(&glb).unwrap();
    let blob = file.blob.as_deref();
    let square = file.meshes().next().unwrap();
    let names = square.extras().as_ref().unwrap().get();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(names).unwrap()["targetNames"][0],
        "Raise"
    );
    assert_eq!(square.weights(), Some(&[0.25][..]));

    let primitive = square.primitives().next().unwrap();
    let reader = primitive.reader(|_| blob);
    assert_eq!(reader.read_positions().unwrap().count(), 4);
    assert_eq!(reader.read_indices().unwrap().into_u32().collect::<Vec<_>>(), [0, 1, 2, 0, 2, 3]);
    let (deltas, _, _) = reader.read_morph_targets().next().unwrap();
    assert_eq!(deltas.unwrap().nth(2), Some([0.0, 0.5, 0.0]));
    let joints: Vec<[u16; 4]> = reader.read_joints(0).unwrap().into_u16().collect();
    assert_eq!(joints, [[0, 0, 0, 0]; 4]);

    let skin = file.skins().next().unwrap();
    assert_eq!(skin.joints().map(|j| j.name().unwrap().to_string()).collect::<Vec<_>>(), ["Bone"]);
    let skinned = file.nodes().find(|n| n.name() == Some("Square")).unwrap();
    assert_eq!(skinned.skin().map(|s| s.index()), Some(0));
}

#[test]
fn an_unweighted_vertex_is_left_to_blender() {
    let mut mesh = square(vec![Vec3::Z; 6]);
    mesh.skin = Some(SkinData {
        influences: vec![vec![(0, 1.0)], vec![], vec![(0, 1.0)], vec![(0, 1.0)]],
        object_world: Mat4::IDENTITY,
        armature_world: Mat4::IDENTITY,
    });
    let scene = Scene { meshes: vec![mesh], ..Default::default() };
    assert!(scene.to_glb().is_err());
}

fn plain(name: &str, light: Option<usize>, camera: Option<usize>) -> Node {
    Node {
        name: name.into(),
        trs: Trs::IDENTITY,
        children: vec![],
        mesh: None,
        skin: None,
        light,
        camera,
    }
}

#[test]
fn lights_and_cameras_and_what_moves_them() {
    let lamp = Light {
        name: "Lamp".into(),
        kind: LightKind::Spot,
        color: [1.0, 0.5, 0.25],
        intensity: 12.0,
        range: Some(4.0),
        cone: Some((0.2, 0.4)),
    };
    let lens = Projection::Perspective { aspect_ratio: 1.5, yfov: 0.7, znear: 0.1, zfar: 100.0 };
    let moving = ClipNode {
        name: "Cam".into(),
        samples: vec![Trs::IDENTITY; 3],
        channels: [Channel::Keyed, Channel::Baked, Channel::Baked],
    };
    let mut scene = Scene {
        name: "Shot".into(),
        nodes: vec![plain("Lamp", Some(0), None), plain("Cam", None, Some(0))],
        roots: vec![0, 1],
        lights: vec![lamp],
        cameras: vec![Camera { name: "Lens".into(), projection: lens }],
        animation: Some(Clip {
            name: "Shot".into(),
            step_seconds: 0.5,
            optimize: true,
            drop_still_baked: true,
            nodes: vec![moving],
        }),
        ..Default::default()
    };
    let glb = scene.to_glb().unwrap();
    let file = gltf::Gltf::from_slice(&glb).unwrap();
    assert!(file.extensions_used().any(|e| e == "KHR_lights_punctual"));
    let light = file.lights().unwrap().next().unwrap();
    assert_eq!(light.range(), Some(4.0));
    assert!(matches!(light.kind(), gltf::khr_lights_punctual::Kind::Spot { .. }));
    let camera = file.nodes().find_map(|n| n.camera()).unwrap();
    assert_eq!(camera.name(), Some("Lens"));
    let animation = file.animations().next().unwrap();
    let channels: Vec<_> = animation.channels().map(|c| c.target().node().name()).collect();
    assert_eq!(channels, [Some("Cam")], "only its still keyed channel stays");

    scene.animation.as_mut().unwrap().nodes[0].name = "Elsewhere".into();
    assert!(scene.to_glb().is_err());
}
