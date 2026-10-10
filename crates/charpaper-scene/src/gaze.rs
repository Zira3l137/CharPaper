use bevy::app::AnimationSystems;
use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::WorldInstanceReady;
use serde::Deserialize;
use serde::Serialize;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::character::Armature;
use crate::character::CharacterClips;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::config::SceneConfig;
use crate::render::RenderSettings;
use crate::render::ScreenParts;
use crate::screens::Screen;
use crate::state::CharacterState;
use crate::suite::ActiveSuite;

// Turns the neck, head and eyes towards the mouse cursor, on top of whatever animation is
// playing: after the animation has posed the bones this frame, before their final positions
// are worked out from the pose. The eyes turn first; the head and neck share what the eyes
// cannot reach.
//
// All the aiming happens in the armature's own space, where the character faces +Z and her
// left is +X (Blender's -Y front, as the glTF exporter converts it).

pub(crate) struct GazePlugin;

impl Plugin for GazePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorPosition>()
            .init_resource::<GazeRig>()
            .add_observer(build_rig)
            .add_systems(Update, reset_rig.in_set(SceneSet::Fill))
            .add_systems(
                Update,
                find_shape_keys.run_if(resource_changed::<SkinObjects>).in_set(SceneSet::Run),
            )
            .add_systems(
                PostUpdate,
                aim.after(AnimationSystems).before(TransformSystems::Propagate),
            );
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GazeSettings {
    pub follow_cursor: bool,
}

impl Default for GazeSettings {
    fn default() -> Self {
        Self { follow_cursor: true }
    }
}

// The app fills it in every frame.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct CursorPosition(pub Option<CursorAt>);

// In physical pixels from the top-left corner of a Screen's window, possibly outside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorAt {
    pub window: Entity,
    pub position: Vec2,
}

#[derive(Clone, Debug)]
pub struct GazeTuning {
    // Where along the ray from the camera through the cursor she looks, as a share of the
    // distance to her head: 1 looks at the cursor as if it were beside her, near 0 at the
    // camera itself, and she barely moves.
    pub target_depth: f32,
    pub fade_secs: f32,
    // With the cursor still this long, she goes back to what the animation has her do.
    pub idle_secs: f32,
    // How quickly each part catches up with where it should look.
    pub eye_secs: f32,
    pub head_secs: f32,
    pub neck_secs: f32,
}

impl Default for GazeTuning {
    fn default() -> Self {
        Self {
            target_depth: 0.5,
            fade_secs: 0.4,
            idle_secs: 5.0,
            eye_secs: 0.06,
            head_secs: 0.25,
            neck_secs: 0.35,
        }
    }
}

struct Part {
    bone: Entity,
    // The way the face points, in the bone's own space, from the rest pose.
    forward: Vec3,
    limits: Vec2,
    // Yaw and pitch this part adds on top of the animation, smoothed, in radians.
    angles: Vec2,
    // The animation's rotation before the turn was added, and the rotation written. A clip
    // that does not key this bone leaves the written one in place, which must not be turned
    // again on top.
    base: Quat,
    written: Quat,
}

#[derive(Resource, Default)]
pub(crate) struct GazeRig {
    armature: Option<Entity>,
    neck: Option<Part>,
    head: Option<Part>,
    eyes: Vec<Part>,
    head_share: f32,
    // Left, right, up, down: every (mesh, weight index) of that shape key in the worn skin.
    keys: [Vec<(Entity, usize)>; 4],
    weight: f32,
    target: Option<Vec3>,
    last_cursor: Option<CursorAt>,
    still_secs: f32,
    keys_written: bool,
}

fn reset_rig(mut rig: ResMut<GazeRig>) {
    *rig = default();
}

