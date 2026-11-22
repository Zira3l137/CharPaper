// The suite's hotspots: capsules around bones that play clips when clicked, or when the cursor
// rests on them. A hotspot finds its bones by name, on the character or on an armature of the
// shown environment, so it follows every move and is back after every switch of skin or
// environment. The armature needs no clips of its own.
//
// Reactions play in slots above everything else the actor plays, so what played before shows
// again as they fade. One that replaces plays in the actor's reaction slot and stops its
// layers; a layered one takes the layer slot of one moving the same parts, if any.
//
// A hotspot is held while the button pressed on it stays down, the cursor rests on it, or the
// mouse keeps still, by trigger. Its `while` clips play on and on in its reaction slot while it
// is, and its once-only clips over them, in the top layer, so they fade into the loops as they
// end. A click hotspot with `while` clips goes off as it's pressed rather than as it's let go.
//
// `then` switches actors once the hotspot lets go, or its clips end: at once, underneath the
// reaction fading out, so the reaction's last pose crossfades into the new animation.
//
// Idle and welcome hotspots go by real time: the app may be drawing a frame a second while
// nobody looks, and virtual time would barely move then.

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
use charpaper_suite::ActorClip;
use charpaper_suite::Hotspot;
use charpaper_suite::Trigger;

use crate::CharacterState;
use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::character::CharacterActor;
use crate::character::CharacterClips;
use crate::character::REACTIONS as CHARACTER_REACTIONS;
use crate::character::play_selected;
use crate::environment::EnvironmentReady;
use crate::props::PropActor;
use crate::props::REACTIONS as PROP_REACTIONS;
use crate::props::base_mode;
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
// How soon after it can go off again it has to, for that to count as in a row.
const STREAK_SECS: f64 = 3.0;

pub(crate) struct HotspotsPlugin;

impl Plugin for HotspotsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, list_hotspots.in_set(SceneSet::Fill))
            // A switch it makes plays in the same frame, before its clips' first sounds start.
            .add_systems(Update, react.in_set(SceneSet::Run).before(play_selected));
    }
}

// Where an actor's reactions play.
pub(crate) struct Reactions {
    pub replacing: usize,
    pub layers: Range<usize>,
}

impl Reactions {
    // Above whatever else the hotspot plays on the actor.
    fn top(&self, actor: &mut Actor, clip: ClipId) -> usize {
        let slot = self.layers.end - 1;
        actor.play(slot, clip, Mode::Once);
        slot
    }

