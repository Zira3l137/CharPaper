use std::path::PathBuf;

use charpaper_anim::surface::Surface;
use charpaper_anim::surface::body;
use charpaper_anim::surface::read_parts;
use glam::Vec3;

// A box from -half to +half on X and Z and from 0 to `height` on Y, wound to face out.
fn cube(half: f32, height: f32) -> (Vec<Vec3>, Vec<u16>) {
    let corners = (0..8)
        .map(|i| {
            let x = if i & 1 == 0 { -half } else { half };
            let y = if i & 2 == 0 { 0.0 } else { height };
            let z = if i & 4 == 0 { -half } else { half };
            Vec3::new(x, y, z)
        })
        .collect();
    let faces: [[u16; 4]; 6] =
        [[0, 4, 6, 2], [1, 3, 7, 5], [0, 1, 5, 4], [2, 6, 7, 3], [0, 2, 3, 1], [4, 5, 7, 6]];
    let indices = faces.iter().flat_map(|[a, b, c, d]| [*a, *b, *c, *a, *c, *d]).collect();
    (corners, indices)
}

fn triangles(half: f32, height: f32) -> Vec<[Vec3; 3]> {
    let (corners, indices) = cube(half, height);
    indices.chunks(3).map(|t| [0, 1, 2].map(|i| corners[t[i] as usize])).collect()
}

#[test]
fn a_point_inside_is_pushed_out_through_the_nearest_side_and_one_outside_is_left() {
    let surface = Surface::new(triangles(0.5, 1.0));
    let pushed = surface.push(Vec3::new(0.4, 0.5, 0.0), 0.1);
    assert!((pushed - Vec3::new(0.6, 0.5, 0.0)).length() < 1e-5, "{pushed}");
    let near = surface.push(Vec3::new(0.55, 0.5, 0.0), 0.1);
    assert!((near - Vec3::new(0.6, 0.5, 0.0)).length() < 1e-5, "{near}");
    let far = Vec3::new(0.7, 0.5, 0.0);
    assert_eq!(surface.push(far, 0.1), far);
    // Out at a corner, along the diagonal.
    let corner = surface.push(Vec3::new(0.52, 1.02, 0.52), 0.1);
    assert!((corner - Vec3::new(0.5, 1.0, 0.5)).length() > 0.1 - 1e-5, "{corner}");
}

// A skin of two objects around a bone Head, 1 long, standing at y = 1: a Body box 0.1 thick
// and a Coat box 0.3 thick. Written as .gltf with its .bin beside it.
fn skin_file(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("charpaper-surface-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut bin = Vec::new();
    let mut views = Vec::new();
    let mut push = |bytes: Vec<u8>| {
        views.push(format!(
            r#"{{"buffer": 0, "byteOffset": {}, "byteLength": {}}}"#,
            bin.len(),
            bytes.len()
        ));
        bin.extend(bytes);
        views.len() - 1
    };
    let floats = |values: &[f32]| values.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    for half in [0.1f32, 0.3] {
        let (corners, mut indices) = cube(half, 1.0);
        // In the mesh's space the bone stands at y = 1.
        let positions: Vec<f32> = corners.iter().flat_map(|c| [c.x, c.y + 1.0, c.z]).collect();
        let count = corners.len();
        if indices.len() % 2 == 1 {
            indices.push(0);
        }
        let at = accessors.len();
        let position = push(floats(&positions));
        accessors.push(format!(r#"{{"bufferView": {position}, "componentType": 5126, "count": {count}, "type": "VEC3", "min": [-1, -1, -1], "max": [1, 3, 1]}}"#));
        let joints = push(vec![0u8; count * 8]);
        accessors.push(format!(
            r#"{{"bufferView": {joints}, "componentType": 5123, "count": {count}, "type": "VEC4"}}"#
        ));
        let weights =
            push(floats(&(0..count).flat_map(|_| [1.0, 0.0, 0.0, 0.0]).collect::<Vec<_>>()));
        accessors.push(format!(r#"{{"bufferView": {weights}, "componentType": 5126, "count": {count}, "type": "VEC4"}}"#));
        let index = push(indices.iter().flat_map(|i| i.to_le_bytes()).collect());
        accessors.push(format!(
            r#"{{"bufferView": {index}, "componentType": 5123, "count": 36, "type": "SCALAR"}}"#
        ));
        meshes.push(format!(
            r#"{{"primitives": [{{"attributes": {{"POSITION": {at}, "JOINTS_0": {}, "WEIGHTS_0": {}}}, "indices": {}}}]}}"#,
            at + 1, at + 2, at + 3
        ));
    }
    // From the mesh into Head's space: down by 1.
    let mut inverse = Vec::new();
    for column in
        [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, -1.0, 0.0, 1.0]]
    {
        inverse.extend(column);
    }
    let matrices = push(floats(&inverse));
    accessors.push(format!(
        r#"{{"bufferView": {matrices}, "componentType": 5126, "count": 1, "type": "MAT4"}}"#
    ));
    let json = format!(
        r#"{{"asset": {{"version": "2.0"}}, "scene": 0, "scenes": [{{"nodes": [0, 1, 2]}}],
        "nodes": [
            {{"name": "Head", "translation": [0, 1, 0]}},
            {{"name": "Body", "mesh": 0, "skin": 0}},
            {{"name": "Coat", "mesh": 1, "skin": 0}}
        ],
        "skins": [{{"joints": [0], "inverseBindMatrices": {}}}],
        "meshes": [{}],
        "buffers": [{{"uri": "skin.bin", "byteLength": {}}}],
        "bufferViews": [{}],
        "accessors": [{}]}}"#,
        accessors.len() - 1,
        meshes.join(", "),
        bin.len(),
        views.join(", "),
        accessors.join(", ")
    );
    std::fs::write(folder.join("skin.bin"), bin).unwrap();
    std::fs::write(folder.join("skin.gltf"), json).unwrap();
    folder.join("skin.gltf")
}

#[test]
fn parts_are_read_in_the_bones_space_and_the_body_is_the_thinnest_shown() {
    let path = skin_file("read");
    let parts = read_parts(&path, &[("Head".into(), 1.0), ("Hand".into(), 0.2)]).unwrap();
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
    let summary: Vec<(&str, &str, usize)> =
        parts.iter().map(|p| (p.object.as_str(), p.bone.as_str(), p.surface.len())).collect();
    assert_eq!(summary, [("Body", "Head", 12), ("Coat", "Head", 12)]);
    assert!((parts[0].thickness - 0.1f32.hypot(0.1)).abs() < 1e-5, "{}", parts[0].thickness);

    // In Head's space the Body box stands from 0 to 1: a point at its middle goes out sideways.
    let shown = body(&parts, |_| false);
    assert_eq!(shown["Head"], 0);
    let out = parts[shown["Head"]].surface.push(Vec3::new(0.05, 0.5, 0.0), 0.0);
    assert!((out - Vec3::new(0.1, 0.5, 0.0)).length() < 1e-5, "{out}");
    let coat = body(&parts, |object| object == "Body");
    assert_eq!(coat["Head"], 1);
    let out = parts[coat["Head"]].surface.push(Vec3::new(0.05, 0.5, 0.0), 0.0);
    assert!((out - Vec3::new(0.3, 0.5, 0.0)).length() < 1e-5, "{out}");
}
