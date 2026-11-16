// What an environment's lights take from suite.toml beyond glTF: the size of their bulb, and
// their keyed Strength and Color, played as a loop.

use std::collections::BTreeMap;

use bevy::app::AnimationSystems;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_suite::Light;
use charpaper_suite::LightAnimation;

use crate::environment::CharacterClock;
use crate::environment::EnvironmentRoot;
use crate::environment::SAME_LENGTH;
use crate::environment::ShownEnvironment;
use crate::environment::shown_environment;
use crate::look::LightTree;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

pub(crate) struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(set_up_lights)
            .add_systems(PostUpdate, animate_lights.after(AnimationSystems));
    }
}

// A light's keyed Strength and Color, and the strength and color its file gave it, which they
// multiply.
#[derive(Component)]
struct KeyedLight {
    animation: LightAnimation,
    strength: f32,
    color: LinearRgba,
    // Seconds into its loop by its own clock, for when it doesn't follow the character's.
    played: f32,
}

type Lights<'w, 's> = Query<
    'w,
    's,
    AnyOf<(&'static mut PointLight, &'static mut SpotLight, &'static mut DirectionalLight)>,
>;

// Bevy's glTF loader gives a point light no radius and a spot light its range as one, though
// glTF lights are points: a spot with a range of a few meters would shine like a soft panel
// that wide. Both get the Radius they have in Blender instead, or none.
//
// It also reads a light's color as sRGB, while glTF keeps it linear, as Blender shows it. Read
// that way, a tinted light came out darker and more saturated than in Blender.
fn set_up_lights(
    ready: On<WorldInstanceReady>,
    roots: Query<&ChildOf, With<EnvironmentRoot>>,
    stages: Query<(Option<&ActiveSuite>, &ShownEnvironment)>,
    tree: LightTree,
    mut lights: Lights,
    mut commands: Commands,
) {
    let Ok(stage) = roots.get(ready.entity) else {
        return;
    };
    let settings = stages
        .get(stage.parent())
        .ok()
        .and_then(|(suite, shown)| shown_environment(suite, shown))
        .map(|environment| &environment.settings.lights);

    for entity in tree.children.iter_descendants(ready.entity) {
        let Ok((point, spot, sun)) = lights.get_mut(entity) else {
            continue;
        };
        let light = settings.and_then(|table| entry(table, entity, &tree));
        let radius = light.and_then(|light| light.radius).unwrap_or(0.0).max(0.0);
        let loaded = match (point, spot, sun) {
            (Some(mut point), _, _) => {
                point.radius = radius;
                point.color = linear(point.color);
                (point.intensity, point.color)
            }
            (_, Some(mut spot), _) => {
                spot.radius = radius;
                spot.color = linear(spot.color);
                (spot.intensity, spot.color)
            }
            (_, _, Some(mut sun)) => {
                sun.color = linear(sun.color);
                (sun.illuminance, sun.color)
            }
            (None, None, None) => continue,
        };
        if let Some(animation) = light.and_then(|light| light.animation.clone()) {
            let (strength, color) = loaded;
            let color = color.to_linear();
            commands.entity(entity).insert(KeyedLight { animation, strength, color, played: 0.0 });
        }
    }
}

// The loader's sRGB channels hold the file's linear ones.
fn linear(color: Color) -> Color {
    let Srgba { red, green, blue, alpha } = color.to_srgba();
    Color::linear_rgba(red, green, blue, alpha)
}

// suite.toml names a light by its object, and glTF puts the light on a child of its object,
// named after its data. Either name finds it, the object's first.
fn entry<'a>(
    lights: &'a BTreeMap<String, Light>,
    entity: Entity,
    tree: &LightTree,
) -> Option<&'a Light> {
    let named = |e: Entity| tree.names.get(e).ok().and_then(|name| lights.get(name.as_str()));
    tree.parents
        .get(entity)
        .ok()
        .and_then(|parent| named(parent.parent()))
        .or_else(|| named(entity))
}

// As a prop clip does, a light's loop exactly as long as the character's playing clip keeps
// the character's time, so that the two stay in step as on Blender's one timeline. Others keep
// their own, which advances with the animation players'.
fn animate_lights(
    time: Res<Time>,
    clock: CharacterClock,
    stage_of: StageOf,
    mut lights: Query<(
        Entity,
        &mut KeyedLight,
        AnyOf<(&mut PointLight, &mut SpotLight, &mut DirectionalLight)>,
    )>,
) {
    let leads = clock.leads();
    for (entity, mut keyed, (point, spot, sun)) in &mut lights {
        keyed.played += time.delta_secs();
        let lead = stage_of.get(entity).and_then(|stage| leads.get(&stage));
        let seconds = match lead {
            Some(&(length, at)) if (length - keyed.animation.duration).abs() < SAME_LENGTH => at,
            _ => keyed.played,
        };
        let sample = keyed.animation.at(seconds);
        let strength = keyed.strength * sample.strength;
        let [r, g, b] = sample.color;
        let base = keyed.color;
        let color = Color::linear_rgba(base.red * r, base.green * g, base.blue * b, base.alpha);

        // Only a light whose values change is marked changed.
        if let Some(mut light) = point
            && (light.intensity != strength || light.color != color)
        {
            light.intensity = strength;
            light.color = color;
        }
        if let Some(mut light) = spot
            && (light.intensity != strength || light.color != color)
        {
            light.intensity = strength;
            light.color = color;
        }
        if let Some(mut light) = sun
            && (light.illuminance != strength || light.color != color)
        {
            light.illuminance = strength;
            light.color = color;
        }
    }
}
