use std::fs;
use std::path::Path;
use std::path::PathBuf;

use charpaper_suite::ClipSet;
use charpaper_suite::Severity;
use charpaper_suite::Suite;
use charpaper_suite::SuiteError;
use charpaper_suite::inspect;

// Never read: inspection only looks at the JSON, but validation wants every
// accessor to point somewhere.
const ACCESSORS: &str = r#""buffers": [{"uri": "unused.bin", "byteLength": 64}],
"bufferViews": [{"buffer": 0, "byteLength": 64}],
"accessors": [
    {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
     "min": [0, 0, 0], "max": [1, 1, 1]},
    {"bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0], "max": [1]},
    {"bufferView": 0, "componentType": 5126, "count": 2, "type": "VEC4"},
    {"bufferView": 0, "componentType": 5126, "count": 2, "type": "VEC3"}
]"#;

const MESH: &str = r#""meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}]"#;

fn model() -> String {
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scene": 0, "scenes": [{{"nodes": [0]}}],
        "nodes": [
            {{"name": "Armature", "children": [1]}},
            {{"name": "Hips", "children": [2]}},
            {{"name": "Spine", "children": [3], "translation": [0, 0.5, 0]}},
            {{"name": "Head"}}
        ],
        {ACCESSORS}}}"#
    )
}

fn animation(armature: &str, clip: &str) -> String {
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scenes": [{{"nodes": [0]}}],
        "nodes": [
            {{"name": "{armature}", "children": [1]}},
            {{"name": "Hips", "children": [2]}},
            {{"name": "Spine", "children": [3]}},
            {{"name": "Head"}}
        ],
        "animations": [{{"name": "{clip}",
            "channels": [{{"sampler": 0, "target": {{"node": 2, "path": "rotation"}}}}],
            "samplers": [{{"input": 1, "output": 2}}]}}],
        {ACCESSORS}}}"#
    )
}

fn outfit(spine_y: f32) -> String {
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scenes": [{{"nodes": [0]}}],
        "nodes": [
            {{"name": "Armature", "children": [1, 5]}},
            {{"name": "Hips", "children": [2]}},
            {{"name": "Spine", "children": [3], "translation": [0, {spine_y}, 0]}},
            {{"name": "Head", "children": [4, 6]}},
            {{"name": "Ponytail"}},
            {{"name": "Head", "mesh": 0, "skin": 0}},
            {{"name": "Hat", "mesh": 0}}
        ],
        "skins": [{{"joints": [1, 2, 3, 4]}}], {MESH}, {ACCESSORS}}}"#
    )
}

/// A camera under a rig empty, with one clip per name moving the rig.
fn camera(clips: &[&str]) -> String {
    let animations: Vec<String> = clips
        .iter()
        .map(|name| {
            format!(
                r#"{{"name": "{name}",
                "channels": [{{"sampler": 0, "target": {{"node": 0, "path": "translation"}}}}],
                "samplers": [{{"input": 1, "output": 3}}]}}"#
            )
        })
        .collect();
    let animations = match animations.is_empty() {
        true => String::new(),
        false => format!(r#""animations": [{}],"#, animations.join(", ")),
    };
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scenes": [{{"nodes": [0]}}],
        "nodes": [
            {{"name": "Rig", "children": [1]}},
            {{"name": "Lens", "camera": 0}}
        ],
        "cameras": [{{"type": "perspective", "perspective": {{"yfov": 0.6, "znear": 0.1}}}}],
        {animations} {ACCESSORS}}}"#
    )
}

