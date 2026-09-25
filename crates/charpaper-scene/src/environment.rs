//! The environment around the character, one at a time: loaded when picked
//! and unloaded when left.
//!
//! An environment is also everything that lights the character. Its `.glb`
//! brings the scene and its lights, and its maps become the camera's skybox
//! and reflections. Environments can be whole rooms, so unlike skins only the
//! one on screen is kept in memory.
//!
//! Maps missing next to a panorama are baked the first time the environment
//! is shown, on a background thread: it appears straight away with its scene
//! and whatever maps exist, and gains the rest a few seconds later.

use std::collections::HashMap;

use bevy::gltf::GltfLoaderSettings;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::Task;
use bevy::tasks::block_on;
use bevy::tasks::poll_once;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_suite::Environment;
use charpaper_suite::Suite;

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

/// Bakes running, by environment name. The result is only an error message:
/// on success the maps are simply looked for again on disk.
#[derive(Resource, Default)]
pub(crate) struct Baking(HashMap<String, Task<Result<(), String>>>);

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
    config: Res<SceneConfig>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownEnvironment>,
    mut baking: ResMut<Baking>,
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

    attach_maps(&mut commands, camera, &assets, &suite, environment);
    if environment.needs_baking() && !baking.0.contains_key(&environment.name) {
        start_bake(&mut baking, &suite, environment, &config);
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

/// Gives the camera the environment's sky and reflections, whichever exist.
/// Brightness is left at zero; `apply_look` sets it straight after, as it
/// does whenever the look changes.
fn attach_maps(
    commands: &mut Commands,
    camera: Entity,
    assets: &AssetServer,
    suite: &Suite,
    environment: &Environment,
) {
    if let Some(sky) = environment.sky() {
        let image = assets.load(asset_path(suite, sky));
        commands.entity(camera).insert(Skybox { image: Some(image), brightness: 0.0, ..default() });
    }
    if let Some((diffuse, specular)) = environment.reflections() {
        commands.entity(camera).insert(EnvironmentMapLight {
            diffuse_map: assets.load(asset_path(suite, diffuse)),
            specular_map: assets.load(asset_path(suite, specular)),
            intensity: 0.0,
            ..default()
        });
    }
}

fn start_bake(baking: &mut Baking, suite: &Suite, environment: &Environment, config: &SceneConfig) {
    let Some(panorama) = &environment.panorama else {
        return;
    };
    let panorama = suite.absolute(panorama);
    let folder = suite.absolute(&environment.folder());
    let settings = config.bake.clone();
    info!(
        "baking the missing maps of environment {:?} from {}; it shows without them until done",
        environment.name,
        panorama.display()
    );
    let task = AsyncComputeTaskPool::get().spawn(async move {
        charpaper_bake::bake(&panorama, &folder, &settings).map(drop).map_err(|e| e.to_string())
    });
    baking.0.insert(environment.name.clone(), task);
}

/// Picks up finished bakes: the environment learns about its new maps, and if
/// it is the one on screen, the camera gets them at once.
pub(crate) fn finish_bakes(
    mut commands: Commands,
    mut baking: ResMut<Baking>,
    suite: Option<ResMut<ActiveSuite>>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownEnvironment>,
    camera: Query<Entity, With<OrbitCamera>>,
) {
    let Some(mut suite) = suite else {
        return;
    };
    let mut finished = Vec::new();
    for (name, task) in &mut baking.0 {
        if let Some(result) = block_on(poll_once(task)) {
            finished.push((name.clone(), result));
        }
    }

    for (name, result) in finished {
        baking.0.remove(&name);
        if let Err(err) = result {
            warn!("cannot bake the maps of environment {name:?}: {err}");
            continue;
        }
        let root = suite.root.clone();
        let Some(environment) = suite.0.environments.iter_mut().find(|e| e.name == name) else {
            continue;
        };
        environment.find_maps(&root);
        info!("environment {name:?}: maps baked");

        if shown.name.as_deref() == Some(name.as_str()) {
            if let Ok(camera) = camera.single() {
                let environment = environment.clone();
                attach_maps(&mut commands, camera, &assets, &suite, &environment);
                // So `apply_look` runs and gives the new maps their brightness.
                shown.set_changed();
            }
        }
    }
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
