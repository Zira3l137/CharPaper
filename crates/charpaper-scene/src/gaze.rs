use bevy::app::AnimationSystems;
use bevy::ecs::system::SystemParam;
use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_gaze::Eyes;
use charpaper_gaze::Frame;
use charpaper_gaze::Setup;
use charpaper_gaze::Spring;
use charpaper_gaze::View;
use charpaper_settings::GazeSettings;
use charpaper_settings::Graphics;
use charpaper_suite::EyeKind;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::character::Armature;
use crate::character::CharacterClips;
use crate::character::ShownSkin;
use crate::character::SkinObjects;
use crate::render::ScreenParts;
use crate::screens::Screen;
use crate::screens::ScreenPlace;
use crate::stage::StageOf;
use crate::state::CharacterState;
use crate::suite::ActiveSuite;

// Turns the neck, head and eyes towards the mouse cursor, on top of whatever animation is
// playing: after the animation has posed the bones this frame, before their final positions
// are worked out from the pose.
//
// Where she looks comes from the picture: how far the cursor is from her eyes on screen, and
// which way. charpaper-gaze turns that into a direction and shares the turn out between neck,
// head and eyes. This file reads the pose for it and writes the bones and shape keys.
//
// Everything is measured in the armature's own space, which is Blender's world as exported,
// with +Y up, and against bones rather than the world: the head from the bone above the neck,
// the eyes from the head. So it holds while she leans, lies down or turns away.

pub(crate) struct GazePlugin;

impl Plugin for GazePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorPosition>()
            .add_observer(build_rig)
            .add_systems(Update, reset_rig.in_set(SceneSet::Fill))
            .add_systems(Update, find_shape_keys.in_set(SceneSet::Run))
            .add_systems(
                PostUpdate,
                aim.after(AnimationSystems).before(TransformSystems::Propagate),
            );
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

const FADE_SECS: f32 = 0.4;
// With the cursor still this long, she goes back to what the animation has her do.
const IDLE_SECS: f32 = 5.0;
// About how long the head and neck take to get where they should point. The eyes answer at
// once and wait there for the head, as eyes do.
const HEAD_SECS: f32 = 0.5;
const EYE_SECS: f32 = 0.06;
// Eyes that are no eyeballs, or are moved by shape keys alone, are at their fullest for a
// target this far aside.
const DRAWN_REACH_DEG: [f32; 2] = [35.0, 25.0];

struct Bone {
    entity: Entity,
    // In the armature's space, in the rest pose.
    rest: Quat,
    // The animation's rotation before gaze changed it, and the rotation written. A clip that
    // does not key this bone leaves the written one in place, which must not be turned again
    // on top.
    base: Quat,
    written: Quat,
}

// A bone something is measured from, with its rotation in the rest pose.
type Anchor = Option<(Entity, Quat)>;

#[derive(Component, Default)]
pub(crate) struct GazeRig {
    armature: Option<Entity>,
    frame: Frame,
    setup: Setup,
    // The bone above the neck: the head's turn and its limits are measured from it. Without
    // one, from the armature.
    chest: Anchor,
    // Only beside a head. A rig with a neck alone has it stand in as the head.
    neck: Option<Bone>,
    head: Option<Bone>,
    eyes: Vec<Bone>,
    // What the eyes turn in: the head, or what they hang from.
    socket: Anchor,
    // Left, right, up, down: every (mesh, weight index) of that shape key in the worn skin.
    keys: [Vec<(Entity, usize)>; 4],
    weight: f32,
    // Whether the cursor is where she can turn to. Out of reach she lets go of it.
    in_reach: bool,
    // The way she looks, kept while no screen of hers sees the cursor.
    direction: Option<Vec3>,
    // How far head and neck should turn from where the clip points the head, and how far
    // they have.
    turn_goal: Vec2,
    turned: Spring,
    // The eyes' look, each way from -1 to 1.
    look_goal: Vec2,
    look: Vec2,
    last_cursor: Option<CursorAt>,
    still_secs: f32,
    keys_written: bool,
}

fn reset_rig(mut stages: Query<&mut GazeRig, Added<ActiveSuite>>) {
    for mut rig in &mut stages {
        *rig = default();
    }
}

