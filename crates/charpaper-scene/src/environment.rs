// An environment is everything that lights the character: its .glb brings a scene with
// lights, its maps become the camera's sky and reflections. Maps missing next to a
// panorama are baked in the background the first time it shows, and appear when done.

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

use crate::SceneSet;
use crate::assets::GPU_ONLY;
use crate::assets::asset_path;
use crate::camera::SceneCamera;
use crate::config::SceneConfig;
use crate::state::CharacterState;
use crate::state::prefer;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;

pub(crate) struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShownEnvironment>()
            .init_resource::<Baking>()
            .add_observer(on_environment_ready)
            .add_observer(point_sized_spots)
            .add_systems(Update, choose_environment.in_set(SceneSet::Fill))
            .add_systems(
                Update,
                (switch_environment, finish_bakes, spawn_environment_scenes)
                    .chain()
                    .in_set(SceneSet::Run),
            );
    }
}

#[derive(Resource, Default)]
pub(crate) struct ShownEnvironment {
    pub name: Option<String>,
    pub root: Option<Entity>,
}

#[derive(Component)]
pub(crate) struct EnvironmentRoot {
    file: Handle<Gltf>,
    clips: Vec<Handle<AnimationClip>>,
}

#[derive(Component)]
pub(crate) struct AwaitingScene;

#[derive(Resource, Default)]
pub(crate) struct Baking(pub HashMap<String, Task<Result<(), String>>>);

#[derive(Component)]
pub(crate) struct EnvironmentReady;

pub(crate) fn choose_environment(
    suite: Option<Res<ActiveSuite>>,
    remembered: Res<Remembered>,
    mut state: ResMut<CharacterState>,
) {
    let Some(suite) = suite else {
        return;
    };
    state.environment = prefer(
        suite.remembered(&remembered).environment,
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
    camera: Query<Entity, With<SceneCamera>>,
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
        let file = assets
            .load_builder()
            .with_settings(|s: &mut GltfLoaderSettings| {
                s.load_cameras = false;
                s.load_meshes = GPU_ONLY;
                s.load_materials = GPU_ONLY;
            })
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

// Brightness starts at 0; apply_look sets it right after.
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

pub(crate) fn finish_bakes(
    mut commands: Commands,
    mut baking: ResMut<Baking>,
    suite: Option<ResMut<ActiveSuite>>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownEnvironment>,
    camera: Query<Entity, With<SceneCamera>>,
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

// Every player gets every clip. A clip only moves the nodes its own player drives.
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

// Bevy's glTF loader gives a spot light its range as its radius, the size of the bulb. glTF
// lights are points, and a spot with a range of a few meters would shine like a soft panel
// that wide, so it gets its point back.
fn point_sized_spots(
    ready: On<WorldInstanceReady>,
    roots: Query<(), With<EnvironmentRoot>>,
    children: Query<&Children>,
    mut spots: Query<&mut SpotLight>,
) {
    if !roots.contains(ready.entity) {
        return;
    }
    for entity in children.iter_descendants(ready.entity) {
        if let Ok(mut spot) = spots.get_mut(entity) {
            spot.radius = 0.0;
        }
    }
}
