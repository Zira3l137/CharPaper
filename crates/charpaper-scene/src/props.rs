// Everything that moves besides the character: an environment's animated objects and
// armatures, and a moving camera. Each is mixed like the character, on its stage's timeline,
// so a prop keyed over the same frames as the character's clip moves in step with it.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use charpaper_anim::Actor;
use charpaper_anim::ClipId;
use charpaper_anim::Event;
use charpaper_anim::Light;
use charpaper_anim::Mode;
use charpaper_anim::Rig;
use charpaper_audio::AudioSource;
use charpaper_audio::Hush;
use charpaper_audio::Layer;
use charpaper_audio::Sound;
use charpaper_suite::ClipCue;
use charpaper_suite::ClipFile;
use charpaper_suite::ClipMode;
use charpaper_suite::Suite;

use crate::assets::asset_path;
use crate::clips::ClipAsset;
use crate::clips::Mixing;
use crate::clips::Timeline;
use crate::clips::due;
use crate::hotspots::Reactions;
use crate::stage::StageOf;

pub(crate) struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (add_loaded_clips, mix_props).chain().in_set(Mixing::Pose));
    }
}

// A prop's own clip plays in the first slot; reactions play above it.
const BASE: usize = 0;
pub(crate) const REACTIONS: Reactions = Reactions { replacing: 1, layers: 2..5 };
const SLOTS: usize = 5;
const FADE: Duration = Duration::from_millis(250);

#[derive(Component)]
pub(crate) struct PropActor {
    name: String,
    actor: Actor,
    nodes: Vec<Entity>,
    clips: Vec<PropClip>,
    sounds: BTreeMap<PathBuf, Handle<AudioSource>>,
    light: Option<LitBy>,
    started: bool,
    events: Vec<Event>,
}

struct PropClip {
    name: String,
    file: Handle<ClipAsset>,
    mode: ClipMode,
    cues: Vec<ClipCue>,
    id: Option<ClipId>,
}

// The light the prop is, with the strength and linear color its file gave it, which its clips'
// light curves multiply.
struct LitBy {
    entity: Entity,
    strength: f32,
    color: LinearRgba,
    written: Light,
}

// A sound one of a stage's props played, as a child of that stage.
#[derive(Component)]
pub(crate) struct PropSound;

type Lights<'w, 's> = Query<
    'w,
    's,
    AnyOf<(&'static mut PointLight, &'static mut SpotLight, &'static mut DirectionalLight)>,
>;

// The rig is the prop's node and every named node under it, at rest as they spawned, except
// those of `others`, which are props of their own.
pub(crate) fn rig_of(
    root: Entity,
    others: &HashSet<Entity>,
    children: &Query<&Children>,
    nodes: &Query<(&Name, &Transform)>,
) -> (Rig, Vec<Entity>) {
    let mut rig = Rig::default();
    let mut entities = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if let Ok((name, transform)) = nodes.get(node) {
            let rest = charpaper_anim::Transform {
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
            };
            rig.nodes.push((name.to_string(), rest));
            entities.push(node);
        }
        let below = children.get(node).map(|c| &**c).unwrap_or_default();
        pending.extend(below.iter().filter(|child| !others.contains(child)));
    }
    (rig, entities)
}

impl PropActor {
    pub(crate) fn new(
        name: &str,
        (rig, nodes): (Rig, Vec<Entity>),
        clips: &[ClipFile],
        (suite, assets): (&Suite, &AssetServer),
        light: Option<(Entity, f32, Color)>,
    ) -> Self {
        let mut sounds = BTreeMap::new();
        for cue in clips.iter().flat_map(|clip| &clip.cues) {
            sounds
                .entry(cue.sound.clone())
                .or_insert_with(|| assets.load(asset_path(suite, &cue.sound)));
        }
        let clips = clips
            .iter()
            .map(|clip| PropClip {
                name: clip.name.clone(),
                file: assets.load(asset_path(suite, &clip.json)),
                mode: clip.mode,
                cues: clip.cues.clone(),
                id: None,
            })
            .collect();
        let light = light.map(|(entity, strength, color)| LitBy {
            entity,
            strength,
            color: color.to_linear(),
            written: Light::default(),
        });
        Self {
            name: name.to_string(),
            actor: Actor::new(rig, SLOTS, FADE.as_secs_f32()),
            nodes,
            clips,
            sounds,
            light,
            started: false,
            events: Vec::new(),
        }
    }
}

impl PropActor {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn mixer(&self) -> &Actor {
        &self.actor
    }