// Runs as the armature spawns, before any animation has touched it, so the bones still hold
// their rest pose.
fn build_rig(
    ready: On<WorldInstanceReady>,
    armatures: Query<(), With<Armature>>,
    stage_of: StageOf,
    mut stages: Query<(Option<&ActiveSuite>, &mut GazeRig)>,
    children: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    transforms: Query<&Transform>,
) {
    let armature = ready.entity;
    if !armatures.contains(armature) {
        return;
    }
    let Some((suite, mut rig)) = stage_of.get(armature).and_then(|s| stages.get_mut(s).ok()) else {
        return;
    };
    let keys = std::mem::take(&mut rig.keys);
    *rig = GazeRig { keys, ..default() };
    let Some(gaze) = suite.and_then(|s| s.gaze.clone()) else {
        return;
    };

    // A bone that is named but missing turns gaze off rather than aiming with what is left.
    let find = |name: &str| {
        let bone = children
            .iter_descendants(armature)
            .find(|&e| names.get(e).is_ok_and(|n| n.as_str() == name));
        if bone.is_none() {
            warn!(
                "gaze: the armature has no bone {name:?}; the character will not follow the cursor"
            );
        }
        bone
    };
    let optional = |name: Option<&str>| match name {
        Some(name) => find(name).map(Some),
        None => Some(None),
    };
    let eyes: Option<Vec<Entity>> = gaze.eyes.iter().map(|eye| find(eye)).collect();
    let (Some(eyes), Some(head), Some(neck)) =
        (eyes, optional(gaze.head.as_deref()), optional(gaze.neck.as_deref()))
    else {
        return;
    };
    let eye_keys = gaze.shape_keys.named().next().is_some();
    if eyes.is_empty() && head.is_none() && neck.is_none() && !eye_keys {
        return;
    }

    let rest = |bone: Entity| pose(bone, armature, &parents, &transforms);
    let bone = |entity: Entity| {
        let rotation = transforms.get(entity).map_or(Quat::IDENTITY, |t| t.rotation);
        Bone { entity, rest: rest(entity).rotation, base: rotation, written: rotation }
    };
    let above = |bone: Entity| {
        let parent = parents.get(bone).ok()?.parent();
        (parent != armature).then(|| (parent, rest(parent).rotation))
    };
    let radians = |[yaw, pitch]: [f32; 2]| Vec2::new(yaw.to_radians(), pitch.to_radians()).abs();

    let head_limit = radians(gaze.head_limits_deg());
    let neck_limit = radians(gaze.neck_limits_deg());
    let (neck, head, turn_limit, neck_limit) = match (neck, head) {
        (Some(neck), Some(head)) => (Some(neck), Some(head), head_limit + neck_limit, neck_limit),
        (None, Some(head)) => (None, Some(head), head_limit, Vec2::ZERO),
        (neck, None) => (None, neck, neck_limit, Vec2::ZERO),
    };
    let eye_limit = radians(gaze.eye_limits_deg());
    let drawn_reach = radians(DRAWN_REACH_DEG);
    let eyes_setup = match gaze.eye_kind {
        _ if eyes.is_empty() && !eye_keys => None,
        _ if eyes.is_empty() => Some(Eyes { limit: Vec2::ZERO, reach: drawn_reach }),
        EyeKind::Ball => Some(Eyes { limit: eye_limit, reach: eye_limit }),
        // Never less than the limit: the bone would turn further than she looks.
        EyeKind::Drawn => Some(Eyes { limit: eye_limit, reach: drawn_reach.max(eye_limit) }),
    };

    let eyes_at: Vec<Vec3> = eyes.iter().map(|&eye| rest(eye).translation).collect();
    let stated = gaze.front.is_some().then(|| Vec3::from_array(gaze.front_gltf()));
    let worked_out = head
        .map(|head| rest(head).translation)
        .and_then(|pivot| charpaper_gaze::front_from_eyes(pivot, &eyes_at, Vec3::Y));
    let front = stated.or(worked_out).unwrap_or(Vec3::Z);
    let how = match (stated, worked_out) {
        (Some(_), _) => "as suite.toml says",
        (None, Some(_)) => "worked out from her eye bones",
        (None, None) => "taken as Blender's front view, since her bones don't say",
    };
    info!("gaze: she faces {front:.2} in her rest pose, {how}");
    if let (Some(stated), Some(worked_out)) = (stated, worked_out)
        && stated.dot(worked_out) < 0.0
    {
        warn!(
            "gaze: suite.toml's `front` points to the back of her head, going by her eye bones; \
             she will turn away from the cursor. Remove it to have it worked out"
        );
    }

    rig.armature = Some(armature);
    rig.frame = Frame::facing(front, Vec3::Y).unwrap_or_default();
    rig.setup = Setup {
        turn_limit,
        neck_limit,
        head_share: gaze.head_share(),
        head_turn: gaze.head_turn(),
        eyes: eyes_setup,
    };
    rig.chest = neck.or(head).and_then(above);
    rig.socket = match head {
        Some(head) => Some((head, rest(head).rotation)),
        None => eyes.first().copied().and_then(above),
    };
    rig.neck = neck.map(bone);
    rig.head = head.map(bone);
    rig.eyes = eyes.into_iter().map(bone).collect();
}

