use std::fs;
use std::path::Path;
use std::path::PathBuf;

use charpaper_suite::ActorClip;
use charpaper_suite::ChainCursor;
use charpaper_suite::ClipMode;
use charpaper_suite::EyeKind;
use charpaper_suite::Suite;
use charpaper_suite::SuiteError;
use charpaper_suite::Trigger;

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
        fixture.write("clips/idle.json", "read by the app");
        fixture.write("clips/idle.glb", "read by the app");
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
    let fixture = Fixture::new("convention", "schema = 2");
    let suite = fixture.load().unwrap();

    assert_eq!(suite.model, Path::new("aki.gltf"));
    assert_eq!(suite.timeline.seconds(), 250.0 / 24.0);
    let idle = &suite.clips[0];
    assert_eq!((suite.clips.len(), idle.name.as_str()), (1, "idle"));
    assert_eq!(idle.json, Path::new("clips/idle.json"));
    assert_eq!(idle.glb.as_deref(), Some(Path::new("clips/idle.glb")));
    assert_eq!((idle.mode, idle.gaze), (ClipMode::Timeline, true));
    assert_eq!(suite.default_clip, None);
    let skins: Vec<&str> = suite.skins.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(skins, ["casual", "dress"]);
    assert_eq!(suite.skins[1].file, Path::new("skins/dress.gltf"));
    assert_eq!(suite.default_skin.as_deref(), Some("casual"));
}

#[test]
fn clips_are_set_by_name_and_settings_for_missing_ones_are_refused() {
    let manifest = r#"
        schema = 2
        [timeline]
        start = 1
        end = 120
        fps = 30
        [character]
        default_clip = "wave"
        [character.clips.wave]
        mode = "once"
        label = "Waving"
    "#;
    let fixture = Fixture::new("listed", manifest);
    fixture.write("clips/wave.json", "{}");
    fixture.write("clips/stray.glb", "no .json beside it");
    let suite = fixture.load().unwrap();

    assert_eq!(suite.timeline.seconds(), 4.0);
    let names: Vec<&str> = suite.clips.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["idle", "wave"]);
    let wave = &suite.clips[1];
    assert_eq!(
        (wave.mode, wave.label.as_deref(), wave.glb.as_ref()),
        (ClipMode::Once, Some("Waving"), None)
    );
    assert_eq!(suite.default_clip.as_deref(), Some("wave"));

    fixture.write("suite.toml", "schema = 2\n[character.clips.run]\nmode = \"hold\"");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("clip \"run\""), "{err}");
}

#[test]
fn cues_name_sounds_in_the_sounds_folder() {
    let manifest = r#"
        schema = 2
        [character.clips.idle]
        cues = [{ at = 0.5, sound = "step" }, { at = 0.0, sound = "theme" }]
    "#;
    let fixture = Fixture::new("cues", manifest);
    fixture.write("sounds/step.ogg", "step");
    fixture.write("sounds/theme.ogg", "theme");
    let suite = fixture.load().unwrap();

    let cues: Vec<(f32, &Path)> =
        suite.clips[0].cues.iter().map(|c| (c.at, c.sound.as_path())).collect();
    assert_eq!(cues, [(0.5, Path::new("sounds/step.ogg")), (0.0, Path::new("sounds/theme.ogg"))]);

    fixture.write("suite.toml", manifest.replace("\"step\"", "\"stomp\""));
    let err = fixture.load().unwrap_err();
    assert!(matches!(&err, SuiteError::Missing(p) if p.ends_with("stomp.ogg")), "{err}");
}

