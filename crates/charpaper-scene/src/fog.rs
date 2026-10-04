// Volumetric fog. Each fog object named in suite.toml gets a box of Bevy fog shaped by its
// mesh's bounds, and its own mesh is hidden. Fog shows only where lights light it, and every
// light that does adds a pass over the screen, so the camera only does fog while there is
// fog to draw.

use bevy::camera::primitives::Aabb;
use bevy::light::FogVolume;
use bevy::light::VolumetricFog;
use bevy::light::VolumetricLight;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_suite::Environment;
use charpaper_suite::Fog;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::environment::ShownEnvironment;
use crate::environment::spawn_environment_scenes;
use crate::look::ActiveLook;
use crate::look::Resolved;
use crate::look::look_needs_applying;
use crate::render::RenderSettings;
use crate::suite::ActiveSuite;

pub(crate) struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_fog).add_systems(
            Update,
            light_fog
                .run_if(look_needs_applying.or_eager(resource_changed::<RenderSettings>))
                .in_set(SceneSet::Run)
                .after(spawn_environment_scenes),
        );
    }
}

fn spawn_fog(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    shown: Res<ShownEnvironment>,
    suite: Option<Res<ActiveSuite>>,
    children: Query<&Children>,
    names: Query<&Name>,
    bounds: Query<&Aabb, With<Mesh3d>>,
) {
    if shown.root != Some(ready.entity) {
        return;
    }
    let Some(environment) = shown_environment(suite.as_deref(), &shown) else {
        return;
    };

    for (name, fog) in &environment.settings.fog {
        let Some(object) = children
            .iter_descendants(ready.entity)
            .find(|&e| names.get(e).is_ok_and(|n| n.as_str() == name))
        else {
            warn!("environment {:?} has no fog object {name:?}", environment.name);
            continue;
        };

        // A glTF mesh spawns as one child per primitive, each with bounds in the object's
        // own space.
        let meshes: Vec<Entity> = children
            .get(object)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&e| bounds.contains(e))
            .collect();
        let (min, max) = meshes
            .iter()
            .filter_map(|&e| bounds.get(e).ok())
            .map(|b| (Vec3::from(b.min()), Vec3::from(b.max())))
            .reduce(|(a_min, a_max), (b_min, b_max)| (a_min.min(b_min), a_max.max(b_max)))
            .unwrap_or((Vec3::NEG_ONE, Vec3::ONE));
        for mesh in meshes {
            commands.entity(mesh).insert(Visibility::Hidden);
        }

        // Bevy's fog fills a 1 m cube, so the box's size is its scale. Never quite flat: the
        // fog shader needs the inverse of this scale.
        let size = (max - min).max(Vec3::splat(0.001));
        commands.spawn((
            Name::new(format!("Fog {name}")),
            volume(fog),
            Transform::from_translation((min + max) / 2.0).with_scale(size),
            ChildOf(object),
        ));
    }
}

fn volume(fog: &Fog) -> FogVolume {
    let medium = fog.medium();
    let [r, g, b] = medium.color;
    FogVolume {
        fog_color: Color::linear_rgb(r, g, b),
        density_factor: medium.density,
        absorption: medium.absorption,
        scattering: medium.scattering,
        scattering_asymmetry: medium.anisotropy,
        ..default()
    }
}

fn light_fog(
    mut commands: Commands,
    settings: Res<RenderSettings>,
    look: Option<Res<ActiveLook>>,
    shown: Res<ShownEnvironment>,
    suite: Option<Res<ActiveSuite>>,
    camera: Single<Entity, With<SceneCamera>>,
    volumes: Query<(), With<FogVolume>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    lights: Query<
        Has<DirectionalLight>,
        Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>,
    >,
) {
    let on = settings.fog && !volumes.is_empty();
    if on {
        // No ambient: Bevy adds it evenly whatever the density, which no Blender fog does.
        commands.entity(*camera).insert(VolumetricFog { ambient_intensity: 0.0, ..default() });
    } else {
        commands.entity(*camera).remove::<VolumetricFog>();
    }

    let Some(root) = shown.root else {
        return;
    };
    let look = look.map(|l| l.0.clone()).unwrap_or_default();
    let shadows = Resolved::new(&look, shown.name.as_deref()).shadows;
    let skipped = shown_environment(suite.as_deref(), &shown)
        .map(|e| e.settings.no_volume_scatter.clone())
        .unwrap_or_default();
    // glTF puts a light on a child of its object, named after the light data.
    let named = |e: Entity| names.get(e).is_ok_and(|n| skipped.iter().any(|s| s == n.as_str()));

    for entity in children.iter_descendants(root) {
        let Ok(directional) = lights.get(entity) else {
            continue;
        };
        let skip = named(entity) || parents.get(entity).is_ok_and(|p| named(p.parent()));
        // A sun lights fog through its shadow map, so without shadows it can't.
        if on && !skip && (shadows || !directional) {
            commands.entity(entity).insert(VolumetricLight);
        } else {
            commands.entity(entity).remove::<VolumetricLight>();
        }
    }
}

fn shown_environment<'a>(
    suite: Option<&'a ActiveSuite>,
    shown: &ShownEnvironment,
) -> Option<&'a Environment> {
    let name = shown.name.as_deref()?;
    suite?.environments.iter().find(|e| e.name == name)
}
