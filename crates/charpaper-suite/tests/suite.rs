use std::fs;
use std::path::Path;
use std::path::PathBuf;

use charpaper_suite::ClipSet;
use charpaper_suite::Suite;
use charpaper_suite::SuiteError;

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

fn outfit() -> String {
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scenes": [{{"nodes": [0]}}],
        "nodes": [
            {{"name": "Armature", "children": [1, 5]}},
            {{"name": "Hips", "children": [2]}},
            {{"name": "Spine", "children": [3], "translation": [0, 0.5, 0]}},
            {{"name": "Head", "children": [4, 6]}},
            {{"name": "Ponytail"}},
            {{"name": "Head", "mesh": 0, "skin": 0}},
            {{"name": "Hat", "mesh": 0}}
        ],
        "skins": [{{"joints": [1, 2, 3, 4]}}], {MESH}, {ACCESSORS}}}"#
    )
}

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

fn ktx2(faces: u32, levels: u32, supercompression: u32) -> Vec<u8> {
    let mut bytes = vec![0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n'];
    for word in [0, 1, 64, 64, 0, 0, faces, levels, supercompression] {
        bytes.extend_from_slice(&u32::to_le_bytes(word));
    }
    bytes
}

fn lit_scene() -> String {
    r#"{"asset": {"version": "2.0"}, "scenes": [{"nodes": [0]}],
        "extensionsUsed": ["KHR_lights_punctual"],
        "extensions": {"KHR_lights_punctual": {"lights": [{"type": "directional"}]}},
        "nodes": [{"name": "Sun", "extensions": {"KHR_lights_punctual": {"light": 0}}}]}"#
        .to_string()
}

fn foggy_scene() -> String {
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scenes": [{{"nodes": [0, 1, 2]}}],
        "extensionsUsed": ["KHR_lights_punctual"],
        "extensions": {{"KHR_lights_punctual": {{"lights": [
            {{"type": "directional"}}, {{"type": "point", "name": "Bulb", "range": 4.0}}]}}}},
        "nodes": [
            {{"name": "Mist", "mesh": 0}},
            {{"name": "Sun", "extensions": {{"KHR_lights_punctual": {{"light": 0}}}}}},
            {{"name": "Lamp", "extensions": {{"KHR_lights_punctual": {{"light": 1}}}}}}
        ],
        {MESH}, {ACCESSORS}}}"#
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
        fixture.write("skins/casual.gltf", &outfit());
        fixture.write("skins/dress.gltf", &outfit());
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
}

#[test]
fn cues_name_sounds_in_the_sounds_folder() {
    let manifest = r#"
        schema = 1
        [animations.walk]
        file = "animations/idle.gltf"
        cues = [{ at = 0.5, sound = "step" }, { at = 0.0, sound = "theme" }]
    "#;
    let fixture = Fixture::new("cues", manifest);
    fixture.write("sounds/step.ogg", "step");
    fixture.write("sounds/theme.ogg", "theme");
    let suite = fixture.load().unwrap();

    let ClipSet::Listed(bindings) = &suite.animations[0].clips else { panic!("listed") };
    let cues: Vec<(f32, &Path)> =
        bindings[0].cues.iter().map(|c| (c.at, c.sound.as_path())).collect();
    assert_eq!(cues, [(0.5, Path::new("sounds/step.ogg")), (0.0, Path::new("sounds/theme.ogg"))]);

    fixture.write("suite.toml", manifest.replace("\"step\"", "\"stomp\""));
    let err = fixture.load().unwrap_err();
    assert!(matches!(&err, SuiteError::Missing(p) if p.ends_with("stomp.ogg")), "{err}");
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
        ("cam-entry", "schema = 1\n[cameras.drone]\nf_stop = 2.0", "camera \"drone\""),
        ("lighting", "schema = 1\n[lighting.sun]\nilluminance = 1.0", "not a valid suite manifest"),
        ("skins", "schema = 1\n[skins.casual]", "not a valid suite manifest"),
    ];
    for (name, manifest, expected) in cases {
        let err = Fixture::new(name, manifest).load().unwrap_err();
        assert!(err.to_string().contains(expected), "{name}: {err}");
    }
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
fn lens_settings_reach_their_camera_unless_they_cannot_work() {
    let manifest = r#"schema = 1
[camera]
f_stop = 0.0
[cameras.closeup]
f_stop = 2.8
focus_object = "Head"
[cameras.wide]
f_stop = 4.0
focus_distance = -1.0
"#;
    let suite = Fixture::new("lens", manifest).load().unwrap();
    assert_eq!(suite.camera.f_stop, None);
    let (closeup, wide) = (&suite.cameras[0].settings, &suite.cameras[1].settings);
    assert_eq!((closeup.f_stop, closeup.focus_object.as_deref()), (Some(2.8), Some("Head")));
    assert_eq!((wide.f_stop, wide.focus_distance), (Some(4.0), None));
    assert_eq!(wide.focus_distance(), 10.0);
}

