// What an environment's lights take from suite.toml beyond glTF: the size of their bulb.

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_suite::Light;

use crate::environment::EnvironmentRoot;
use crate::environment::ShownEnvironment;
use crate::environment::shown_environment;
use crate::look::LightTree;
use crate::suite::ActiveSuite;

pub(crate) struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(set_up_lights);
    }
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
        match (point, spot, sun) {
            (Some(mut point), _, _) => {
                point.radius = radius;
                point.color = linear(point.color);
            }
            (_, Some(mut spot), _) => {
                spot.radius = radius;
                spot.color = linear(spot.color);
            }
            (_, _, Some(mut sun)) => sun.color = linear(sun.color),
            (None, None, None) => {}
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