#[test]
fn bad_manifests_are_rejected() {
    let cases = [
        ("escape", "schema = 2\n[character]\nmodel = \"../aki.gltf\"", "must be relative"),
        ("old", "schema = 1", "export the suite again"),
        ("future", "schema = 3", "schema version 3"),
        ("timeline", "schema = 2\n[timeline]\nstart = 10\nend = 5", "timeline from frame 10"),
        ("clip", "schema = 2\n[character]\ndefault_clip = \"run\"", "default clip \"run\""),
        ("expression", "schema = 2\n[character.expressions.sad]", "expression \"sad\""),
        ("actor", "schema = 2\n[environments.stage.actors.Door]", "actor \"stage/Door\""),
        ("typo", "schema = 2\n[charater]", "not a valid suite manifest"),
        ("skin", "schema = 2\n[character]\ndefault_skin = \"gown\"", "default skin \"gown\""),
        ("camera", "schema = 2\n[camera]\ndefault = \"drone\"", "default camera \"drone\""),
        ("env", "schema = 2\n[environment]\ndefault = \"moon\"", "default environment \"moon\""),
        ("env-entry", "schema = 2\n[environments.moon]\nshadows = false", "environment \"moon\""),
        ("cam-entry", "schema = 2\n[cameras.drone]\nf_stop = 2.0", "camera \"drone\""),
        ("lighting", "schema = 2\n[lighting.sun]\nilluminance = 1.0", "not a valid suite manifest"),
        ("skins", "schema = 2\n[skins.casual]", "not a valid suite manifest"),
    ];
    for (name, manifest, expected) in cases {
        let err = Fixture::new(name, manifest).load().unwrap_err();
        assert!(err.to_string().contains(expected), "{name}: {err}");
    }
}

#[test]
fn two_models_next_to_the_manifest_are_ambiguous() {
    let fixture = Fixture::new("two-models", "schema = 2");
    fixture.write("spare.glb", "unread");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("the suite folder holds several"), "{err}");

    fixture.write("suite.toml", "schema = 2\n[character]\nmodel = \"aki.gltf\"");
    assert_eq!(fixture.load().unwrap().model, Path::new("aki.gltf"));
}

#[test]
fn cameras_are_found_and_orbit_stays_the_default() {
    let fixture = Fixture::new("cameras", "schema = 2");
    let suite = fixture.load().unwrap();
    let names: Vec<&str> = suite.cameras.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["closeup", "wide"]);
    assert_eq!(suite.cameras[0].file, Path::new("cameras/closeup.gltf"));
    assert_eq!(suite.default_camera, None);

    fixture.write("suite.toml", "schema = 2\n[camera]\ndefault = \"closeup\"");
    assert_eq!(fixture.load().unwrap().default_camera.as_deref(), Some("closeup"));

    fixture.write("suite.toml", "schema = 2\n[camera]\ndefault = \"orbit\"");
    assert_eq!(fixture.load().unwrap().default_camera, None);
}

#[test]
fn a_camera_file_cannot_take_the_orbit_cameras_name() {
    let fixture = Fixture::new("orbit-file", "schema = 2");
    fixture.write("cameras/orbit.glb", "unread");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("reserved name"), "{err}");
}

#[test]
fn lens_settings_reach_their_camera_unless_they_cannot_work() {
    let manifest = r#"schema = 2
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
    let fixture = Fixture::new("luts", "schema = 2\n[post.lut]\nname = \"teal\"");
    fixture.write("luts/teal.cube", "LUT_3D_SIZE 2\n0 0 0\n");
    fixture.write("luts/warm.cube", "LUT_1D_SIZE 2\n0 0 0\n1 1 1\n");
    fixture.write("luts/readme.txt", "not a LUT");
    let suite = fixture.load().unwrap();
    let names: Vec<&str> = suite.luts.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["teal", "warm"]);
    assert_eq!(suite.post.lut.name(), Some("teal"));

    fixture.write("suite.toml", "schema = 2\n[post.lut]\nname = \"none\"");
    assert_eq!(fixture.load().unwrap().post.lut.name(), None);

    fixture.write("luts/none.cube", "LUT_1D_SIZE 2\n0 0 0\n1 1 1\n");
    assert!(fixture.load().unwrap_err().to_string().contains("LUT \"none\" uses a reserved name"));
}