/// Just the fixed KTX2 header; inspection never reads further.
fn ktx2(faces: u32, levels: u32, supercompression: u32) -> Vec<u8> {
    let mut bytes = vec![0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n'];
    for word in [0, 1, 64, 64, 0, 0, faces, levels, supercompression] {
        bytes.extend_from_slice(&u32::to_le_bytes(word));
    }
    bytes
}

/// A scene holding one directional light and nothing else.
fn lit_scene() -> String {
    r#"{"asset": {"version": "2.0"}, "scenes": [{"nodes": [0]}],
        "extensionsUsed": ["KHR_lights_punctual"],
        "extensions": {"KHR_lights_punctual": {"lights": [{"type": "directional"}]}},
        "nodes": [{"name": "Sun", "extensions": {"KHR_lights_punctual": {"light": 0}}}]}"#
        .to_string()
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str, manifest: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("charpaper-suite-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let fixture = Self(root);
        fixture.write("suite.toml", manifest);
        fixture.write("aki.gltf", &model());
        fixture.write("animations/idle.gltf", &animation("Armature", "Idle"));
        fixture.write("skins/casual.gltf", &outfit(0.5));
        fixture.write("skins/dress.gltf", &outfit(0.5));
        fixture.write("skins/notes.txt", "ignored");
        fixture.write("skins/textures/ignored.gltf", "not a skin");
        fixture.write("cameras/closeup.gltf", &camera(&["Dolly"]));
        fixture.write("cameras/wide.gltf", &camera(&[]));
        fixture.write("environment/stage.gltf", &lit_scene());
        fixture
    }

    fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn load(&self) -> Result<Suite, SuiteError> {
        Suite::load(&self.0)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn messages(suite: &Suite, severity: Severity) -> Vec<String> {
    inspect(suite)
        .findings
        .into_iter()
        .filter(|f| f.severity == severity)
        .map(|f| f.to_string())
        .collect()
}

#[test]
fn folder_layout_fills_in_what_the_manifest_leaves_out() {
    let fixture = Fixture::new("convention", "schema = 1");
    let suite = fixture.load().unwrap();

    assert_eq!(suite.model, Path::new("aki.gltf"));
    assert_eq!(suite.animations.len(), 1);
    assert_eq!(suite.animations[0].clips, ClipSet::All);
    let skins: Vec<&str> = suite.skins.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(skins, ["casual", "dress"]);
    assert_eq!(suite.skins[1].file, Path::new("skins/dress.gltf"));
    assert_eq!(suite.default_skin.as_deref(), Some("casual"));

    let errors = messages(&suite, Severity::Error);
    assert!(errors.is_empty(), "{errors:#?}");
    let warnings = messages(&suite, Severity::Warning);
    assert_eq!(warnings.len(), 2, "{warnings:#?}");
    assert!(warnings.iter().all(|w| w.contains("Ponytail -> Head")));
}

#[test]
fn listing_a_file_replaces_its_discovered_clips() {
    let manifest = r#"
        schema = 1
        [character]
        default_animation = "wave"
        [animations.wave]
        file = "animations/wave.gltf"
        mode = "once"
        [animations.broken]
        file = "animations/wave.gltf"
        clip = "Nope"
    "#;
    let fixture = Fixture::new("listed", manifest);
    fixture.write("animations/wave.gltf", &animation("Armature", "Wave"));
    let suite = fixture.load().unwrap();

    let wave = suite.animations.iter().find(|a| a.path.ends_with("wave.gltf")).unwrap();
    assert!(matches!(&wave.clips, ClipSet::Listed(b) if b.len() == 2));

    let errors = messages(&suite, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(errors[0].contains("no clip named \"Nope\""));
}

#[test]
fn renamed_armature_is_reported() {
    let fixture = Fixture::new("renamed", "schema = 1");
    fixture.write("animations/idle.gltf", &animation("Rig", "Idle"));
    let suite = fixture.load().unwrap();

    let warnings = messages(&suite, Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("Rig/Hips/Spine")), "{warnings:#?}");
}

#[test]
fn skin_with_a_different_rest_pose_is_reported() {
    let fixture = Fixture::new("rest", "schema = 1");
    fixture.write("skins/casual.gltf", &outfit(0.6));
    let suite = fixture.load().unwrap();

    let warnings = messages(&suite, Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("rest pose of 1 bone(s)")), "{warnings:#?}");
}

#[test]
fn bad_manifests_are_rejected() {
    let cases = [
        ("escape", "schema = 1\n[character]\nmodel = \"../aki.gltf\"", "must be relative"),
        ("future", "schema = 2", "schema version 2"),
        ("typo", "schema = 1\n[charater]", "not a valid suite manifest"),
        ("skin", "schema = 1\n[character]\ndefault_skin = \"gown\"", "default skin \"gown\""),
        ("camera", "schema = 1\n[camera]\ndefault = \"drone\"", "default camera \"drone\""),
        ("env", "schema = 1\n[environment]\ndefault = \"moon\"", "default environment \"moon\""),
        ("env-entry", "schema = 1\n[environments.moon]\nshadows = false", "environment \"moon\""),
        ("lighting", "schema = 1\n[lighting.sun]\nilluminance = 1.0", "not a valid suite manifest"),
        ("skins", "schema = 1\n[skins.casual]", "not a valid suite manifest"),
    ];
    for (name, manifest, expected) in cases {
        let err = Fixture::new(name, manifest).load().unwrap_err();
        assert!(err.to_string().contains(expected), "{name}: {err}");
    }
}

#[test]
fn model_meshes_are_reported_because_no_skin_can_hide_them() {
    let fixture = Fixture::new("model-mesh", "schema = 1");
    fixture.write("aki.gltf", &outfit(0.5));
    let suite = fixture.load().unwrap();

    let warnings = messages(&suite, Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("shown under every skin")), "{warnings:#?}");
}

