// The suite's hotspots: things that react to the mouse. Most are capsules around bones that
// go off when clicked, or when the cursor rests on them; `idle` and `welcome` ones go off when
// the mouse keeps still, or comes back. A hotspot finds its bones by name, on the character or
// on an armature of the shown environment, so it follows every move and is back after every
// switch of skin or environment. The armature needs no clips of its own.
//
// Each frame runs in four steps, one system each: `mouse` works out where the pointer is and
// how long the mouse has kept still; `keep_up` drops what has ended and stops hotspots whose
// actors no longer play what they wait for; `notice` turns the mouse into hotspots due to go
// off and hotspots let go of; `switch` and `go_off` carry those out.
//
// A hotspot goes through phases (see `Phase`). Reactions play in slots above everything else
// the actor plays, so what played before shows again as they fade. One that replaces plays in
// the actor's reaction slot and stops its layers; a layered one takes the layer slot of one
// moving the same parts, if any. Its `while` clips play there on and on while it's held, and
// its once-only clips above them, in the top layer, so they fade into the loops as they end.
//
// `then` switches actors once the hotspot lets go, or its clips end: at once, underneath the
// reaction fading out, so the reaction's last pose crossfades into the new animation.

mod actors;
mod areas;
mod mouse;
mod react;

use std::ops::Range;

use bevy::prelude::*;
use charpaper_anim::Actor;
use charpaper_anim::ClipId;
use charpaper_anim::Mode;
use charpaper_suite::Hotspot;

use crate::SceneSet;
use crate::character::play_selected;
use crate::suite::ActiveSuite;
pub(crate) use areas::reach;
pub(crate) use mouse::Mouse;

use actors::Played;

pub(crate) struct HotspotsPlugin;

impl Plugin for HotspotsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Mouse>()
            .add_systems(Update, list_hotspots.in_set(SceneSet::Fill))
            // A switch plays in the same frame, before the first sounds of what it ended start.
            .add_systems(
                Update,
                (mouse::mouse, react::keep_up, react::notice, react::switch, react::go_off)
                    .chain()
                    .in_set(SceneSet::Run)
                    .before(play_selected),
            );
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
    // The hotspots due to go off this frame, in the order noticed.
    due: Vec<usize>,
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

impl Hotspots {
    // While one is yet to switch what actors play, others wait: they may wait for it.
    fn switching(&self) -> bool {
        self.spots.iter().any(Spot::switches_later)
    }
}

struct Spot {
    hotspot: Hotspot,
    // Each area's bone, once found, and whether the shown environment was searched for it.
    bones: Vec<Option<Entity>>,
    searched: Vec<bool>,
    // The clips of each actor it plays, with the one picked last.
    picks: Vec<Pick>,
    phase: Phase,
    playing: Vec<Played>,
    // Whether its actors play what it waits for.
    allowed: bool,
    hover: Hover,
    history: History,
}

// Ready → Playing (its clips play once) or Held (its `while` clips play on and on) → Switching,
// if it has a `then`, which carries out the switch that frame → Ending (what it played once
// plays out) → Ready. Without a `then` it skips Switching. Stopped, it is Ready at once and
// doesn't switch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
    #[default]
    Ready,
    Playing,
    Held,
    Switching,
    Ending,
}

impl Spot {
    fn switches_later(&self) -> bool {
        let coming = matches!(self.phase, Phase::Playing | Phase::Held | Phase::Switching);
        coming && !self.hotspot.then.is_empty()
    }

    // Once let go, or its clips have ended: its switch, if any, is due.
    fn done(&mut self) {
        self.phase = if !self.hotspot.then.is_empty() {
            Phase::Switching
        } else if self.playing.is_empty() {
            Phase::Ready
        } else {
            Phase::Ending
        };
    }
}

struct Pick {
    actor: Option<String>,
    // Each clip with the time in a row it plays from.
    clips: Vec<(String, u32)>,
    last: Option<usize>,
}

impl Pick {
    // The clips for the furthest time in a row reached, or none if every one waits for later.
    fn tier(&self, streak: u32) -> Vec<usize> {
        let froms = self.clips.iter().map(|(_, from)| *from);
        let Some(tier) = froms.filter(|&from| from <= streak).max() else {
            return Vec::new();
        };
        (0..self.clips.len()).filter(|&at| self.clips[at].1 == tier).collect()
    }
}

// When it last went off and ended, and how often in a row: game time, but real time for
// `idled`, as idle goes by the mouse.
#[derive(Default)]
struct History {
    fired: Option<f64>,
    ended: Option<f64>,
    idled: Option<f64>,
    streak: u32,
}

#[derive(Default)]
enum Hover {
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
                phase: Phase::Ready,
                playing: Vec::new(),
                allowed: true,
                hover: Hover::Off,
                history: History::default(),
            }
        });
        let seed = stage.to_bits() | 1;
        commands.entity(stage).insert(Hotspots { spots: spots.collect(), seed, ..default() });
    }
}
