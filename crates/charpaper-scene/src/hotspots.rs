// The suite's hotspots: capsules around bones that play clips when clicked, or when the cursor
// rests on them. A hotspot finds its bones by name, on the character or on an armature of the
// shown environment, so it follows every move and is back after every switch of skin or
// environment. The armature needs no clips of its own.
//
// Reactions are Once clips in slots above everything else the actor plays, so what played
// before shows again as they fade. One that replaces plays in the actor's reaction slot and
// stops its layers; a layered one takes the layer slot of one moving the same parts, if any.

use std::ops::Range;

use bevy::ecs::system::SystemParam;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;
use bevy::window::CursorLeft;
use bevy::window::CursorMoved;
use charpaper_anim::Actor;
use charpaper_anim::ClipId;
use charpaper_anim::Mode;
use charpaper_settings::GazeSettings;
use charpaper_suite::Hotspot;
use charpaper_suite::Trigger;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::character::CharacterActor;
use crate::character::CharacterClips;
use crate::character::REACTIONS as CHARACTER_REACTIONS;
use crate::environment::EnvironmentReady;
use crate::props::PropActor;
use crate::props::REACTIONS as PROP_REACTIONS;
use crate::render::ScreenParts;
use crate::screens::Screen;
use crate::screens::in_picture;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

// How long the cursor rests on a hover hotspot before it goes off: a cursor passing over
// shouldn't set it off.
const HOVER_SECS: f64 = 0.35;
// How far the cursor may move between press and release for a click, in logical pixels.
const CLICK_SLOP: f32 = 6.0;

pub(crate) struct HotspotsPlugin;

impl Plugin for HotspotsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, list_hotspots.in_set(SceneSet::Fill))
            .add_systems(Update, react.in_set(SceneSet::Run));
    }
}

// Where an actor's reactions play.
pub(crate) struct Reactions {
    pub replacing: usize,
    pub layers: Range<usize>,
}

impl Reactions {
    fn play(&self, actor: &mut Actor, clip: ClipId, layer: bool) -> usize {
        if layer {
            let slot = actor.slot_for(self.layers.clone(), clip);
            actor.play(slot, clip, Mode::Once);
            return slot;
        }
        self.layers.clone().for_each(|slot| actor.stop(slot));
        actor.play(self.replacing, clip, Mode::Once);
        self.replacing
    }
}

#[derive(Component, Default)]
pub(crate) struct Hotspots {
    spots: Vec<Spot>,
    // The hotspot whose expression the character wears.
    face: Option<usize>,
    // The click hotspot pressed on, and where.
    pressed: Option<(usize, Vec2)>,
    // The environment the bones of environments' armatures were looked for in.
    environment: Option<Entity>,
    // While a reaction that lets the cursor go plays on the character, gaze does.
    pub(crate) gaze_off: bool,
    seed: u64,
}

struct Spot {
    hotspot: Hotspot,
    // Each area's bone, once found, and whether the shown environment was searched for it.
    bones: Vec<Option<Entity>>,
    searched: Vec<bool>,
    // The clips of each actor it plays, with the one picked last.
    picks: Vec<Pick>,
    playing: Vec<Played>,
    fired: Option<f64>,
    hovered: Hovered,
}

struct Pick {
    actor: Option<String>,
    clips: Vec<String>,
    last: Option<usize>,
}

struct Played {
    actor: Entity,
    slot: usize,
    clip: ClipId,
    gaze: bool,
}

#[derive(Default)]
enum Hovered {
    #[default]
    Off,
    Since(f64),
    WentOff,
}

fn list_hotspots(
    mut commands: Commands,
    stages: Query<(Entity, &ActiveSuite), Added<ActiveSuite>>,
) {
    for (stage, suite) in &stages {
        let spots = suite.hotspots.values().map(|hotspot| {
            let mut picks: Vec<Pick> = Vec::new();
            for wanted in &hotspot.clips {
                match picks.iter_mut().find(|pick| pick.actor == wanted.actor) {
                    Some(pick) => pick.clips.push(wanted.clip.clone()),
                    None => picks.push(Pick {
                        actor: wanted.actor.clone(),
                        clips: vec![wanted.clip.clone()],
                        last: None,
                    }),
                }
            }
            Spot {
                bones: vec![None; hotspot.areas.len()],
                searched: vec![false; hotspot.areas.len()],
                hotspot: hotspot.clone(),
                picks,
                playing: Vec::new(),
                fired: None,
                hovered: Hovered::Off,
            }
        });
        let seed = stage.to_bits() | 1;
        commands.entity(stage).insert(Hotspots { spots: spots.collect(), seed, ..default() });
    }
}

