// One real camera does all the rendering. Each exported camera is loaded as a rig whose
// own Camera is switched off; the real camera copies the rig's transform and projection.
// Only the rig in use is loaded.

use bevy::gltf::GltfLoaderSettings;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_suite::CameraEntry;
use charpaper_suite::ExportedCamera;
use charpaper_suite::ORBIT_CAMERA;

use crate::assets::asset_path;
use crate::camera::focus::FocusTarget;
use crate::camera::orbit::OrbitCamera;
use crate::camera::orbit::update_camera_transform;
use crate::state::CharacterState;
use crate::state::prefer;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;

#[derive(Resource, Default)]
pub(crate) struct ShownRig {
    pub name: Option<String>,
    pub root: Option<Entity>,
    pub loading: Option<(Handle<Gltf>, ExportedCamera)>,
}

#[derive(Component)]
pub(crate) struct CameraRig {
    pub name: String,
    pub settings: CameraEntry,
    node: Option<String>,
    clip: Option<Handle<AnimationClip>>,
    _file: Handle<Gltf>,
}

#[derive(Component)]
pub(crate) struct Lens(pub Entity);

#[derive(Component, Default)]
pub(crate) struct Following(Option<String>);

impl Following {
    pub fn name(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

pub(crate) fn choose_camera(
    suite: Option<Res<ActiveSuite>>,
    remembered: Res<Remembered>,
    mut state: ResMut<CharacterState>,
) {
    let Some(suite) = suite else {
        return;
    };
    state.camera = match suite.remembered(&remembered).camera.as_deref() {
        Some(ORBIT_CAMERA) => None,
        remembered => prefer(
            remembered.map(str::to_string),
            |name| suite.cameras.iter().any(|c| c.name == name),
            suite.default_camera.clone(),
        ),
    };
}

// Loaded as a whole Gltf for its clip: asking for a missing #Animation0 label would fail
// every static camera.
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
                let file = assets
                    .load_builder()
                    .with_settings(|s: &mut GltfLoaderSettings| s.load_lights = false)
                    .load(asset_path(&suite, &camera.file));
                (file, camera.clone())
            },
        );
}

pub(crate) fn spawn_rig(
    mut commands: Commands,
    mut shown: ResMut<ShownRig>,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
) {
    let (Some(name), Some((file, camera))) = (shown.name.clone(), shown.loading.clone()) else {
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
    let clip = match &camera.clip {
        Some(clip) => gltf.named_animations.get(clip.as_str()).cloned(),
        None => gltf.animations.first().cloned(),
    };
    let root = commands
        .spawn((
            Name::new(format!("Camera rig {name}")),
            CameraRig { name, settings: camera.settings, node: camera.node, clip, _file: file },
            FocusTarget::default(),
            WorldAssetRoot(scene),
        ))
        .id();
    shown.root = Some(root);
}

// An observer, so the rig's camera is off in the frame it spawns. A system would run a
// frame later and let it render once over the real one.
pub(crate) fn on_rig_ready(
    ready: On<WorldInstanceReady>,
    rigs: Query<&CameraRig>,
    children: Query<&Children>,
    names: Query<&Name>,
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

    let lens = match &rig.node {
        Some(node) => {
            lenses.iter().copied().find(|&l| names.get(l).is_ok_and(|n| n.as_str() == node))
        }
        None => lenses.first().copied(),
    };
    let Some(lens) = lens else {
        warn!("camera {:?}: its file holds no such camera", rig.name);
        return;
    };
    commands.entity(ready.entity).insert(Lens(lens));

    let Some(clip) = &rig.clip else {
        info!("camera {:?} ready, static", rig.name);
        return;
    };
    let roots: Vec<Entity> =
        children.iter_descendants(ready.entity).filter(|&e| players.contains(e)).collect();
    if roots.is_empty() {
        warn!("camera {:?}: its clip animates nothing, so it stays static", rig.name);
        return;
    }
    let (graph, nodes) = AnimationGraph::from_clips([clip.clone()]);
    let graph = graphs.add(graph);
    for root in roots {
        commands.entity(root).insert(AnimationGraphHandle(graph.clone()));
        if let Ok(mut player) = players.get_mut(root) {
            player.play(nodes[0]).repeat();
        }
    }
    info!("camera {:?} ready, looping its clip", rig.name);
}

// GlobalTransform is written too, because propagation already ran this frame. The
// projection is copied only on a switch: assigning it makes Bevy refit the aspect ratio.
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
        // A rig still loading keeps the last view instead of flashing the orbit camera.
        None if wanted.is_none() && following.0.is_some() => {
            *projection = Projection::default();
            update_camera_transform(&mut transform, orbit);
            *global = GlobalTransform::from(*transform);
            following.0 = None;
        }
        None => {}
    }
}