// Which meshes of the worn skin carry the shape keys that follow the eyes. Bevy keeps no
// shape key names for GPU-only meshes, so they are read from the skin's file.
fn find_shape_keys(
    mut stages: Query<
        (Option<&ActiveSuite>, &ShownSkin, &SkinObjects, &mut GazeRig),
        Changed<SkinObjects>,
    >,
) {
    for (suite, shown, objects, mut rig) in &mut stages {
        rig.keys = default();
        if let Some(suite) = suite {
            find_stage_shape_keys(suite, shown, objects, &mut rig);
        }
    }
}

fn find_stage_shape_keys(
    suite: &ActiveSuite,
    shown: &ShownSkin,
    objects: &SkinObjects,
    rig: &mut GazeRig,
) {
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

// The bones the gaze reads and turns, and the shape keys it sets.
#[derive(SystemParam)]
struct Bones<'w, 's> {
    globals: Query<'w, 's, &'static GlobalTransform>,
    parents: Query<'w, 's, &'static ChildOf>,
    transforms: Query<'w, 's, &'static mut Transform>,
    morphs: Query<'w, 's, &'static mut MorphWeights>,
}

fn aim(
    time: Res<Time>,
    cursor: Res<CursorPosition>,
    mut stages: Query<(
        Entity,
        &GazeSettings,
        &CharacterState,
        Option<&CharacterClips>,
        &Graphics,
        &mut GazeRig,
    )>,
    screens: Query<(&Screen, &ScreenParts, Option<&ScreenPlace>)>,
    cameras: Query<(&Camera, &GlobalTransform), With<SceneCamera>>,
    stage_of: StageOf,
    mut bones: Bones,
) {
    for (stage, settings, state, clips, graphics, mut rig) in &mut stages {
        let seen = cursor.0.and_then(|at| {
            let (position, camera) = seen_from(stage, at, &screens, &stage_of)?;
            Some((position, cameras.get(camera).ok()?))
        });
        let watched = Watched { cursor: cursor.0, seen, settings, state, clips };
        aim_stage(&time, graphics, watched, &mut rig, &mut bones);
    }
}

// Where the cursor is for one of the stage's own screens, and that screen's camera: the screen
// the cursor is on, or, with the cursor on a monitor that shows something else, the stage's
// nearest one, so that she looks across to it.
fn seen_from(
    stage: Entity,
    at: CursorAt,
    screens: &Query<(&Screen, &ScreenParts, Option<&ScreenPlace>)>,
    stage_of: &StageOf,
) -> Option<(Vec2, Entity)> {
    let (_, on, on_place) = screens.iter().find(|(screen, ..)| screen.window == at.window)?;
    if stage_of.get(on.camera) == Some(stage) {
        return Some((at.position, on.camera));
    }
    let desktop = on_place?.0.min.as_vec2() + at.position;
    let away = |place: &IRect| {
        (desktop - desktop.clamp(place.min.as_vec2(), place.max.as_vec2())).length()
    };
    screens
        .iter()
        .filter(|(_, parts, _)| stage_of.get(parts.camera) == Some(stage))
        .filter_map(|(_, parts, place)| Some((place?.0, parts.camera)))
        .min_by(|(a, _), (b, _)| away(a).total_cmp(&away(b)))
        .map(|(place, camera)| (desktop - place.min.as_vec2(), camera))
}

// What the character watches: the cursor, and how one of the stage's screens sees it.
struct Watched<'a> {
    cursor: Option<CursorAt>,
    seen: Option<(Vec2, (&'a Camera, &'a GlobalTransform))>,
    settings: &'a GazeSettings,
    state: &'a CharacterState,
    clips: Option<&'a CharacterClips>,
}

