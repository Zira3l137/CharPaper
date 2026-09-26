//! Cameras exported from Blender, one per file in `cameras/`.
//!
//! One lens, many tripods. The app keeps a single real camera: the one that
//! carries the post-processing, the ambient light and the wallpaper's window.
//! Each exported camera is loaded as a rig whose own `Camera` is switched off
//! the moment it spawns, so it never renders. It only carries a transform,
//! animated or not, and a projection for the real camera to copy.
//!
//! Only the rig being looked through is loaded. Its clip loops on its own
//! clock from the moment it spawns, so switching back to a camera starts its
//! clip from the beginning.

use bevy::gltf::GltfLoaderSettings;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_suite::ORBIT_CAMERA;

use crate::OrbitCamera;
use crate::SceneConfig;
use crate::character::CharacterState;
use crate::character::prefer;
use crate::suite::ActiveSuite;
use crate::suite::asset_path;
use crate::update_camera_transform;

/// The rig in use, as opposed to [`CharacterState::camera`], the one asked
/// for. Only that one rig is loaded; the others are not in memory at all.
#[derive(Resource, Default)]
pub(crate) struct ShownRig {
    name: Option<String>,
    root: Option<Entity>,
    /// Its file while it loads; the rig spawns once it has.
    loading: Option<Handle<Gltf>>,
}

#[derive(Component)]
pub(crate) struct CameraRig {
    pub name: String,
    clip: Option<Handle<AnimationClip>>,
    /// Keeps the file's assets alive for as long as the rig exists, and no
    /// longer.
    _file: Handle<Gltf>,
}

/// The switched-off camera inside a rig.
#[derive(Component)]
pub(crate) struct Lens(pub Entity);

/// On the real camera: which rig it is currently copying, `None` for the orbit
/// camera. Kept apart from [`CharacterState::camera`] so a switch can be told
/// from an ordinary frame, and so a rig still loading reads as "not yet".
#[derive(Component, Default)]
pub(crate) struct Following(Option<String>);

pub(crate) fn choose_camera(
    suite: Option<Res<ActiveSuite>>,
    config: Res<SceneConfig>,
    mut state: ResMut<CharacterState>,
) {
    let Some(suite) = suite else {
        return;
    };
    state.camera = match suite.remembered(&config).camera.as_deref() {
        Some(ORBIT_CAMERA) => None,
        remembered => prefer(
            remembered.map(str::to_string),
            |name| suite.cameras.iter().any(|c| c.name == name),
            suite.default_camera.clone(),
        ),
    };
}

/// Swaps the rig for the one asked for. The file is loaded whole, as a
/// `Gltf`, rather than just its scene like the character's files: the rig
/// needs the file's clip too, and a missing `#Animation0` label would be a
/// load error for every static camera.
pub(crate) fn switch_rig(
    mut commands: Commands,
    state: Res<CharacterState>,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownRig>,
) {
    if shown.name == state.camera {
        return;
    }
    let Some(suite) = suite else {
        return;
    };
    if let Some(root) = shown.root.take() {
        commands.entity(root).despawn();
    }
    shown.name = state.camera.clone();
    shown.loading =
        state.camera.as_ref().and_then(|name| suite.cameras.iter().find(|c| &c.name == name)).map(
            |camera| {
                assets
                    .load_builder()
                    .with_settings(|s: &mut GltfLoaderSettings| s.load_lights = false)
                    .load(asset_path(&suite, &camera.file))
            },
        );
}

pub(crate) fn spawn_rig(
    mut commands: Commands,
    mut shown: ResMut<ShownRig>,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
) {
    let (Some(name), Some(file)) = (shown.name.clone(), shown.loading.clone()) else {
        return;
    };
    if assets.load_state(file.id()).is_failed() {
        warn!("camera {name:?}: its file failed to load");
        shown.loading = None;
        return;
    }
    let Some(gltf) = gltfs.get(&file) else {
        return;
    };
    shown.loading = None;
    let Some(scene) = gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned()) else {
        warn!("camera {name:?}: its file holds no scene");
        return;
    };
    let clip = gltf.animations.first().cloned();
    let root = commands
        .spawn((
            Name::new(format!("Camera rig {name}")),
            CameraRig { name, clip, _file: file },
            WorldAssetRoot(scene),
        ))
        .id();
    shown.root = Some(root);
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

/// Moves the real camera onto the selected rig's lens every frame. Both
/// `Transform` and `GlobalTransform` are written: propagation has already run
/// this frame, so writing only `Transform` would show last frame's position.
///
/// The projection is copied only on a switch. It never animates (Blender can
/// export animated focal length only through an extension Bevy cannot load),
/// and assigning it makes Bevy fit its aspect ratio to the window again.
pub(crate) fn follow_selected(
    state: Res<CharacterState>,
    rigs: Query<(&CameraRig, &Lens)>,
    lenses: Query<(&GlobalTransform, &Projection), Without<OrbitCamera>>,
    mut viewer: Query<(
        &mut Transform,
        &mut GlobalTransform,
        &mut Projection,
        &OrbitCamera,
        &mut Following,
    )>,
) {
    let Ok((mut transform, mut global, mut projection, orbit, mut following)) = viewer.single_mut()
    else {
        return;
    };

    let wanted = state.camera.as_deref();
    let lens = wanted
        .and_then(|name| rigs.iter().find(|(rig, _)| rig.name == name))
        .and_then(|(_, lens)| lenses.get(lens.0).ok());

    match lens {
        Some((lens_global, lens_projection)) => {
            if following.0.as_deref() != wanted {
                *projection = lens_projection.clone();
                following.0 = wanted.map(str::to_string);
            }
            *transform = lens_global.compute_transform();
            *global = *lens_global;
        }
        // A rig still loading keeps the last view rather than flashing the
        // orbit camera for the frames in between.
        None if wanted.is_none() && following.0.is_some() => {
            *projection = Projection::default();
            update_camera_transform(&mut transform, orbit);
            *global = GlobalTransform::from(*transform);
            following.0 = None;
        }
        None => {}
    }
}