    fn play(&self, actor: &mut Actor, clip: ClipId, mode: Mode, layer: bool) -> usize {
        if layer {
            let slot = actor.slot_for(self.layers.clone(), clip);
            actor.play(slot, clip, mode);
            return slot;
        }
        self.layers.clone().for_each(|slot| actor.stop(slot));
        actor.play(self.replacing, clip, mode);
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
    // Whether it's held, playing its `while` clips, and whether its `then` is still to come.
    holding: bool,
    switching: bool,
    // Whether its actors' animations are ones it may go off during.
    open: bool,
    // How many times in a row it went off, and when its clips last ended.
    streak: u32,
    ended: Option<f64>,
    // When an idle hotspot last went off, in real time.
    idled: Option<f64>,
}

struct Pick {
    actor: Option<String>,
    // Each clip with the time in a row it plays from.
    clips: Vec<(String, u32)>,
    last: Option<usize>,
}

// When the mouse last moved or pressed, in real time, and how long it had kept still before
// it did this frame.
#[derive(Default)]
struct Activity {
    last: Option<f64>,
    came_back_after: Option<f64>,
}

struct Played {
    actor: Entity,
    slot: usize,
    clip: ClipId,
    mode: Mode,
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
                let clip = (wanted.clip.clone(), wanted.from.unwrap_or(1));
                match picks.iter_mut().find(|pick| pick.actor == wanted.actor) {
                    Some(pick) => pick.clips.push(clip),
                    None => picks.push(Pick {
                        actor: wanted.actor.clone(),
                        clips: vec![clip],
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
                holding: false,
                switching: false,
                open: true,
                streak: 0,
                ended: None,
                idled: None,
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

    // What the actor plays on and on.
    fn base(&mut self, stage: Entity, actor: Option<&str>) -> Option<String> {
        match actor {
            None => self.character(stage)?.1.base().map(str::to_owned),
            Some(name) => self.prop(stage, name)?.1.base().map(str::to_owned),
        }
    }

    // Whether every actor it waits on plays one of the animations it waits for.
    fn allows(&mut self, stage: Entity, during: &[ActorClip]) -> bool {
        during.iter().all(|wanted| {
            let base = self.base(stage, wanted.actor.as_deref());
            let fits = |other: &&ActorClip| other.actor == wanted.actor;
            during.iter().filter(fits).any(|other| base.as_deref() == Some(other.clip.as_str()))
        })
    }

    fn still_playing(&self, played: &Played) -> bool {
        let mixer = match self.characters.get(played.actor) {
            Ok((_, character)) => character.mixer(),
            Err(_) => match self.props.get(played.actor) {
                Ok((_, prop)) => prop.mixer(),
                Err(_) => return false,
            },
        };
        mixer.playing(played.slot) == Some((played.clip, played.mode))
    }

    // Fades out what the hotspot played, or only its `while` clips.
    fn stop(&mut self, played: &Played) {
        let mixer = match self.characters.get_mut(played.actor) {
            Ok((_, character)) => character.into_inner().mixer_mut(),
            Err(_) => match self.props.get_mut(played.actor) {
                Ok((_, prop)) => prop.into_inner().mixer_mut(),
                Err(_) => return,
            },
        };
        if mixer.playing(played.slot) == Some((played.clip, played.mode)) {
            mixer.stop(played.slot);
        }
    }
}

// Where the pointer is over the desktop, as the window's own input or the desktop's
// forwarded to it tells. Unlike CursorPosition, it is gone while another app's window covers
// the desktop.
#[derive(Default)]
struct Pointer(Option<(Entity, Vec2)>);

type StageHotspots = (
    Entity,
    &'static GazeSettings,
    &'static mut Hotspots,
    Option<&'static CharacterClips>,
    Option<&'static mut CharacterState>,
);

fn react(
    time: Res<Time>,
    real: Res<Time<Real>>,
    buttons: Res<ButtonInput<MouseButton>>,
    hover: Option<Res<HoverMap>>,
    mut moved: MessageReader<CursorMoved>,
    mut left: MessageReader<CursorLeft>,
    mut pointer: Local<Pointer>,
    mut activity: Local<Activity>,
    windows: Query<&Window>,
    screens: Query<(&Screen, &ScreenParts)>,
    cameras: Query<(&Camera, &GlobalTransform), With<SceneCamera>>,
    mut stages: Query<StageHotspots>,
    globals: Query<&GlobalTransform>,
    mut actors: Actors,
) {
    let real_now = real.elapsed_secs_f64();
    let last_active = *activity.last.get_or_insert(real_now);
    let mut active = buttons.get_just_pressed().next().is_some();
    for event in moved.read() {
        pointer.0 = Some((event.window, event.position));
        active = true;
    }
    activity.came_back_after = active.then_some(real_now - last_active);
    if active {
        activity.last = Some(real_now);
    }
    if left.read().last().is_some() {
        pointer.0 = None;
    }
    // Over the panel, nothing under it reacts. Picking finds UI and the window under it alone,
    // and the scene image is UI too: it has to be pickable, since the orbit camera is steered
    // by drags and scrolls on it.
    let hits = hover.as_ref().and_then(|map| map.get(&PointerId::Mouse));
    let on_panel = hits.is_some_and(|hits| {
        hits.keys().any(|&hit| {
            !windows.contains(hit) && !screens.iter().any(|(_, parts)| parts.image_node == hit)
        })
    });
    let sight = pointer.0.filter(|_| !on_panel).and_then(|(window, position)| {
        let (_, parts) = screens.iter().find(|(screen, _)| screen.window == window)?;
        let (camera, camera_global) = cameras.get(parts.camera).ok()?;
        let window = windows.get(window).ok()?;
        let at = in_picture(camera, position / Vec2::new(window.width(), window.height()))?;
        let ray = camera.viewport_to_world(camera_global, at).ok()?;
        Some((actors.stage_of.get(parts.camera)?, ray, position))
    });
    let now = time.elapsed_secs_f64();

    for (stage, settings, mut hotspots, clips, mut state) in &mut stages {
        let hotspots = &mut *hotspots;
        end_finished(stage, now, hotspots, &mut actors);
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
        for spot in &mut hotspots.spots {
            spot.open = actors.allows(stage, &spot.hotspot.during);
            if !spot.open {
                stop(spot, now, &mut actors);
            }
        }
        let front = |trigger: Trigger| {
            let (_, ray, _) = seen?;
            let hits = hotspots
                .spots
                .iter()
                .enumerate()
                .filter(|(_, s)| s.open && s.hotspot.trigger == trigger);
            hits.filter_map(|(at, spot)| Some((at, distance(spot, ray, &globals)?)))
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(at, _)| at)
        };
        let (clicked, hovered) = (front(Trigger::Click), front(Trigger::Hover));

        let mut due = Vec::new();
        let position = seen.map(|(.., position)| position);
        let held = |at: usize| !hotspots.spots[at].hotspot.meanwhile.is_empty();
        if buttons.just_pressed(MouseButton::Left) {
            hotspots.pressed = clicked.zip(position);
            if let Some(at) = clicked.filter(|&at| held(at)) {
                due.push(at);
            }
        }
        if buttons.just_released(MouseButton::Left)
            && let Some((at, from)) = hotspots.pressed.take()
            && clicked == Some(at)
            && position.is_some_and(|to| to.distance(from) <= CLICK_SLOP)
            && !held(at)
        {
            due.push(at);
        }
        if !buttons.pressed(MouseButton::Left) {
            for spot in &mut hotspots.spots {
                if spot.holding && spot.hotspot.trigger == Trigger::Click {
                    release(spot, now, &mut actors);
                }
            }
        }
        for (at, spot) in hotspots.spots.iter_mut().enumerate() {
            if spot.hotspot.trigger != Trigger::Hover {
                continue;
            }
            match (&spot.hovered, hovered == Some(at)) {
                (_, false) => {
                    spot.hovered = Hovered::Off;
                    if spot.holding {
                        release(spot, now, &mut actors);
                    }
                }
                (Hovered::Off, true) => spot.hovered = Hovered::Since(now),
                (Hovered::Since(since), true) if now - since >= HOVER_SECS => {
                    due.push(at);
                    spot.hovered = Hovered::WentOff;
                }
                _ => {}
            }
        }
        let still = real_now - activity.last.unwrap_or(real_now);
        for (at, spot) in hotspots.spots.iter_mut().enumerate() {
            let after = f64::from(spot.hotspot.after_secs());
            match spot.hotspot.trigger {
                Trigger::Idle if activity.came_back_after.is_some() && spot.holding => {
                    release(spot, now, &mut actors);
                }
                Trigger::Idle if spot.open && still >= after => {
                    // Again every `after` seconds while the mouse stays still.
                    let stale = |idled: f64| real_now - idled >= after || idled < real_now - still;
                    if spot.idled.is_none_or(stale) {
                        due.push(at);
                    }
                }
                Trigger::Welcome if activity.came_back_after.is_some_and(|gap| gap >= after) => {
                    due.push(at);
                }
                _ => {}
            }
        }
        for spot in &mut hotspots.spots {
            // Without `while` clips to let go of, it switches once its clips have ended.
            let let_go =
                !spot.holding && (!spot.hotspot.meanwhile.is_empty() || spot.playing.is_empty());
            if spot.switching && let_go {
                spot.switching = false;
                switch(stage, &spot.hotspot.then, state.as_deref_mut(), clips, &mut actors);
            }
        }
        // A hotspot about to switch what actors play is left to, so its switch makes sense.
        if !settings.reactions || hotspots.spots.iter().any(|spot| spot.switching) {
            continue;
        }
        for at in due {
            let spot = &hotspots.spots[at];
            let in_row = match spot.hotspot.trigger {
                Trigger::Idle => spot.idled.is_some_and(|idled| idled >= real_now - still),
                Trigger::Welcome => false,
                Trigger::Click | Trigger::Hover => spot.ended.is_some_and(|ended| {
                    let cooldown = f64::from(spot.hotspot.cooldown.unwrap_or(0.0));
                    let free = spot.fired.map_or(ended, |fired| ended.max(fired + cooldown));
                    now - free <= STREAK_SECS
                }),
            };
            let played = fire(stage, at, (now, in_row), hotspots, clips, &mut actors);
            if !played {
                continue;
            }
            let spot = &mut hotspots.spots[at];
            if spot.hotspot.trigger == Trigger::Idle {
                spot.idled = Some(real_now);
            }
            // Others may wait for what it switches to: they go off only once it has.
            if spot.switching {
                break;
            }
        }
    }
}

// Switches each actor to its `then` clip: the character as if the viewer had picked it.
fn switch(
    stage: Entity,
    then: &[ActorClip],
    state: Option<&mut CharacterState>,
    clips: Option<&CharacterClips>,
    actors: &mut Actors,
) {
    let mut state = state;
    for wanted in then {
        match wanted.actor.as_deref() {
            None => {
                if let (Some(state), Some(clips), Some((_, mut character))) =
                    (state.as_deref_mut(), clips, actors.character(stage))
                    && character.cut_to(&wanted.clip, clips)
                {
                    state.animation = Some(wanted.clip.clone());
                }
            }
            Some(name) => {
                if let Some((_, mut prop)) = actors.prop(stage, name) {
                    prop.switch(&wanted.clip);
                }
            }
        }
    }
}

// Lets go of its `while` clips; what it plays once plays on.
fn release(spot: &mut Spot, now: f64, actors: &mut Actors) {
    spot.holding = false;
    spot.ended = Some(now);
    spot.playing.retain(|played| {
        if played.mode == Mode::Once {
            return true;
        }
        actors.stop(played);
        false
    });
}

// A reaction meant for another animation would look wrong over this one: it fades out.
// Its switch, too, is off: what was switched away from no longer is.
fn stop(spot: &mut Spot, now: f64, actors: &mut Actors) {
    spot.holding = false;
    spot.switching = false;
    if !spot.playing.is_empty() {
        spot.ended = Some(now);
    }
    for played in spot.playing.drain(..) {
        actors.stop(&played);
    }
}

// Drops what has stopped playing: run out, or taken over by another reaction. A hotspot's
// expression goes once all its clips have.
fn end_finished(stage: Entity, now: f64, hotspots: &mut Hotspots, actors: &mut Actors) {
    for spot in &mut hotspots.spots {
        let was = !spot.playing.is_empty();
        spot.playing.retain(|played| actors.still_playing(played));
        if was && spot.playing.is_empty() {
            spot.ended = Some(now);
        }
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

// Whether it went off: it may be cooling down, still playing, or have nothing loaded to play.
fn fire(
    stage: Entity,
    at: usize,
    (now, in_row): (f64, bool),
    hotspots: &mut Hotspots,
    clips: Option<&CharacterClips>,
    actors: &mut Actors,
) -> bool {
    let spot = &hotspots.spots[at];
    let cooldown = f64::from(spot.hotspot.cooldown.unwrap_or(0.0));
    if spot.fired.is_some_and(|fired| now - fired < cooldown) || !spot.playing.is_empty() {
        return false;
    }
    let streak = if in_row { spot.streak + 1 } else { 1 };
    let layer = spot.hotspot.layer;
    // The `while` clips first, so the clips it plays once play over them.
    let mut played: Vec<Played> = (spot.hotspot.meanwhile.iter())
        .filter_map(|wanted| {
            start(stage, wanted.actor.as_deref(), &wanted.clip, Slot::Loop(layer), clips, actors)
        })
        .collect();
    let once = if played.is_empty() { Slot::Once(layer) } else { Slot::Top };
    let mut waiting = false;
    for at_pick in 0..spot.picks.len() {
        let pick = &hotspots.spots[at].picks[at_pick];
        // The clips for the furthest time in a row reached so far.
        let Some(tier) = pick.clips.iter().map(|(_, from)| *from).filter(|&f| f <= streak).max()
        else {
            waiting = true;
            continue;
        };
        let choices: Vec<usize> =
            (0..pick.clips.len()).filter(|&i| pick.clips[i].1 == tier).collect();
        let last = pick.last.and_then(|last| choices.iter().position(|&i| i == last));
        let choice = choices[random_other(&mut hotspots.seed, choices.len(), last)];
        let actor = pick.actor.clone();
        let name = pick.clips[choice].0.clone();
        if let Some(started) = start(stage, actor.as_deref(), &name, once, clips, actors) {
            hotspots.spots[at].picks[at_pick].last = Some(choice);
            played.push(started);
        }
    }
    // Nothing it plays has loaded, or its actors are in another environment. Clips that wait for
    // a later time in a row still count this one.
    if played.is_empty() && !waiting {
        return false;
    }
    let spot = &mut hotspots.spots[at];
    spot.holding = played.iter().any(|played| played.mode != Mode::Once);
    spot.switching = !spot.hotspot.then.is_empty();
    spot.playing = played;
    spot.fired = Some(now);
    spot.streak = streak;
    if spot.playing.is_empty() {
        spot.ended = Some(now);
    }
    if let Some(expression) = &spot.hotspot.expression
        && let Some((_, mut character)) = actors.character(stage)
    {
        character.reaction_face(Some(expression));
        hotspots.face = Some(at);
    }
    true
}

// Where a clip a hotspot plays goes: on and on in its reaction slot or a layer, or once there,
// or once above its `while` clips.
#[derive(Clone, Copy)]
enum Slot {
    Loop(bool),
    Once(bool),
    Top,
}

fn start(
    stage: Entity,
    actor: Option<&str>,
    name: &str,
    slot: Slot,
    clips: Option<&CharacterClips>,
    actors: &mut Actors,
) -> Option<Played> {
    let place = |reactions: &Reactions, mixer: &mut Actor, id, mode: Option<Mode>| {
        Some(match slot {
            Slot::Loop(layer) => (reactions.play(mixer, id, mode?, layer), mode?),
            Slot::Once(layer) => (reactions.play(mixer, id, Mode::Once, layer), Mode::Once),
            Slot::Top => (reactions.top(mixer, id), Mode::Once),
        })
    };
    match actor {
        None => {
            let (entity, mut character) = actors.character(stage)?;
            let id = character.clip_id(name)?;
            let clip = clips.and_then(|clips| clips.get(name));
            let mode = clip.and_then(|clip| base_mode(clip.mode));
            let (slot, mode) = place(&CHARACTER_REACTIONS, character.mixer_mut(), id, mode)?;
            let gaze = clip.is_none_or(|clip| clip.gaze);
            Some(Played { actor: entity, slot, clip: id, mode, gaze })
        }
        Some(owner) => {
            let (entity, mut prop) = actors.prop(stage, owner)?;
            let id = prop.clip_id(name)?;
            let mode = prop.clip_mode(name).and_then(base_mode);
            let (slot, mode) = place(&PROP_REACTIONS, prop.mixer_mut(), id, mode)?;
            Some(Played { actor: entity, slot, clip: id, mode, gaze: true })
        }
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