#[test]
fn luts_are_found_and_none_is_a_reserved_name() {
    let fixture = Fixture::new("luts", "schema = 1\n[post.lut]\nname = \"teal\"");
    fixture.write("luts/teal.cube", "LUT_3D_SIZE 2\n0 0 0\n");
    fixture.write("luts/warm.cube", "LUT_1D_SIZE 2\n0 0 0\n1 1 1\n");
    fixture.write("luts/readme.txt", "not a LUT");
    let suite = fixture.load().unwrap();
    let names: Vec<&str> = suite.luts.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["teal", "warm"]);
    assert_eq!(suite.post.lut.name(), Some("teal"));

    fixture.write("suite.toml", "schema = 1\n[post.lut]\nname = \"none\"");
    assert_eq!(fixture.load().unwrap().post.lut.name(), None);

    fixture.write("luts/none.cube", "LUT_1D_SIZE 2\n0 0 0\n1 1 1\n");
    assert!(fixture.load().unwrap_err().to_string().contains("LUT \"none\" uses a reserved name"));
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
}

#[test]
fn environment_folders_bring_their_music_and_ambience() {
    let fixture = Fixture::new("environment-sound", "schema = 1");
    fixture.write("environment/room.gltf", &camera(&[]));
    fixture.write("environment/room/ambience.ogg", "rain");
    fixture.write("environment/room/music/02_night.ogg", "second");
    fixture.write("environment/room/music/01_day.ogg", "first");
    fixture.write("environment/room/music/cover.png", "not music");
    fixture.write("environment/silent/music/01_alone.ogg", "music alone is no environment");
    let suite = fixture.load().unwrap();

    let names: Vec<&str> = suite.environments.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["room", "stage"]);
    let room = &suite.environments[0];
    assert_eq!(room.ambience.as_deref(), Some(Path::new("environment/room/ambience.ogg")));
    assert_eq!(
        room.music,
        [
            PathBuf::from("environment/room/music/01_day.ogg"),
            PathBuf::from("environment/room/music/02_night.ogg")
        ]
    );
    let stage = &suite.environments[1];
    assert!(stage.music.is_empty() && stage.ambience.is_none());
}

#[test]
fn environment_gifs_are_found_by_material_name() {
    let fixture = Fixture::new("environment-gifs", "schema = 1");
    let scene = r#"{"asset": {"version": "2.0"}, "scenes": [{"nodes": []}],
        "materials": [{"name": "TV screen"}, {"name": "Wall"}]}"#;
    fixture.write("environment/room.gltf", scene);
    fixture.write("environment/room/animated/TV screen.gif", "GIF89a and the rest");
    fixture.write("environment/room/animated/Poster.gif", "GIF87a and the rest");
    fixture.write("environment/room/animated/Wall.gif", "a PNG in disguise");
    fixture.write("environment/room/animated/notes.txt", "not a GIF");
    let suite = fixture.load().unwrap();

    let room = &suite.environments[0];
    let materials: Vec<&str> = room.animated.keys().map(String::as_str).collect();
    assert_eq!(materials, ["Poster", "TV screen", "Wall"]);
    assert_eq!(room.animated["TV screen"], Path::new("environment/room/animated/TV screen.gif"));
}

#[test]
fn environment_settings_hold_fog_and_light_lists() {
    let manifest = r#"schema = 1
[environments.foggy]
no_volume_scatter = ["Lamp"]
[environments.foggy.fog.Mist]
density = 0.2
"#;
    let fixture = Fixture::new("fog", manifest);
    fixture.write("environment/foggy.gltf", &foggy_scene());
    let suite = fixture.load().unwrap();
    let foggy = &suite.environments[0];
    assert_eq!(foggy.settings.fog["Mist"].density, Some(0.2));
    assert_eq!(foggy.settings.no_volume_scatter, ["Lamp"]);
}

#[test]
fn a_panorama_makes_an_environment_whose_maps_are_baked_later() {
    let fixture = Fixture::new("panorama", "schema = 1");
    fixture.write("environment/dusk/sky.hdr", "read only when baking");
    let suite = fixture.load().unwrap();

    let dusk = suite.environments.iter().find(|e| e.name == "dusk").unwrap();
    assert_eq!(dusk.panorama.as_deref(), Some(Path::new("environment/dusk/sky.hdr")));
    assert!(dusk.needs_baking());

    fixture.write("environment/dusk/other.exr", "a second panorama");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("several panoramas"), "{err}");
}

fn face(clips: &str) -> String {
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scenes": [{{"nodes": [0]}}],
        "nodes": [
            {{"name": "Armature", "children": [1, 4]}},
            {{"name": "Hips", "children": [2]}},
            {{"name": "Spine", "children": [3], "translation": [0, 0.5, 0]}},
            {{"name": "Head"}},
            {{"name": "Face", "mesh": 0, "skin": 0}}
        ],
        "skins": [{{"joints": [1, 2, 3]}}],
        "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}},
            "targets": [{{"POSITION": 0}}, {{"POSITION": 0}}]}}],
            "extras": {{"targetNames": ["mouth_corner_up", "brows_up"]}}}}],
        "animations": [{clips}],
        {ACCESSORS}}}"#
    )
}