#[test]
fn a_suite_with_nothing_to_show_is_an_error() {
    let fixture = Fixture::new("empty", "schema = 1");
    fs::remove_dir_all(fixture.0.join("skins")).unwrap();
    let suite = fixture.load().unwrap();

    let errors = messages(&suite, Severity::Error);
    assert!(errors.iter().any(|e| e.contains("nothing to show")), "{errors:#?}");
}

#[test]
fn two_models_next_to_the_manifest_are_ambiguous() {
    let fixture = Fixture::new("two-models", "schema = 1");
    fixture.write("spare.glb", "unread");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("the suite folder holds several"), "{err}");

    fixture.write("suite.toml", "schema = 1\n[character]\nmodel = \"aki.gltf\"");
    assert_eq!(fixture.load().unwrap().model, Path::new("aki.gltf"));
}

#[test]
fn cameras_are_found_and_orbit_stays_the_default() {
    let fixture = Fixture::new("cameras", "schema = 1");
    let suite = fixture.load().unwrap();
    let names: Vec<&str> = suite.cameras.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["closeup", "wide"]);
    assert_eq!(suite.cameras[0].file, Path::new("cameras/closeup.gltf"));
    assert_eq!(suite.default_camera, None);

    fixture.write("suite.toml", "schema = 1\n[camera]\ndefault = \"closeup\"");
    assert_eq!(fixture.load().unwrap().default_camera.as_deref(), Some("closeup"));

    fixture.write("suite.toml", "schema = 1\n[camera]\ndefault = \"orbit\"");
    assert_eq!(fixture.load().unwrap().default_camera, None);
}

#[test]
fn a_camera_file_cannot_take_the_orbit_cameras_name() {
    let fixture = Fixture::new("orbit-file", "schema = 1");
    fixture.write("cameras/orbit.glb", "unread");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("reserved name"), "{err}");
}

#[test]
fn camera_files_hold_one_camera_and_at_most_one_clip() {
    let fixture = Fixture::new("camera-checks", "schema = 1");
    let suite = fixture.load().unwrap();
    assert!(messages(&suite, Severity::Error).is_empty());

    fixture.write("cameras/closeup.gltf", &camera(&["Dolly", "Pan"]));
    fixture.write("cameras/wide.gltf", &model());
    let suite = fixture.load().unwrap();
    let errors = messages(&suite, Severity::Error);
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(errors.iter().any(|e| e.contains("holds 2 clips (Dolly, Pan)")));
    assert!(errors.iter().any(|e| e.contains("wide.gltf: holds no camera")));
}

#[test]
fn animation_pointer_clips_get_a_readable_error() {
    let fixture = Fixture::new("pointer", "schema = 1");
    let zoom = camera(&["Zoom"]).replace(
        r#""target": {"node": 0, "path": "translation"}"#,
        r#""target": {"path": "pointer", "extensions": {"KHR_animation_pointer":
            {"pointer": "/cameras/0/perspective/yfov"}}}"#,
    );
    fixture.write("cameras/closeup.gltf", &zoom);
    let suite = fixture.load().unwrap();

    let errors = messages(&suite, Severity::Error);
    assert!(errors.iter().any(|e| e.contains("KHR_animation_pointer")), "{errors:#?}");
}