#[test]
fn environments_come_from_scenes_and_map_folders() {
    let manifest =
        "schema = 2\n[environment]\ndefault = \"room\"\n[environments.room]\nexposure = 2.0";
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
    let fixture = Fixture::new("environment-sound", "schema = 2");
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
    let fixture = Fixture::new("environment-gifs", "schema = 2");
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
fn environment_settings_hold_fog_lights_and_light_lists() {
    let manifest = r#"schema = 2
[environments.foggy]
no_volume_scatter = ["Lamp"]
[environments.foggy.fog.Mist]
density = 0.2
[environments.foggy.lights.Lamp]
radius = 0.1
"#;
    let fixture = Fixture::new("fog", manifest);
    fixture.write("environment/foggy.gltf", &foggy_scene());
    let suite = fixture.load().unwrap();
    let foggy = &suite.environments[0];
    assert_eq!(foggy.settings.fog["Mist"].density, Some(0.2));
    assert_eq!(foggy.settings.no_volume_scatter, ["Lamp"]);
    assert_eq!(foggy.settings.lights["Lamp"].radius, Some(0.1));
}

#[test]
fn environment_actors_are_folders_of_clips() {
    let manifest = r#"schema = 2
[environments.stage.actors.Door.clips.slam]
mode = "once"
"#;
    let fixture = Fixture::new("actors", manifest);
    fixture.write("environment/stage/clips/Door/slam.json", "{}");
    fixture.write("environment/stage/clips/Door/slam.glb", "glb");
    fixture.write("environment/stage/clips/Fan/spin.json", "{}");
    let suite = fixture.load().unwrap();

    let stage = &suite.environments[0];
    let actors: Vec<&str> = stage.actors.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(actors, ["Door", "Fan"]);
    let slam = &stage.actors[0].clips[0];
    assert_eq!((slam.name.as_str(), slam.mode), ("slam", ClipMode::Once));
    assert_eq!(slam.glb.as_deref(), Some(Path::new("environment/stage/clips/Door/slam.glb")));
    assert_eq!(stage.actors[1].clips[0].mode, ClipMode::Timeline);
    assert!(stage.settings.actors.is_empty(), "moved out of the environment's look");

    fixture.write("suite.toml", "schema = 2\n[environments.stage.actors.Door.clips.open]");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("clip \"stage/Door/open\""), "{err}");

    fixture.write("environment/stage/clips/Door/closed.json", "{}");
    let picked = format!("{manifest}[environments.stage.actors.Door]\ndefault_clip = \"closed\"\n");
    fixture.write("suite.toml", &picked);
    let suite = fixture.load().unwrap();
    assert_eq!(suite.environments[0].actors[0].default_clip.as_deref(), Some("closed"));
    let once = format!("{manifest}[environments.stage.actors.Door]\ndefault_clip = \"slam\"\n");
    fixture.write("suite.toml", &once);
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("default timeline or hold clip \"stage/Door/slam\""), "{err}");
}

#[test]
fn a_panorama_makes_an_environment_whose_maps_are_baked_later() {
    let fixture = Fixture::new("panorama", "schema = 2");
    fixture.write("environment/dusk/sky.hdr", "read only when baking");
    let suite = fixture.load().unwrap();

    let dusk = suite.environments.iter().find(|e| e.name == "dusk").unwrap();
    assert_eq!(dusk.panorama.as_deref(), Some(Path::new("environment/dusk/sky.hdr")));
    assert!(dusk.needs_baking());

    fixture.write("environment/dusk/other.exr", "a second panorama");
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("several panoramas"), "{err}");
}

fn face() -> String {
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
        {ACCESSORS}}}"#
    )
}

