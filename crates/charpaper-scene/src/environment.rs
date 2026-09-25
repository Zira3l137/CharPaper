//! The environment around the character, one at a time: loaded when picked
//! and unloaded when left.
//!
//! An environment is also everything that lights the character. Its `.glb`
//! brings the scene and its lights, and its maps become the camera's skybox
//! and reflections. Environments can be whole rooms, so unlike skins only the
//! one on screen is kept in memory.

use bevy::gltf::GltfLoaderSettings;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::OrbitCamera;
use crate::SceneConfig;
use crate::character::CharacterState;
use crate::character::prefer;
use crate::suite::ActiveSuite;
use crate::suite::asset_path;

/// The environment on screen, as opposed to [`CharacterState::environment`],
/// the one asked for.
#[derive(Resource, Default)]
pub(crate) struct ShownEnvironment {
    pub name: Option<String>,
    /// `None` for a sky-only environment, which has no scene.
    pub root: Option<Entity>,
}

#[derive(Component)]
pub(crate) struct EnvironmentRoot {
    file: Handle<Gltf>,
    clips: Vec<Handle<AnimationClip>>,
}

/// The file is still loading; the scene is spawned once it arrives.
#[derive(Component)]
pub(crate) struct AwaitingScene;

/// The scene has spawned, so its lights exist and can take the look's
/// shadow setting.
#[derive(Component)]
pub(crate) struct EnvironmentReady;

pub(crate) fn choose_environment(
    suite: Option<Res<ActiveSuite>>,
    config: Res<SceneConfig>,
    mut state: ResMut<CharacterState>,
) {
    let Some(suite) = suite else {
        return;
    };
    state.environment = prefer(
        suite.remembered(&config).environment,
        |name| suite.environments.iter().any(|e| e.name == name),
        suite.default_environment.clone(),
    );
}

pub(crate) fn switch_environment(
    mut commands: Commands,
    state: Res<CharacterState>,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownEnvironment>,
    camera: Query<Entity, With<OrbitCamera>>,
) {
    if shown.name == state.environment {
        return;
    }
    let (Some(suite), Ok(camera)) = (suite, camera.single()) else {
        return;
    };

    if let Some(root) = shown.root.take() {
        commands.entity(root).despawn();
    }
    commands.entity(camera).remove::<(Skybox, EnvironmentMapLight)>();
    shown.name = state.environment.clone();

    let Some(environment) = state
        .environment
        .as_ref()
        .and_then(|name| suite.environments.iter().find(|e| &e.name == name))
    else {
        return;
    };

    // Brightness is left at zero here; `apply_look` sets it from the look
    // straight after, as it does whenever the look changes.
    if let Some(sky) = environment.sky() {
        let image = assets.load(asset_path(&suite, sky));
        commands.entity(camera).insert(Skybox { image: Some(image), brightness: 0.0, ..default() });
    }
    if let Some((diffuse, specular)) = environment.reflections() {
        commands.entity(camera).insert(EnvironmentMapLight {
            diffuse_map: assets.load(asset_path(&suite, diffuse)),
            specular_map: assets.load(asset_path(&suite, specular)),
            intensity: 0.0,
            ..default()
        });
    }
    if let Some(scene) = &environment.scene {
        // Loaded whole, like a camera rig, because its clips are needed too.
        // Its lights are the point: they are all the lighting there is.
        let file = assets
            .load_builder()
            .with_settings(|s: &mut GltfLoaderSettings| s.load_cameras = false)
            .load(asset_path(&suite, scene));
        let root = commands
            .spawn((
                Name::new(format!("Environment {}", environment.name)),
                EnvironmentRoot { file, clips: Vec::new() },
                AwaitingScene,
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        shown.root = Some(root);
    }
    info!("environment {:?}", environment.name);
}

pub(crate) fn spawn_environment_scenes(
    mut commands: Commands,
    roots: Query<(Entity, &mut EnvironmentRoot), With<AwaitingScene>>,
    gltfs: Res<Assets<Gltf>>,
    assets: Res<AssetServer>,
) {
    for (entity, mut root) in roots {
        if assets.load_state(root.file.id()).is_failed() {
            warn!("the environment's file failed to load");
            commands.entity(entity).remove::<AwaitingScene>();
            continue;
        }
        let Some(gltf) = gltfs.get(&root.file) else {
            continue;
        };
        commands.entity(entity).remove::<AwaitingScene>();
        let Some(scene) = gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned())
        else {
            warn!("the environment's file holds no scene");
            continue;
        };
        root.clips = gltf.animations.clone();
        commands.entity(entity).insert(WorldAssetRoot(scene));
    }
}

/// Loops every clip in the file. Bevy's loader has already put a player on
/// the root of each animated hierarchy; giving every player every clip is
/// safe, since a clip only moves the nodes that player drives.
pub(crate) fn on_environment_ready(
    ready: On<WorldInstanceReady>,
    roots: Query<&EnvironmentRoot>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    let Ok(root) = roots.get(ready.entity) else {
        return;
    };
    commands.entity(ready.entity).insert(EnvironmentReady);
    if root.clips.is_empty() {
        return;
    }

    let (graph, nodes) = AnimationGraph::from_clips(root.clips.iter().cloned());
    let graph = graphs.add(graph);
    for entity in children.iter_descendants(ready.entity) {
        let Ok(mut player) = players.get_mut(entity) else {
            continue;
        };
        commands.entity(entity).insert(AnimationGraphHandle(graph.clone()));
        for &node in &nodes {
            player.play(node).repeat();
        }
    }
    info!("environment: looping {} clip(s)", nodes.len());
}