// Runs as the armature spawns, before any animation has touched it, so the bones still hold
// their rest pose.
fn build_rig(
    ready: On<WorldInstanceReady>,
    armatures: Query<(), With<Armature>>,
    suite: Option<Res<ActiveSuite>>,
    children: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    transforms: Query<&Transform>,
    mut rig: ResMut<GazeRig>,
) {
    let armature = ready.entity;
    if !armatures.contains(armature) {
        return;
    }
    let keys = std::mem::take(&mut rig.keys);
    *rig = GazeRig { keys, ..default() };
    let Some(gaze) = suite.as_ref().and_then(|s| s.gaze.clone()) else {
        return;
    };

    let find = |name: &str| {
        children
            .iter_descendants(armature)
            .find(|&e| names.get(e).is_ok_and(|n| n.as_str() == name))
    };
    let part = |name: &str, [yaw, pitch]: [f32; 2]| -> Option<Part> {
        let Some(bone) = find(name) else {
            warn!(
                "gaze: the armature has no bone {name:?}; the character will not follow the cursor"
            );
            return None;
        };
        let rest = pose(bone, armature, &parents, &transforms).rotation;
        let rotation = transforms.get(bone).map_or(Quat::IDENTITY, |t| t.rotation);
        Some(Part {
            bone,
            forward: rest.inverse() * Vec3::Z,
            limits: Vec2::new(yaw.to_radians(), pitch.to_radians()).abs(),
            angles: Vec2::ZERO,
            base: rotation,
            written: rotation,
        })
    };

    // A bone that is named but missing turns gaze off rather than aiming with what is left.
    let optional = |name: Option<&str>, limits| match name {
        Some(name) => part(name, limits).map(Some),
        None => Some(None),
    };
    let eyes: Option<Vec<Part>> =
        gaze.eyes.iter().map(|eye| part(eye, gaze.eye_limits_deg())).collect();
    let head = optional(gaze.head.as_deref(), gaze.head_limits_deg());
    let neck = optional(gaze.neck.as_deref(), gaze.neck_limits_deg());
    let (Some(eyes), Some(head), Some(neck)) = (eyes, head, neck) else {
        return;
    };
    if eyes.is_empty() {
        return;
    }
    rig.armature = Some(armature);
    rig.eyes = eyes;
    rig.head = head;
    rig.neck = neck;
    rig.head_share = gaze.head_share();
}