#[test]
fn expressions_belong_to_the_character() {
    let manifest = "schema = 2\n[character.expressions.happy]\nlabel = \"Happy\"";
    let fixture = Fixture::new("expressions", manifest);
    fixture.write("expressions/happy.json", "{}");
    fixture.write("expressions/sleepy.json", "{}");
    let suite = fixture.load().unwrap();

    let found: Vec<(&str, Option<&str>)> =
        suite.expressions.iter().map(|e| (e.name.as_str(), e.label.as_deref())).collect();
    assert_eq!(found, [("happy", Some("Happy")), ("sleepy", None)]);
    assert_eq!(suite.expressions[1].json, Path::new("expressions/sleepy.json"));
}

#[test]
fn without_a_model_file_the_armature_comes_from_the_default_skin() {
    let fixture = Fixture::new("no-model", "schema = 2\n[character]\ndefault_skin = \"dress\"");
    fs::remove_file(fixture.0.join("aki.gltf")).unwrap();
    let suite = fixture.load().unwrap();
    assert!(suite.model_is_skin);
    assert_eq!(suite.model, Path::new("skins/dress.gltf"));

    fs::remove_dir_all(fixture.0.join("skins")).unwrap();
    fixture.write("suite.toml", "schema = 2");
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
fn a_camera_file_may_hold_several_cameras_and_a_glb_one_a_clip() {
    let fixture = Fixture::new("camera-pair", "schema = 2");
    fixture.write("cameras/pair.gltf", &camera_pair());
    let suite = fixture.load().unwrap();

    let names: Vec<&str> = suite.cameras.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Front", "Side", "closeup", "wide"]);
    let front = &suite.cameras[0];
    assert_eq!(front.file, Path::new("cameras/pair.gltf"));
    assert_eq!(front.node.as_deref(), Some("Front"));
    assert_eq!((front.clip.as_ref(), suite.cameras[2].node.as_ref()), (None, None));

    // A .gltf can't be a clip's .glb, so only a .glb camera file moves.
    fixture.write("cameras/pair.json", "{}");
    assert_eq!(fixture.load().unwrap().cameras[0].clip, None);
    fixture.write("cameras/dolly.glb", "read by the app");
    fixture.write("cameras/dolly.json", "{}");
    let suite = fixture.load().unwrap();
    let dolly = suite.cameras.iter().find(|c| c.name == "dolly").unwrap();
    assert_eq!(dolly.clip.as_deref(), Some(Path::new("cameras/dolly.json")));

    fixture.write("cameras/Front.gltf", &camera(&[]));
    let err = fixture.load().unwrap_err();
    assert!(err.to_string().contains("camera \"Front\" is named twice"), "{err}");
}

#[test]
fn gaze_names_bones_of_the_armature_and_optional_shape_keys() {
    let manifest = r#"schema = 2
[gaze]
eyes = ["Head"]
neck = "Spine"
[gaze.shape_keys]
up = "brows_up"
"#;
    let fixture = Fixture::new("gaze", manifest);
    fixture.write("skins/casual.gltf", face());
    let suite = fixture.load().unwrap();
    let gaze = suite.gaze.as_ref().unwrap();
    assert_eq!(gaze.bones().collect::<Vec<_>>(), ["Head", "Spine"]);
    assert_eq!(gaze.eye_limits_deg(), [30.0, 20.0]);

    let keys = charpaper_suite::mesh_shape_keys(&fixture.0.join("skins/casual.gltf")).unwrap();
    assert_eq!(keys[0].node, "Face");
    assert_eq!(keys[0].keys, ["mouth_corner_up", "brows_up"]);
}

#[test]
fn gaze_needs_no_eye_bones() {
    let manifest = "schema = 2\n[gaze]\nhead = \"Head\"\n[gaze.shape_keys]\nleft = \"look_l\"";
    let gaze = Fixture::new("gaze-no-eyes", manifest).load().unwrap().gaze.unwrap();
    assert!(gaze.eyes.is_empty());
    assert_eq!(gaze.bones().collect::<Vec<_>>(), ["Head"]);
    assert_eq!(gaze.head_turn(), 0.6);
}

#[test]
fn eyes_are_eyeballs_unless_said_otherwise() {
    let manifest = |kind: &str| format!("schema = 2\n[gaze]\neyes = [\"Head\"]\n{kind}");
    let load = |name: &str, kind: &str| Fixture::new(name, &manifest(kind)).load();
    assert_eq!(load("gaze-ball", "").unwrap().gaze.unwrap().eye_kind, EyeKind::Ball);
    let drawn = load("gaze-drawn", "eye_kind = \"drawn\"").unwrap().gaze.unwrap();
    assert_eq!(drawn.eye_kind, EyeKind::Drawn);
    assert!(load("gaze-other", "eye_kind = \"glass\"").is_err());
}

#[test]
fn a_clip_may_opt_out_of_gaze() {
    let manifest = "schema = 2\n[character.clips.idle]\ngaze = false";
    let suite = Fixture::new("gaze-opt-out", manifest).load().unwrap();
    assert!(!suite.clips[0].gaze);
}

#[test]
fn a_sun_gets_as_many_cascades_as_the_depth_needs() {
    use charpaper_suite::sun_cascades;
    let cases = [(0.0, 1), (5.0, 1), (10.0, 1), (11.0, 2), (24.0, 2), (25.0, 3), (60.0, 3)];
    for (depth, cascades) in cases.into_iter().chain([(61.0, 4), (150.0, 4), (1000.0, 4)]) {
        assert_eq!(sun_cascades(depth), cascades, "{depth} m");
    }
}

#[test]
fn hotspots_name_clips_and_expressions_that_exist() {
    let manifest = r#"schema = 2
[character.clips.flinch]
mode = "once"
reaction_only = true

[hotspots.Head]
expression = "startled"
cooldown = 1.5
during = [{ clip = "idle" }]
areas = [
    { bone = "Head", length = 0.2, radius = 0.12 },
    { actor = "Door", bone = "Hinge", length = 1.0, radius = 0.1 },
]
clips = [{ clip = "flinch" }, { actor = "Door", clip = "slam" }]

[hotspots.Belly]
trigger = "hover"
layer = true
areas = [{ bone = "Spine", length = 0.3, radius = 0.15 }]
clips = [{ clip = "idle" }]
"#;
    let fixture = Fixture::new("hotspots", manifest);
    fixture.write("clips/flinch.json", "{}");
    fixture.write("expressions/startled.json", "{}");
    fixture.write("environment/stage/clips/Door/slam.json", "{}");
    let suite = fixture.load().unwrap();

    let head = &suite.hotspots["Head"];
    assert_eq!((head.trigger, head.layer, head.cooldown), (Trigger::Click, false, Some(1.5)));
    assert_eq!(head.areas[1].actor.as_deref(), Some("Door"));
    assert_eq!(head.clips[1].clip, "slam");
    assert_eq!(head.during, [ActorClip { actor: None, clip: "idle".into() }]);
    assert!(suite.hotspots["Belly"].during.is_empty());
    assert_eq!(
        (suite.hotspots["Belly"].trigger, suite.hotspots["Belly"].layer),
        (Trigger::Hover, true)
    );
    let flinch = suite.clips.iter().find(|c| c.name == "flinch").unwrap();
    assert!(flinch.reaction_only);
    assert!(!suite.clips.iter().find(|c| c.name == "idle").unwrap().reaction_only);

    let broken = [
        (r#"clips = [{ clip = "wave" }]"#, "the character has no clip \"wave\""),
        (r#"clips = [{ actor = "Door", clip = "open" }]"#, "Door has no clip \"open\""),
        (r#"clips = [{ actor = "Window", clip = "open" }]"#, "Window has no clip"),
        (
            r#"clips = []
expression = "sad""#,
            "no expression \"sad\"",
        ),
        (
            r#"clips = []
during = [{ clip = "lying" }]"#,
            "the character has no clip \"lying\"",
        ),
        (r#"clips = [{ clip = "flinch", from = 0 }]"#, "`from` counts from 1"),
        (
            r#"clips = []
after = 30"#,
            "only `idle` and `welcome` hotspots have `after`",
        ),
    ];
    for (clips, problem) in broken {
        let text = format!(
            "schema = 2\n[hotspots.Hand]\nareas = [{{ bone = \"Hand\", length = 0.1, radius = 0.05 }}]\n{clips}"
        );
        fixture.write("suite.toml", &text);
        let err = fixture.load().unwrap_err().to_string();
        assert!(err.contains("hotspot \"Hand\"") && err.contains(problem), "{err}");
    }
    let flat = "schema = 2\n[hotspots.Hand]\nareas = [{ bone = \"Hand\", length = 0.1, radius = 0.0 }]\nclips = []";
    fixture.write("suite.toml", flat);
    assert!(fixture.load().unwrap_err().to_string().contains("radius above 0"));
    let unclipped = "schema = 2\n[hotspots.Hand]\nareas = [{ actor = \"Lamp\", bone = \"B\", length = 0.1, radius = 0.1 }]\nclips = []";
    fixture.write("suite.toml", unclipped);
    assert!(fixture.load().is_ok(), "an armature needs no clips to carry a hotspot");
}

#[test]
fn idle_and_welcome_hotspots_streaks_and_then() {
    let manifest = r#"schema = 2
[character.clips.flinch]
mode = "once"
reaction_only = true

[character.clips.lie]
mode = "hold"

[environments.stage.actors.Lamp.clips.off]
mode = "hold"

[hotspots.Bored]
trigger = "idle"
clips = [{ clip = "flinch" }, { clip = "flinch", from = 3 }]
while = [{ clip = "lie" }]
then = [{ clip = "lie" }, { actor = "Lamp", clip = "off" }]

[hotspots.Hello]
trigger = "welcome"
after = 20
clips = [{ clip = "flinch" }]
during = [{ clip = "lie" }, { actor = "Lamp", clip = "off" }]
"#;
    let fixture = Fixture::new("idle-hotspots", manifest);
    fixture.write("clips/flinch.json", "{}");
    fixture.write("clips/lie.json", "{}");
    fixture.write("environment/stage/clips/Lamp/off.json", "{}");
    let suite = fixture.load().unwrap();

    let bored = &suite.hotspots["Bored"];
    assert_eq!((bored.trigger, bored.after_secs()), (Trigger::Idle, 60.0));
    assert_eq!(bored.clips[1].from, Some(3));
    assert_eq!(bored.then[1], ActorClip { actor: Some("Lamp".into()), clip: "off".into() });
    assert_eq!(bored.meanwhile, [ActorClip { actor: None, clip: "lie".into() }]);
    let hello = &suite.hotspots["Hello"];
    assert_eq!(
        (hello.trigger, hello.after_secs(), hello.during.len()),
        (Trigger::Welcome, 20.0, 2)
    );

    let broken = [
        (
            r#"then = [{ clip = "flinch" }]"#,
            "`then` names \"flinch\", which doesn't play on and on",
        ),
        (
            r#"then = [{ clip = "idle" }, { clip = "lie" }]"#,
            "`then` names two clips for the character",
        ),
        (r#"then = [{ actor = "Lamp", clip = "on" }]"#, "Lamp has no clip \"on\""),
        (
            r#"while = [{ clip = "flinch" }]"#,
            "`while` names \"flinch\", which doesn't play on and on",
        ),
        ("after = 0", "`after` must be more than 0 seconds"),
        (
            r#"areas = [{ bone = "Hand", length = 0.1, radius = 0.05 }]"#,
            "only `click` and `hover` hotspots have `areas`",
        ),
    ];
    for (extra, problem) in broken {
        let text = format!("{manifest}\n[hotspots.Nap]\ntrigger = \"idle\"\nclips = []\n{extra}\n");
        fixture.write("suite.toml", &text);
        let err = fixture.load().unwrap_err().to_string();
        assert!(err.contains("hotspot \"Nap\"") && err.contains(problem), "{err}");
    }
    let text = format!(
        "{manifest}\n[hotspots.Hi]\ntrigger = \"welcome\"\nclips = []\nwhile = [{{ clip = \"lie\" }}]\n"
    );
    fixture.write("suite.toml", &text);
    let err = fixture.load().unwrap_err().to_string();
    assert!(err.contains("a `welcome` hotspot isn't held"), "{err}");
}

#[test]
fn physics_chains_and_colliders_are_read_with_their_defaults() {
    let manifest = r#"schema = 2

[physics.chains.Ponytail]
bones = ["Ponytail"]
length = 0.3
radius = 0.02
stiffness = 0.05

[physics.chains.Bangs]
bones = ["Bangs"]
length = 0.1
cursor = "only"

[physics.colliders.Head]
bone = "Head"
length = 0.2
radius = 0.1
"#;
    let suite = Fixture::new("physics", manifest).load().unwrap();
    let chain = &suite.physics.chains["Ponytail"];
    assert_eq!(chain.bones, ["Ponytail"]);
    assert_eq!(chain.cursor, ChainCursor::Pushes);
    assert_eq!(suite.physics.chains["Bangs"].cursor, ChainCursor::Only);
    assert_eq!((chain.stiffness, chain.drag, chain.gravity), (Some(0.05), None, None));
    let head = &suite.physics.colliders["Head"];
    assert_eq!((head.bone.as_str(), head.length, head.radius), ("Head", 0.2, 0.1));
    assert!(Fixture::new("no-physics", "schema = 2").load().unwrap().physics.chains.is_empty());
}

#[test]
fn physics_that_cannot_work_is_refused() {
    let broken = [
        ("[physics.chains.Hair]\nbones = []\nlength = 0.1", "has no bones"),
        (
            "[physics.chains.Hair]\nbones = [\"Ponytail\"]\nlength = 0.1\n\
             [physics.chains.Tail]\nbones = [\"Ponytail\"]\nlength = 0.1",
            "shares the bone \"Ponytail\" with \"Hair\"",
        ),
        ("[physics.chains.Hair]\nbones = [\"Ponytail\"]\nlength = 0.1\ndrag = 2", "go from 0 to 1"),
        ("[physics.chains.Hair]\nbones = [\"Ponytail\"]\nlength = -1", "can't be negative"),
        ("[physics.colliders.Head]\nbone = \"Head\"\nlength = 0.1\nradius = 0", "above 0"),
        ("[physics.chains.Hair]\nbones = [\"Ponytail\"]", "not a valid suite manifest"),
    ];
    let fixture = Fixture::new("bad-physics", "schema = 2");
    for (physics, problem) in broken {
        fixture.write("suite.toml", format!("schema = 2\n{physics}\n"));
        let err = fixture.load().unwrap_err().to_string();
        assert!(err.contains(problem), "{physics}: {err}");
    }
}

#[test]
fn an_environment_can_have_wind_that_can_blow() {
    let manifest = "schema = 2\n[environments.stage.wind]\ndirection = [1, 0, 0]\nstrength = 3\n";
    let fixture = Fixture::new("wind", manifest);
    let wind = fixture.load().unwrap().environments[0].settings.wind.unwrap();
    assert_eq!((wind.direction, wind.strength, wind.gusts), ([1.0, 0.0, 0.0], 3.0, None));
    let broken = [
        ("direction = [0, 0, 0]\nstrength = 3", "points somewhere"),
        ("direction = [1, 0, 0]\nstrength = -1", "negative `strength`"),
        ("direction = [1, 0, 0]\nstrength = 1\ngusts = 2", "go from 0 to 1"),
    ];
    for (wind, problem) in broken {
        fixture.write("suite.toml", format!("schema = 2\n[environments.stage.wind]\n{wind}\n"));
        let err = fixture.load().unwrap_err().to_string();
        assert!(err.contains("environment \"stage\": its wind") && err.contains(problem), "{err}");
    }
}