fn clip(name: &str, node: u32, path: &str) -> String {
    format!(
        r#"{{"name": "{name}",
            "channels": [{{"sampler": 0, "target": {{"node": {node}, "path": "{path}"}}}}],
            "samplers": [{{"input": 1, "output": 1}}]}}"#
    )
}

#[test]
fn a_skins_clips_that_key_only_shape_keys_are_its_expressions() {
    let fixture = Fixture::new("expressions", "schema = 1");
    let clips =
        [clip("Smile", 4, "weights"), clip("Angry", 4, "weights"), clip("Nod", 3, "rotation")];
    fixture.write("skins/casual.gltf", face(&clips.join(", ")));

    let found = charpaper_suite::skin_clips(&fixture.0.join("skins/casual.gltf")).unwrap();
    assert_eq!(found.expressions, ["Angry", "Smile"]);
    assert_eq!(found.expression_meshes.into_iter().collect::<Vec<_>>(), ["Face"]);
}

#[test]
fn without_a_model_file_the_armature_comes_from_the_default_skin() {
    let fixture = Fixture::new("no-model", "schema = 1\n[character]\ndefault_skin = \"dress\"");
    fs::remove_file(fixture.0.join("aki.gltf")).unwrap();
    let suite = fixture.load().unwrap();
    assert!(suite.model_is_skin);
    assert_eq!(suite.model, Path::new("skins/dress.gltf"));

    fs::remove_dir_all(fixture.0.join("skins")).unwrap();
    fixture.write("suite.toml", "schema = 1");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("no model and no skin"), "{err}");
}

fn camera_pair() -> String {
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scenes": [{{"nodes": [0, 2]}}],
        "nodes": [
            {{"name": "DollyA", "children": [1]}},
            {{"name": "Front", "camera": 0}},
            {{"name": "DollyB", "children": [3]}},
            {{"name": "Side", "camera": 0}}
        ],
        "cameras": [{{"type": "perspective", "perspective": {{"yfov": 0.6, "znear": 0.1}}}}],
        "animations": [
            {{"name": "SideSway",
              "channels": [{{"sampler": 0, "target": {{"node": 2, "path": "translation"}}}}],
              "samplers": [{{"input": 1, "output": 3}}]}},
            {{"name": "FrontPush",
              "channels": [{{"sampler": 0, "target": {{"node": 1, "path": "translation"}}}}],
              "samplers": [{{"input": 1, "output": 3}}]}}
        ],
        {ACCESSORS}}}"#
    )
}

#[test]
fn a_camera_file_may_hold_several_cameras_each_with_its_clip() {
    let fixture = Fixture::new("camera-pair", "schema = 1");
    fixture.write("cameras/pair.gltf", &camera_pair());
    let suite = fixture.load().unwrap();

    let names: Vec<&str> = suite.cameras.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Front", "Side", "closeup", "wide"]);
    let front = &suite.cameras[0];
    assert_eq!(front.file, Path::new("cameras/pair.gltf"));
    assert_eq!(front.node.as_deref(), Some("Front"));
    assert_eq!(front.clip.as_deref(), Some("FrontPush"));
    assert_eq!(suite.cameras[1].clip.as_deref(), Some("SideSway"));
    assert_eq!(suite.cameras[2].node, None);

    fixture.write("cameras/Front.gltf", &camera(&[]));
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("camera \"Front\" is named twice"), "{err}");
}

#[test]
fn gaze_names_bones_of_the_armature_and_optional_shape_keys() {
    let manifest = r#"schema = 1
[gaze]
eyes = ["Head"]
neck = "Spine"
[gaze.shape_keys]
up = "brows_up"
"#;
    let fixture = Fixture::new("gaze", manifest);
    fixture.write("skins/casual.gltf", face(""));
    let suite = fixture.load().unwrap();
    let gaze = suite.gaze.as_ref().unwrap();
    assert_eq!(gaze.bones().collect::<Vec<_>>(), ["Head", "Spine"]);
    assert_eq!(gaze.eye_limits_deg(), [30.0, 20.0]);

    let keys = charpaper_suite::mesh_shape_keys(&fixture.0.join("skins/casual.gltf")).unwrap();
    assert_eq!(keys[0].node, "Face");
    assert_eq!(keys[0].keys, ["mouth_corner_up", "brows_up"]);
}

#[test]
fn an_animation_may_opt_out_of_gaze() {
    let manifest = "schema = 1\n[animations.idle]\nfile = \"animations/idle.gltf\"\ngaze = false";
    let suite = Fixture::new("gaze-opt-out", manifest).load().unwrap();
    let ClipSet::Listed(bindings) = &suite.animations[0].clips else { panic!("listed") };
    assert!(!bindings[0].gaze);
}

#[test]
fn a_sun_gets_as_many_cascades_as_the_depth_needs() {
    use charpaper_suite::sun_cascades;
    let cases = [(0.0, 1), (5.0, 1), (10.0, 1), (11.0, 2), (24.0, 2), (25.0, 3), (60.0, 3)];
    for (depth, cascades) in cases.into_iter().chain([(61.0, 4), (150.0, 4), (1000.0, 4)]) {
        assert_eq!(sun_cascades(depth), cascades, "{depth} m");
    }
}