fn aim_stage(
    time: &Time,
    graphics: &Graphics,
    Watched { cursor, seen, settings, state, clips }: Watched,
    rig: &mut GazeRig,
    bones: &mut Bones,
) {
    let Some(armature) = rig.armature else {
        return;
    };
    let Ok(armature_global) = bones.globals.get(armature).copied() else {
        return;
    };
    let dt = time.delta_secs();
    let (frame, setup) = (rig.frame, rig.setup);

    if cursor.is_some() && cursor != rig.last_cursor {
        rig.last_cursor = cursor;
        rig.still_secs = 0.0;
    } else {
        rig.still_secs += dt;
    }
    let allowed = state
        .animation
        .as_deref()
        .and_then(|name| clips.as_ref()?.get(name))
        .is_none_or(|clip| clip.gaze);
    let wanted =
        settings.follow_cursor && cursor.is_some() && allowed && rig.still_secs < IDLE_SECS;

    for bone in rig.bones_mut() {
        let Ok(mut transform) = bones.transforms.get_mut(bone.entity) else {
            return;
        };
        if transform.rotation == bone.written {
            transform.rotation = bone.base;
        }
        bone.base = transform.rotation;
        bone.written = transform.rotation;
    }
    if !wanted && rig.weight <= 0.0 && !rig.keys_written {
        rig.start_over();
        return;
    }

    // From here on the bones hold what the clip gave them.
    if let Some((at, (camera, camera_global))) = seen {
        let eyes = armature_global.transform_point(eyes_at(rig, armature, bones));
        let at = at * graphics.scale_percent() as f32 / 100.0;
        if let Some(direction) = look_from(camera, camera_global, at, eyes) {
            let to_armature = armature_global.affine().inverse();
            rig.direction = Some(to_armature.transform_vector3(direction).normalize_or(Vec3::Z));
        }
    }
    let Some(direction) = rig.direction else {
        return;
    };

    let chest = turn_of(rig.chest, armature, bones);
    let target = frame.angles(chest.inverse() * direction);
    rig.in_reach = setup.in_reach(target, rig.in_reach);
    let step = dt / FADE_SECS;
    let goal = if wanted && rig.in_reach { 1.0 } else { 0.0 };
    rig.weight += (goal - rig.weight).clamp(-step, step);
    if rig.weight <= 0.0 && !rig.keys_written {
        rig.start_over();
        return;
    }
    let weight = rig.weight;

    // Neck before head before eyes: each turn moves the bones below it, and the next part
    // works from where that left it.
    if let Some(head) = &mut rig.head {
        let head_turn = turn_of(head.anchor(), armature, bones);
        let clip = frame.angles(chest.inverse() * head_turn * frame.front);
        // Out of reach the goal stays as it was, so she eases back from where she looked
        // rather than swinging after a target that passes behind her.
        if rig.in_reach {
            rig.turn_goal = setup.head(target, clip) - clip;
        }
        rig.turned.step(rig.turn_goal, HEAD_SECS, dt);
        let head_at = clip + rig.turned.value * weight;
        let neck_at = setup.neck(clip, head_at);
        // The turns are worked out as the chest stood in the rest pose, and go wherever the
        // clip has taken the chest since.
        let carried = |turn: Quat| charpaper_gaze::carried(chest, turn);
        if let Some(neck) = &mut rig.neck {
            turn(neck, carried(frame.turn(clip, neck_at)), armature, bones);
        }
        turn(head, carried(frame.turn(neck_at, head_at)), armature, bones);
    }

    let Some(eyes) = setup.eyes else {
        rig.keys_written = false;
        return;
    };
    let socket = turn_of(rig.socket, armature, bones);
    if rig.in_reach {
        rig.look_goal = eyes.look(frame.angles(socket.inverse() * direction));
    }
    rig.look = charpaper_gaze::follow(rig.look, rig.look_goal, EYE_SECS, dt);
    // Set outright rather than added to the clip's own turn, so the eyes can't leave their
    // sockets. Both get the same turn: aimed at a point close by, they would cross.
    let aimed = socket * frame.toward(eyes.turn(rig.look));
    for eye in &mut rig.eyes {
        let parent = parent_rotation(eye.entity, armature, bones);
        if let Ok(mut transform) = bones.transforms.get_mut(eye.entity) {
            let set = charpaper_gaze::turned_from_rest(aimed, eye.rest, parent);
            transform.rotation = transform.rotation.slerp(set, weight).normalize();
            eye.written = transform.rotation;
        }
    }

    let look = rig.look * weight;
    let values = [look.x.max(0.0), (-look.x).max(0.0), look.y.max(0.0), (-look.y).max(0.0)];
    for (targets, value) in rig.keys.iter().zip(values) {
        for &(mesh, index) in targets {
            if let Ok(mut morph) = bones.morphs.get_mut(mesh)
                && let Some(slot) = morph.weights_mut().get_mut(index)
            {
                *slot = value;
            }
        }
    }
    // One last write at weight 0 leaves the keys at 0 before gaze lets go of them.
    rig.keys_written = weight > 0.0;
}

