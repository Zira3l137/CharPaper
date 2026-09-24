//! Cameras exported from Blender, one per file in `cameras/`.
//!
//! One lens, many tripods. The app keeps a single real camera: the one that
//! carries the post-processing, the ambient light and the wallpaper's window.
//! Each exported camera is loaded as a rig whose own `Camera` is switched off
//! the moment it spawns, so it never renders. It only carries a transform,
//! animated or not, and a projection for the real camera to copy.
//!
//! A rig's clip loops on its own clock from the moment the rig spawns, whether
//! or not anyone is looking through it.

use bevy::gltf::GltfLoaderSettings;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::suite::ActiveSuite;
use crate::suite::asset_path;

/// Camera files still loading. Removed once every rig has spawned.
#[derive(Resource)]
pub(crate) struct PendingRigs(Vec<(String, Handle<Gltf>)>);

#[derive(Component)]
pub(crate) struct CameraRig {
    pub name: String,
    clip: Option<Handle<AnimationClip>>,
    /// Keeps the file's assets alive for as long as the rig exists.
    _file: Handle<Gltf>,
}

/// The switched-off camera inside a rig.
#[derive(Component)]
pub(crate) struct Lens(pub Entity);

/// Loads each file whole, as a `Gltf`, rather than just its scene like the
/// character's files: the rig needs the file's clip too, and a missing
/// `#Animation0` label would be a load error for every static camera.
pub(crate) fn load_cameras(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
) {
    let Some(suite) = suite else {
        return;
    };
    let rigs = suite
        .cameras
        .iter()
        .map(|camera| {
            let file = assets
                .load_builder()
                .with_settings(|s: &mut GltfLoaderSettings| s.load_lights = false)
                .load(asset_path(&suite, &camera.file));
            (camera.name.clone(), file)
        })
        .collect();
    commands.insert_resource(PendingRigs(rigs));
}

pub(crate) fn spawn_rigs(
    mut commands: Commands,
    pending: Option<ResMut<PendingRigs>>,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
) {
    let Some(mut pending) = pending else {
        return;
    };

    pending.0.retain(|(name, file)| {
        if assets.load_state(file.id()).is_failed() {
            warn!("camera {name:?}: its file failed to load");
            return false;
        }
        let Some(gltf) = gltfs.get(file) else {
            return true;
        };
        let Some(scene) = gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned())
        else {
            warn!("camera {name:?}: its file holds no scene");
            return false;
        };
        commands.spawn((
            Name::new(format!("Camera rig {name}")),
            CameraRig {
                name: name.clone(),
                clip: gltf.animations.first().cloned(),
                _file: file.clone(),
            },
            WorldAssetRoot(scene),
        ));
        false
    });

    if pending.0.is_empty() {
        commands.remove_resource::<PendingRigs>();
    }
}

/// An observer rather than a system, so the rig's camera is switched off in
/// the same frame it spawns. A system would run a frame later and let that
/// camera render one frame over the real one.
pub(crate) fn on_rig_ready(
    ready: On<WorldInstanceReady>,
    rigs: Query<&CameraRig>,
    children: Query<&Children>,
    mut cameras: Query<&mut Camera>,
    mut players: Query<&mut AnimationPlayer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    let Ok(rig) = rigs.get(ready.entity) else {
        return;
    };

    let lenses: Vec<Entity> =
        children.iter_descendants(ready.entity).filter(|&e| cameras.contains(e)).collect();
    for &lens in &lenses {
        if let Ok(mut camera) = cameras.get_mut(lens) {
            camera.is_active = false;
        }
    }

    let Some(&lens) = lenses.first() else {
        warn!("camera {:?}: its file holds no camera", rig.name);
        return;
    };
    if lenses.len() > 1 {
        warn!("camera {:?}: its file holds {} cameras; using the first", rig.name, lenses.len());
    }
    commands.entity(ready.entity).insert(Lens(lens));

    // Unlike the character's, a rig's clip moves nodes of its own file, so
    // Bevy's loader has already marked them and put a player on their root.
    // Only the graph is missing.
    let Some(clip) = &rig.clip else {
        info!("camera {:?} ready, static", rig.name);
        return;
    };
    let Some(root) = children.iter_descendants(ready.entity).find(|&e| players.contains(e)) else {
        warn!("camera {:?}: its clip animates nothing, so it stays static", rig.name);
        return;
    };
    let (graph, nodes) = AnimationGraph::from_clips([clip.clone()]);
    commands.entity(root).insert(AnimationGraphHandle(graphs.add(graph)));
    if let Ok(mut player) = players.get_mut(root) {
        player.play(nodes[0]).repeat();
    }
    info!("camera {:?} ready, looping its clip", rig.name);
}