// The actors hotspots find bones on and play clips on, each on its stage, and the shown
// environment, for armatures that aren't actors.
#[derive(SystemParam)]
struct Actors<'w, 's> {
    characters: Query<'w, 's, (Entity, &'static mut CharacterActor)>,
    props: Query<'w, 's, (Entity, &'static mut PropActor)>,
    stage_of: StageOf<'w, 's>,
    environments: Query<'w, 's, (Entity, &'static ChildOf), With<EnvironmentReady>>,
    children: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
}

impl Actors<'_, '_> {
    fn character(&mut self, stage: Entity) -> Option<(Entity, Mut<'_, CharacterActor>)> {
        let stage_of = &self.stage_of;
        self.characters.iter_mut().find(|(entity, _)| stage_of.get(*entity) == Some(stage))
    }

    // An environment's actor by name. Only the shown environment has any.
    fn prop(&mut self, stage: Entity, name: &str) -> Option<(Entity, Mut<'_, PropActor>)> {
        let stage_of = &self.stage_of;
        let mut props = self.props.iter_mut();
        props.find(|(entity, prop)| prop.name() == name && stage_of.get(*entity) == Some(stage))
    }

    // The stage's environment, once its scene has spawned.
    fn environment(&self, stage: Entity) -> Option<Entity> {
        let mut roots = self.environments.iter();
        roots.find(|(_, parent)| parent.parent() == stage).map(|(root, _)| root)
    }

    fn bone(&mut self, stage: Entity, actor: Option<&str>, bone: &str) -> Option<Entity> {
        let Some(name) = actor else {
            return self.character(stage)?.1.node(bone);
        };
        if let Some(found) = self.prop(stage, name).and_then(|(_, prop)| prop.node(bone)) {
            return Some(found);
        }
        let named = |entity: Entity, wanted: &str| {
            self.names.get(entity).is_ok_and(|own| own.as_str() == wanted)
        };
        let root = self.environment(stage)?;
        let armature = self.children.iter_descendants(root).find(|&e| named(e, name))?;
        self.children.iter_descendants(armature).find(|&e| named(e, bone))
    }

    fn still_playing(&self, played: &Played) -> bool {
        let mixer = match self.characters.get(played.actor) {
            Ok((_, character)) => character.mixer(),
            Err(_) => match self.props.get(played.actor) {
                Ok((_, prop)) => prop.mixer(),
                Err(_) => return false,
            },
        };
        mixer.playing(played.slot) == Some((played.clip, Mode::Once))
    }
}

// Where the pointer is over the desktop, as the window's own input or the desktop's
// forwarded to it tells. Unlike CursorPosition, it is gone while another app's window covers
// the desktop.
#[derive(Default)]
struct Pointer(Option<(Entity, Vec2)>);

fn react(
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    hover: Option<Res<HoverMap>>,
    mut moved: MessageReader<CursorMoved>,
    mut left: MessageReader<CursorLeft>,
    mut pointer: Local<Pointer>,
    windows: Query<&Window>,
    screens: Query<(&Screen, &ScreenParts)>,
    cameras: Query<(&Camera, &GlobalTransform), With<SceneCamera>>,
    mut stages: Query<(Entity, &GazeSettings, &mut Hotspots, Option<&CharacterClips>)>,
    globals: Query<&GlobalTransform>,
    mut actors: Actors,
) {
    for event in moved.read() {
        pointer.0 = Some((event.window, event.position));
    }
    if left.read().last().is_some() {
        pointer.0 = None;
    }
    // Over the panel, nothing under it reacts. Picking finds UI and the window under it alone.
    let hits = hover.as_ref().and_then(|map| map.get(&PointerId::Mouse));
    let on_panel = hits.is_some_and(|hits| hits.keys().any(|&hit| !windows.contains(hit)));
    let sight = pointer.0.filter(|_| !on_panel).and_then(|(window, position)| {
        let (_, parts) = screens.iter().find(|(screen, _)| screen.window == window)?;
        let (camera, camera_global) = cameras.get(parts.camera).ok()?;
        let window = windows.get(window).ok()?;
        let at = in_picture(camera, position / Vec2::new(window.width(), window.height()))?;
        let ray = camera.viewport_to_world(camera_global, at).ok()?;
        Some((actors.stage_of.get(parts.camera)?, ray, position))
    });
    let now = time.elapsed_secs_f64();

    for (stage, settings, mut hotspots, clips) in &mut stages {
        let hotspots = &mut *hotspots;
        end_finished(stage, hotspots, &mut actors);
        if hotspots.spots.is_empty() {
            continue;
        }
        let seen = sight.filter(|(on, ..)| *on == stage);
        let environment = actors.environment(stage);
        if hotspots.environment != environment {
            hotspots.environment = environment;
            hotspots.spots.iter_mut().for_each(|spot| spot.searched.fill(false));
        }
        for spot in &mut hotspots.spots {
            find_bones(stage, spot, &globals, &mut actors);
        }
        let front = |trigger: Trigger| {
            let (_, ray, _) = seen?;
            let hits =
                hotspots.spots.iter().enumerate().filter(|(_, s)| s.hotspot.trigger == trigger);
            hits.filter_map(|(at, spot)| Some((at, distance(spot, ray, &globals)?)))
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(at, _)| at)
        };
        let (clicked, hovered) = (front(Trigger::Click), front(Trigger::Hover));

        let mut due = Vec::new();
        let position = seen.map(|(.., position)| position);
        if buttons.just_pressed(MouseButton::Left) {
            hotspots.pressed = clicked.zip(position);
        }
        if buttons.just_released(MouseButton::Left)
            && let Some((at, from)) = hotspots.pressed.take()
            && clicked == Some(at)
            && position.is_some_and(|to| to.distance(from) <= CLICK_SLOP)
        {
            due.push(at);
        }
        for (at, spot) in hotspots.spots.iter_mut().enumerate() {
            if spot.hotspot.trigger != Trigger::Hover {
                continue;
            }
            match (&spot.hovered, hovered == Some(at)) {
                (_, false) => spot.hovered = Hovered::Off,
                (Hovered::Off, true) => spot.hovered = Hovered::Since(now),
                (Hovered::Since(since), true) if now - since >= HOVER_SECS => {
                    due.push(at);
                    spot.hovered = Hovered::WentOff;
                }
                _ => {}
            }
        }
        if !settings.reactions {
            continue;
        }
        for at in due {
            fire(stage, at, now, hotspots, clips, &mut actors);
        }
    }
}

// Drops what has stopped playing: run out, or taken over by another reaction. A hotspot's
// expression goes once all its clips have.
fn end_finished(stage: Entity, hotspots: &mut Hotspots, actors: &mut Actors) {
    for spot in &mut hotspots.spots {
        spot.playing.retain(|played| actors.still_playing(played));
    }
    if let Some(owner) = hotspots.face
        && hotspots.spots[owner].playing.is_empty()
    {
        hotspots.face = None;
        if let Some((_, mut character)) = actors.character(stage) {
            character.reaction_face(None);
        }
    }
    let gaze_off = hotspots.spots.iter().flat_map(|s| &s.playing).any(|played| !played.gaze);
    if hotspots.gaze_off != gaze_off {
        hotspots.gaze_off = gaze_off;
    }
}

// A bone found earlier stays found until its entity goes: the skin or environment it was in
// switched. An environment is searched once for the bones it lacks, not every frame.
fn find_bones(
    stage: Entity,
    spot: &mut Spot,
    globals: &Query<&GlobalTransform>,
    actors: &mut Actors,
) {
    let areas = spot.bones.iter_mut().zip(&mut spot.searched).zip(&spot.hotspot.areas);
    for ((bone, searched), area) in areas {
        if bone.is_some_and(|entity| globals.contains(entity)) {
            continue;
        }
        *bone = None;
        if area.actor.is_some() && *searched {
            continue;
        }
        *bone = actors.bone(stage, area.actor.as_deref(), &area.bone);
        *searched = area.actor.is_some() && actors.environment(stage).is_some();
    }
}

// How far along the ray the hotspot is, at the nearest of its capsules.
fn distance(spot: &Spot, ray: Ray3d, globals: &Query<&GlobalTransform>) -> Option<f32> {
    let reaches = spot.bones.iter().zip(&spot.hotspot.areas).filter_map(|(bone, area)| {
        let global = globals.get((*bone)?).ok()?;
        let head = global.translation();
        let tail = global.transform_point(Vec3::Y * area.length);
        let affine = global.affine();
        let scale =
            (affine.x_axis.length() + affine.y_axis.length() + affine.z_axis.length()) / 3.0;
        reach(ray.origin, *ray.direction, head, tail, area.radius * scale)
    });
    reaches.min_by(f32::total_cmp)
}

// How far along the ray (origin, unit direction) it enters the capsule around a→b, or None
// if it misses or the capsule is behind it. Found from where the ray passes closest to the
// bone: exact for a sphere and near enough for a capsule to tell what is in front.
fn reach(origin: Vec3, direction: Vec3, a: Vec3, b: Vec3, radius: f32) -> Option<f32> {
    let ab = b - a;
    let r = origin - a;
    let (e, f, c, along) = (ab.dot(ab), ab.dot(r), direction.dot(r), direction.dot(ab));
    let s_for = |t: f32| if e > f32::EPSILON { ((along * t + f) / e).clamp(0.0, 1.0) } else { 0.0 };
    let parallel = e - along * along <= f32::EPSILON * e.max(1.0);
    let t = if parallel { 0.0 } else { ((along * f - c * e) / (e - along * along)).max(0.0) };
    let t = (along * s_for(t) - c).max(0.0);
    let s = s_for(t);
    let gap = (origin + direction * t - (a + ab * s)).length_squared();
    let inside = radius * radius - gap;
    (inside >= 0.0).then(|| (t - inside.sqrt()).max(0.0))
}

fn fire(
    stage: Entity,
    at: usize,
    now: f64,
    hotspots: &mut Hotspots,
    clips: Option<&CharacterClips>,
    actors: &mut Actors,
) {
    let spot = &hotspots.spots[at];
    let cooldown = f64::from(spot.hotspot.cooldown.unwrap_or(0.0));
    if spot.fired.is_some_and(|fired| now - fired < cooldown) || !spot.playing.is_empty() {
        return;
    }
    let layer = spot.hotspot.layer;
    let mut played = Vec::new();
    for at_pick in 0..spot.picks.len() {
        let pick = &hotspots.spots[at].picks[at_pick];
        let choice = random_other(&mut hotspots.seed, pick.clips.len(), pick.last);
        let name = &pick.clips[choice];
        let started = match pick.actor.as_deref() {
            None => actors.character(stage).and_then(|(entity, mut character)| {
                let id = character.clip_id(name)?;
                let slot = CHARACTER_REACTIONS.play(character.mixer_mut(), id, layer);
                let gaze = clips.and_then(|clips| clips.get(name)).is_none_or(|clip| clip.gaze);
                Some(Played { actor: entity, slot, clip: id, gaze })
            }),
            Some(owner) => actors.prop(stage, owner).and_then(|(entity, mut prop)| {
                let id = prop.clip_id(name)?;
                let slot = PROP_REACTIONS.play(prop.mixer_mut(), id, layer);
                Some(Played { actor: entity, slot, clip: id, gaze: true })
            }),
        };
        if let Some(started) = started {
            hotspots.spots[at].picks[at_pick].last = Some(choice);
            played.push(started);
        }
    }
    // Nothing it plays has loaded, or its actors are in another environment.
    if played.is_empty() {
        return;
    }
    let spot = &mut hotspots.spots[at];
    spot.playing = played;
    spot.fired = Some(now);
    if let Some(expression) = &spot.hotspot.expression
        && let Some((_, mut character)) = actors.character(stage)
    {
        character.reaction_face(Some(expression));
        hotspots.face = Some(at);
    }
}

// Any of `count`, but not `last` while there are others.
fn random_other(seed: &mut u64, count: usize, last: Option<usize>) -> usize {
    // xorshift64: plenty for picking a clip.
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    match last.filter(|_| count > 1) {
        Some(last) => (last + 1 + (*seed % (count as u64 - 1)) as usize) % count,
        None => (*seed % count.max(1) as u64) as usize,
    }
}