impl Bone {
    fn anchor(&self) -> Anchor {
        Some((self.entity, self.rest))
    }
}

impl GazeRig {
    fn bones_mut(&mut self) -> impl Iterator<Item = &mut Bone> {
        self.neck.iter_mut().chain(self.head.iter_mut()).chain(self.eyes.iter_mut())
    }

    // Nothing is carried over to the next time she looks up.
    fn start_over(&mut self) {
        self.turn_goal = Vec2::ZERO;
        self.turned = Spring::default();
        self.look_goal = Vec2::ZERO;
        self.look = Vec2::ZERO;
    }
}

// The direction she looks in, in world space, for a cursor at `at` in the camera's picture,
// with her eyes at `eyes`. None while her eyes are behind the camera.
fn look_from(
    camera: &Camera,
    camera_global: &GlobalTransform,
    at: Vec2,
    eyes: Vec3,
) -> Option<Vec3> {
    let eyes_at = camera.world_to_viewport(camera_global, eyes).ok()?;
    let half_height = camera.logical_viewport_size()?.y / 2.0;
    // The line of sight to her eyes, whatever the lens: through an orthographic one it isn't
    // the line to the camera.
    let sight = camera.viewport_to_world(camera_global, eyes_at).ok()?;
    let view =
        View { toward: -*sight.direction, right: *camera_global.right(), up: *camera_global.up() };
    // The picture's y runs down.
    let offset = (at - eyes_at) / half_height * Vec2::new(1.0, -1.0);
    Some(charpaper_gaze::look_direction(&view, offset, 1.0))
}

// Where her eyes are in the armature's space: between the eye bones, or without any, at the
// joint of the head or of whatever stands in for it.
fn eyes_at(rig: &GazeRig, armature: Entity, bones: &Bones) -> Vec3 {
    let at = |bone: Entity| {
        pose(bone, armature, &bones.parents, &bones.transforms.as_readonly()).translation
    };
    if rig.eyes.is_empty() {
        let stand_in =
            rig.head.as_ref().map(|head| head.entity).or(rig.socket.map(|(bone, _)| bone));
        return stand_in.map_or(Vec3::ZERO, at);
    }
    rig.eyes.iter().map(|eye| at(eye.entity)).sum::<Vec3>() / rig.eyes.len() as f32
}

// How far the bone has turned from its rest pose, in the armature's space. Without a bone,
// not at all.
fn turn_of(anchor: Anchor, armature: Entity, bones: &Bones) -> Quat {
    anchor.map_or(Quat::IDENTITY, |(bone, rest)| {
        let now = pose(bone, armature, &bones.parents, &bones.transforms.as_readonly()).rotation;
        charpaper_gaze::turn_from_rest(now, rest)
    })
}

fn parent_rotation(bone: Entity, armature: Entity, bones: &Bones) -> Quat {
    bones.parents.get(bone).ok().filter(|p| p.parent() != armature).map_or(Quat::IDENTITY, |p| {
        pose(p.parent(), armature, &bones.parents, &bones.transforms.as_readonly()).rotation
    })
}

// Turns the bone by `by`, a turn in the armature's space, around the bone's own pivot.
fn turn(bone: &mut Bone, by: Quat, armature: Entity, bones: &mut Bones) {
    let parent = parent_rotation(bone.entity, armature, bones);
    if let Ok(mut transform) = bones.transforms.get_mut(bone.entity) {
        transform.rotation = charpaper_gaze::turned_by(transform.rotation, by, parent);
        bone.written = transform.rotation;
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