    pub(crate) fn mixer_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    pub(crate) fn node(&self, name: &str) -> Option<Entity> {
        let at = self.actor.rig().nodes.iter().position(|(own, _)| own == name)?;
        self.nodes.get(at).copied()
    }

    // None while it is still loading.
    pub(crate) fn clip_id(&self, name: &str) -> Option<ClipId> {
        self.clips.iter().find(|clip| clip.name == name)?.id
    }
}

// The strength and color a light spawned with, on the node itself or the child glTF puts it on.
pub(crate) fn light_of(
    root: Entity,
    children: &Query<&Children>,
    lights: &Query<AnyOf<(&PointLight, &SpotLight, &DirectionalLight)>>,
) -> Option<(Entity, f32, Color)> {
    let below = children.get(root).map(|c| &**c).unwrap_or_default();
    std::iter::once(&root).chain(below).find_map(|&entity| match lights.get(entity).ok()? {
        (Some(point), _, _) => Some((entity, point.intensity, point.color)),
        (_, Some(spot), _) => Some((entity, spot.intensity, spot.color)),
        (_, _, Some(sun)) => Some((entity, sun.illuminance, sun.color)),
        (None, None, None) => None,
    })
}

// A prop plays its first timeline clip from the moment that clip has loaded, or holds its
// first hold clip. Its Once clips wait to be played on it.
fn add_loaded_clips(mut props: Query<&mut PropActor>, files: Res<Assets<ClipAsset>>) {
    for mut prop in &mut props {
        let prop = &mut *prop;
        for clip in prop.clips.iter_mut().filter(|clip| clip.id.is_none()) {
            if let Some(file) = files.get(&clip.file) {
                clip.id = Some(prop.actor.add_clip(file.0.clone()));
            }
        }
        if prop.started {
            continue;
        }
        let first = |mode: ClipMode| prop.clips.iter().find(|clip| clip.mode == mode);
        let Some(own) = first(ClipMode::Timeline).or_else(|| first(ClipMode::Hold)) else {
            prop.started = true;
            continue;
        };
        let Some(id) = own.id else {
            continue;
        };
        let mode = if own.mode == ClipMode::Hold { Mode::Hold } else { Mode::Timeline };
        prop.actor.play(BASE, id, mode);
        prop.started = true;
        debug!("prop {:?} plays its clip", prop.name);
    }
}

fn mix_props(
    mut commands: Commands,
    time: Res<Time>,
    hush: Res<Hush>,
    mut props: Query<(Entity, &mut PropActor)>,
    stage_of: StageOf,
    timelines: Query<&Timeline>,
    mut transforms: Query<&mut Transform>,
    mut lights: Lights,
) {
    for (entity, mut prop) in &mut props {
        let Some(stage) = stage_of.get(entity) else {
            continue;
        };
        let Ok(timeline) = timelines.get(stage) else {
            continue;
        };
        let prop = &mut *prop;
        prop.events.clear();
        prop.actor.update(timeline.seconds, time.delta_secs(), &mut prop.events);

        for (&node, pose) in prop.nodes.iter().zip(&prop.actor.pose().nodes) {
            if let Ok(mut transform) = transforms.get_mut(node) {
                let posed = Transform {
                    translation: pose.translation,
                    rotation: pose.rotation,
                    scale: pose.scale,
                };
                transform.set_if_neq(posed);
            }
        }

        let light = prop.actor.pose().light;
        if let Some(lit) = &mut prop.light
            && lit.written != light
            && let Ok((point, spot, sun)) = lights.get_mut(lit.entity)
        {
            let strength = lit.strength * light.strength;
            let [r, g, b] = light.color.to_array();
            let base = lit.color;
            let color = Color::linear_rgba(base.red * r, base.green * g, base.blue * b, base.alpha);
            if let Some(mut point) = point {
                (point.intensity, point.color) = (strength, color);
            }
            if let Some(mut spot) = spot {
                (spot.intensity, spot.color) = (strength, color);
            }
            if let Some(mut sun) = sun {
                (sun.illuminance, sun.color) = (strength, color);
            }
            lit.written = light;
        }

        if hush.0 {
            continue;
        }
        for event in &prop.events {
            let Event::Passed { clip, .. } = *event else {
                continue;
            };
            let Some(played) = prop.clips.iter().find(|c| c.id == Some(clip)) else {
                continue;
            };
            for cue in due(&played.cues, event) {
                let Some(source) = prop.sounds.get(&cue.sound) else {
                    continue;
                };
                commands.spawn((
                    PropSound,
                    ChildOf(stage),
                    Sound {
                        source: source.clone(),
                        layer: Layer::Character,
                        looped: false,
                        fade_in: Duration::ZERO,
                    },
                ));
            }
        }
    }
}
