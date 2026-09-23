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
    {"bufferView": 0, "componentType": 5126, "count": 2, "type": "VEC4"}
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
        fixture
    }

    fn write(&self, relative: &str, contents: &str) {
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