// Which meshes of the worn skin carry the shape keys that follow the eyes. Bevy keeps no
// shape key names for GPU-only meshes, so they are read from the skin's file.
fn find_shape_keys(
    suite: Option<Res<ActiveSuite>>,
    shown: Res<ShownSkin>,
    objects: Res<SkinObjects>,
    mut rig: ResMut<GazeRig>,
) {
    rig.keys = default();
    let Some(suite) = suite else {
        return;
    };
    let (Some(gaze), Some(name)) = (&suite.gaze, &shown.name) else {
        return;
    };
    let Some(skin) = suite.skins.iter().find(|s| &s.name == name) else {
        return;
    };
    let meshes = match charpaper_suite::mesh_shape_keys(&suite.absolute(&skin.file)) {
        Ok(meshes) => meshes,
        Err(err) => {
            warn!("skin {name:?}: cannot read its shape keys: {err}");
            return;
        }
    };
    let wanted = &gaze.shape_keys;
    let names = [&wanted.left, &wanted.right, &wanted.up, &wanted.down];
    for (slot, key) in rig.keys.iter_mut().zip(names) {
        let Some(key) = key else {
            continue;
        };
        for mesh in &meshes {
            let Some(index) = mesh.keys.iter().position(|k| k == key) else {
                continue;
            };
            if let Some((_, entity)) = objects.0.iter().find(|(node, _)| *node == mesh.node) {
                slot.push((*entity, index));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn aim(
    time: Res<Time>,
    config: Res<SceneConfig>,
    settings: Res<GazeSettings>,
    render: Res<RenderSettings>,
    cursor: Res<CursorPosition>,
    state: Res<CharacterState>,
    clips: Option<Res<CharacterClips>>,
    screens: Query<(&Screen, &ScreenParts)>,
    cameras: Query<(&Camera, &GlobalTransform), With<SceneCamera>>,
    globals: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
    mut morphs: Query<&mut MorphWeights>,
    mut rig: ResMut<GazeRig>,
) {
    let rig = &mut *rig;
    let Some(armature) = rig.armature else {
        return;
    };
    let Ok(armature_global) = globals.get(armature) else {
        return;
    };
    let tuning = &config.gaze_tuning;
    let dt = time.delta_secs();

    if cursor.0.is_some() && cursor.0 != rig.last_cursor {
        rig.last_cursor = cursor.0;
        rig.still_secs = 0.0;
    } else {
        rig.still_secs += dt;
    }
    let allowed = state
        .animation
        .as_deref()
        .and_then(|name| clips.as_ref()?.get(name))
        .is_none_or(|clip| clip.gaze);
    let wanted = settings.follow_cursor
        && cursor.0.is_some()
        && allowed
        && rig.still_secs < tuning.idle_secs;
    let step = if tuning.fade_secs > 0.0 { dt / tuning.fade_secs } else { 1.0 };
    let goal = if wanted { 1.0 } else { 0.0 };
    rig.weight += (goal - rig.weight).clamp(-step, step);

    let parts = rig.neck.iter_mut().chain(rig.head.iter_mut()).chain(rig.eyes.iter_mut());
    for part in parts {
        let Ok(mut transform) = transforms.get_mut(part.bone) else {
            return;
        };
        if transform.rotation == part.written {
            transform.rotation = part.base;
        }
        part.base = transform.rotation;
        part.written = transform.rotation;
    }
    if rig.weight <= 0.0 && !rig.keys_written {
        for part in rig.parts_mut() {
            part.angles = Vec2::ZERO;
        }
        return;
    }

    let reference = rig.head.as_ref().or(rig.neck.as_ref()).unwrap_or(&rig.eyes[0]);
    let (reference_bone, reference_forward) = (reference.bone, reference.forward);
    let to_model = armature_global.affine().inverse();
    // Through the camera of the screen the cursor is measured from.
    let seen = cursor.0.and_then(|at| {
        let (_, parts) = screens.iter().find(|(screen, _)| screen.window == at.window)?;
        Some((at.position, cameras.get(parts.camera).ok()?))
    });
    if let Some((at, (camera, camera_global))) = seen {
        let scale = render.scale_percent() as f32 / 100.0;
        if let Ok(ray) = camera.viewport_to_world(camera_global, at * scale) {
            let head = pose(reference_bone, armature, &parents, &transforms.as_readonly());
            let head_world = armature_global.transform_point(head.translation);
            let distance = (head_world - ray.origin).dot(*ray.direction).max(0.0);
            let point = ray.origin + *ray.direction * distance * tuning.target_depth;
            rig.target = Some(to_model.transform_point3(point));
        }
    }
    let Some(target) = rig.target else {
        return;
    };

    let face = pose(reference_bone, armature, &parents, &transforms.as_readonly());
    let face_forward = face.rotation * reference_forward;
    let total = difference(angles(target - face.translation), angles(face_forward));
    let eye_limits = rig.eyes[0].limits;
    let beyond = total - total.clamp(-eye_limits, eye_limits);
    let (head_goal, neck_goal) = match (&rig.head, &rig.neck) {
        (Some(_), Some(_)) => (beyond * rig.head_share, beyond * (1.0 - rig.head_share)),
        (Some(_), None) => (beyond, Vec2::ZERO),
        (None, Some(_)) => (Vec2::ZERO, beyond),
        (None, None) => (Vec2::ZERO, Vec2::ZERO),
    };
    let weight = rig.weight;

    // Neck before head before eyes: each turn moves the bones below it, and the next part
    // works from where that left it.
    for (part, goal, secs) in
        [(&mut rig.neck, neck_goal, tuning.neck_secs), (&mut rig.head, head_goal, tuning.head_secs)]
    {
        if let Some(part) = part {
            part.angles = follow(part.angles, goal.clamp(-part.limits, part.limits), secs, dt);
            let by = part.angles * weight;
            turn(part, by, face_forward, armature, &parents, &mut transforms);
        }
    }

    let mut eye_angles = Vec2::ZERO;
    for eye in &mut rig.eyes {
        let eye_pose = pose(eye.bone, armature, &parents, &transforms.as_readonly());
        let forward = eye_pose.rotation * eye.forward;
        let goal = difference(angles(target - eye_pose.translation), angles(forward));
        eye.angles = follow(eye.angles, goal.clamp(-eye.limits, eye.limits), tuning.eye_secs, dt);
        let by = eye.angles * weight;
        turn(eye, by, forward, armature, &parents, &mut transforms);
        eye_angles += by;
    }
    eye_angles /= rig.eyes.len() as f32;

    let share =
        |amount: f32, limit: f32| if limit > 0.0 { (amount / limit).clamp(0.0, 1.0) } else { 0.0 };
    let values = [
        share(eye_angles.x, eye_limits.x),
        share(-eye_angles.x, eye_limits.x),
        share(eye_angles.y, eye_limits.y),
        share(-eye_angles.y, eye_limits.y),
    ];
    for (targets, value) in rig.keys.iter().zip(values) {
        for &(mesh, index) in targets {
            if let Ok(mut morph) = morphs.get_mut(mesh)
                && let Some(slot) = morph.weights_mut().get_mut(index)
            {
                *slot = value;
            }
        }
    }
    // One last write at weight 0 leaves the keys at 0 before gaze lets go of them.
    rig.keys_written = weight > 0.0;
}

impl GazeRig {
    fn parts_mut(&mut self) -> impl Iterator<Item = &mut Part> {
        self.neck.iter_mut().chain(self.head.iter_mut()).chain(self.eyes.iter_mut())
    }
}

// Adds `by` (yaw, pitch) to the bone's rotation, as turns about the armature's up axis and
// the face's sideways axis, around the bone's own pivot.
fn turn(
    part: &mut Part,
    by: Vec2,
    face: Vec3,
    armature: Entity,
    parents: &Query<&ChildOf>,
    transforms: &mut Query<&mut Transform>,
) {
    let sideways = Vec3::Y.cross(Vec3::new(face.x, 0.0, face.z)).normalize_or(Vec3::X);
    // Pitching up is a negative turn about the sideways axis, which points to her left.
    let turn = Quat::from_rotation_y(by.x) * Quat::from_axis_angle(sideways, -by.y);
    let parent = parents
        .get(part.bone)
        .ok()
        .filter(|p| p.parent() != armature)
        .map_or(Quat::IDENTITY, |p| {
            pose(p.parent(), armature, parents, &transforms.as_readonly()).rotation
        });
    if let Ok(mut transform) = transforms.get_mut(part.bone) {
        transform.rotation = (parent.inverse() * turn * parent * transform.rotation).normalize();
        part.written = transform.rotation;
    }
}

// The bone's pose in the armature's space, from this frame's local transforms: the global
// ones are only brought up to date after gaze has run.
fn pose(
    bone: Entity,
    armature: Entity,
    parents: &Query<&ChildOf>,
    transforms: &Query<&Transform>,
) -> Transform {
    let mut pose = transforms.get(bone).copied().unwrap_or_default();
    let mut current = bone;
    while let Ok(parent) = parents.get(current) {
        current = parent.parent();
        if current == armature {
            break;
        }
        let Ok(local) = transforms.get(current) else {
            break;
        };
        pose = local.mul_transform(pose);
    }
    pose
}

// Yaw towards her left (+X) and pitch upwards, in radians.
fn angles(direction: Vec3) -> Vec2 {
    let d = direction.normalize_or(Vec3::Z);
    Vec2::new(d.x.atan2(d.z), d.y.clamp(-1.0, 1.0).asin())
}

fn difference(to: Vec2, from: Vec2) -> Vec2 {
    let yaw = (to.x - from.x + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    Vec2::new(yaw, to.y - from.y)
}

fn follow(current: Vec2, goal: Vec2, secs: f32, dt: f32) -> Vec2 {
    if secs <= 0.0 {
        return goal;
    }
    current + (goal - current) * (1.0 - (-dt / secs).exp())
}
