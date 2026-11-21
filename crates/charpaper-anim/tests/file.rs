use charpaper_anim::Clip;
use charpaper_anim::ClipError;
use glam::Vec3;

// A .glb animating "arm" and "leg" from frame 10 to 12 at 24 fps, as an exporter counting from
// the scene's first frame might write it.
fn glb() -> Vec<u8> {
    let times = [10.0f32 / 24.0, 12.0 / 24.0];
    let moves = [[0.0f32, 0.0, 0.0], [0.0, 2.0, 0.0]];
    let mut bin = Vec::new();
    times.iter().for_each(|v| bin.extend(v.to_le_bytes()));
    moves.iter().flatten().for_each(|v| bin.extend(v.to_le_bytes()));
    let json = serde_json::json!({
        "asset": { "version": "2.0" },
        "nodes": [{ "name": "arm" }, { "name": "leg" }],
        "buffers": [{ "byteLength": bin.len() }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 8, "byteLength": 24 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR",
              "min": [times[0]], "max": [times[1]] },
            { "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3" }
        ],
        "animations": [{
            "samplers": [{ "input": 0, "output": 1, "interpolation": "LINEAR" }],
            "channels": [
                { "sampler": 0, "target": { "node": 0, "path": "translation" } },
                { "sampler": 0, "target": { "node": 1, "path": "translation" } }
            ]
        }]
    });
    let mut json = serde_json::to_vec(&json).unwrap();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut glb = Vec::new();
    glb.extend(b"glTF");
    glb.extend(2u32.to_le_bytes());
    glb.extend(((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
    glb.extend((json.len() as u32).to_le_bytes());
    glb.extend(b"JSON");
    glb.extend(json);
    glb.extend((bin.len() as u32).to_le_bytes());
    glb.extend(b"BIN\0");
    glb.extend(bin);
    glb
}

#[test]
fn reads_only_the_keyed_nodes_and_starts_at_the_first_key() {
    let json = br#"{ "format": 1, "fps": 24, "frames": 3, "nodes": ["arm"] }"#;
    let clip = Clip::load(json, Some(&glb())).unwrap();
    assert_eq!(clip.nodes.len(), 1);
    let arm = clip.nodes[0].translation.as_ref().unwrap();
    assert_eq!(arm.sample(0.0), Vec3::ZERO);
    assert!((arm.sample(1.0 / 24.0).y - 1.0).abs() < 1e-5);
    assert!((clip.duration - 3.0 / 24.0).abs() < 1e-6);
}

#[test]
fn reads_shape_keys_and_light_by_frame() {
    let json = br#"{
        "format": 1, "fps": 10, "frames": 3,
        "shape_keys": { "Smile": [0, 0.5, 1] },
        "light": { "strength": [1, 2, 3], "color": [[1, 1, 1], [1, 0.5, 0], [1, 0, 0]] }
    }"#;
    let clip = Clip::load(json, None).unwrap();
    let (name, smile) = &clip.shape_keys[0];
    assert_eq!(name, "Smile");
    assert!((smile.sample(0.05) - 0.25).abs() < 1e-5);
    assert_eq!(clip.light.strength.as_ref().unwrap().sample(0.2), 3.0);
    assert_eq!(clip.light.color.as_ref().unwrap().sample(0.1), Vec3::new(1.0, 0.5, 0.0));
}

#[test]
fn refuses_what_it_cant_trust() {
    let load = |json: &str, glb: Option<&[u8]>| Clip::load(json.as_bytes(), glb).unwrap_err();
    assert!(matches!(load(r#"{"format":2,"fps":24,"frames":1}"#, None), ClipError::Format(2)));
    assert!(matches!(load(r#"{"format":1,"fps":0,"frames":1}"#, None), ClipError::Fps(_)));
    assert!(matches!(
        load(r#"{"format":1,"fps":24,"frames":2,"shape_keys":{"Smile":[0]}}"#, None),
        ClipError::Samples { samples: 1, frames: 2, .. }
    ));
    assert!(matches!(
        load(r#"{"format":1,"fps":24,"frames":1,"nodes":["arm"]}"#, None),
        ClipError::NoGlb
    ));
    let glb = glb();
    assert!(matches!(
        load(r#"{"format":1,"fps":24,"frames":3,"nodes":["tail"]}"#, Some(&glb)),
        ClipError::Missing(name) if name == "tail"
    ));
}