#[test]
fn environments_come_from_scenes_and_map_folders() {
    let manifest =
        "schema = 1\n[environment]\ndefault = \"room\"\n[environments.room]\nexposure = 2.0";
    let fixture = Fixture::new("environments", manifest);
    fixture.write("environment/room.gltf", &camera(&[]));
    fixture.write("environment/room/diffuse.ktx2", ktx2(6, 1, 0));
    fixture.write("environment/room/specular.ktx2", ktx2(6, 8, 2));
    fixture.write("environment/sky/skybox.ktx2", ktx2(6, 1, 0));
    fixture.write("environment/textures/wall.png", "not an environment");
    let suite = fixture.load().unwrap();

    let names: Vec<&str> = suite.environments.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["room", "sky", "stage"]);
    assert_eq!(suite.default_environment.as_deref(), Some("room"));
    let room = &suite.environments[0];
    assert_eq!(room.settings.exposure, Some(2.0));
    assert!(room.reflections().is_some());
    assert_eq!(room.sky(), Some(Path::new("environment/room/specular.ktx2")));
    let sky = &suite.environments[1];
    assert_eq!(
        (sky.scene.as_ref(), sky.sky()),
        (None, Some(Path::new("environment/sky/skybox.ktx2")))
    );

    assert!(messages(&suite, Severity::Error).is_empty());
    let warnings = messages(&suite, Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("room.gltf: 1 camera(s) will be ignored")));
    assert!(warnings.iter().any(|w| w.contains("\"sky\" has no lights and no reflection maps")));
    assert!(!warnings.iter().any(|w| w.contains("\"room\" has no lights")), "{warnings:#?}");
}

#[test]
fn environment_maps_are_inspected() {
    let fixture = Fixture::new("environment-maps", "schema = 1");
    fixture.write("environment/room/diffuse.ktx2", ktx2(6, 1, 0));
    let warnings = messages(&fixture.load().unwrap(), Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("reflections need both")), "{warnings:#?}");

    fixture.write("environment/room/specular.ktx2", ktx2(6, 1, 0));
    let warnings = messages(&fixture.load().unwrap(), Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("has a single level")), "{warnings:#?}");

    let cases = [
        (ktx2(1, 1, 0), "has 1 face(s)"),
        (ktx2(6, 1, 1), "BasisLZ"),
        (b"not a texture at all, just some words".repeat(2), "not a KTX2 texture"),
    ];
    for (bytes, expected) in cases {
        fixture.write("environment/room/diffuse.ktx2", bytes);
        let errors = messages(&fixture.load().unwrap(), Severity::Error);
        assert!(errors.iter().any(|e| e.contains(expected)), "{expected}: {errors:#?}");
    }
}

#[test]
fn a_suite_without_environments_is_warned_about() {
    let fixture = Fixture::new("no-environment", "schema = 1");
    fs::remove_dir_all(fixture.0.join("environment")).unwrap();
    let warnings = messages(&fixture.load().unwrap(), Severity::Warning);
    assert!(warnings.iter().any(|w| w.contains("no environments")), "{warnings:#?}");
}

#[test]
fn a_panorama_makes_an_environment_whose_maps_are_baked_later() {
    let fixture = Fixture::new("panorama", "schema = 1");
    fixture.write("environment/dusk/sky.hdr", "read only when baking");
    let suite = fixture.load().unwrap();

    let dusk = suite.environments.iter().find(|e| e.name == "dusk").unwrap();
    assert_eq!(dusk.panorama.as_deref(), Some(Path::new("environment/dusk/sky.hdr")));
    assert!(dusk.needs_baking());
    let warnings = messages(&suite, Severity::Warning);
    assert!(!warnings.iter().any(|w| w.contains("\"dusk\"")), "{warnings:#?}");

    fixture.write("environment/dusk/other.exr", "a second panorama");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("several panoramas"), "{err}");
}
