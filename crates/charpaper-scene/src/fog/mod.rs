// Volumetric fog. Each fog object named in suite.toml gets a box of fog shaped by its mesh's
// bounds, and its own mesh is hidden. Fog shows only where lights light it, and every light
// that does adds to the work of every pixel the fog covers, so the camera only does fog while
// there is fog to draw. pass.rs draws it.

mod pass;

use bevy::camera::primitives::Aabb;
use bevy::light::FogVolume;
use bevy::light::VolumetricLight;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::world_serialization::WorldInstanceReady;
use charpaper_suite::Fog;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::environment::ShownEnvironment;
use crate::environment::light_listed;
use crate::environment::shown_environment;
use crate::environment::spawn_environment_scenes;
use crate::fog::pass::SceneFog;
use crate::look::ActiveLook;
use crate::look::Resolved;
use crate::look::look_needs_applying;
use crate::render::FogResolution;
use crate::render::Graphics;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

pub(crate) struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(pass::FogPassPlugin)
            .add_observer(spawn_fog)
            .add_systems(
                Update,
                light_fog
                    .run_if(look_needs_applying.or_eager(any_match_filter::<Changed<Graphics>>))
                    .in_set(SceneSet::Run)
                    .after(spawn_environment_scenes),
            )
            .add_systems(PostUpdate, sample_fog.after(TransformSystems::Propagate));
    }
}

fn spawn_fog(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    stages: Query<(&ShownEnvironment, Option<&ActiveSuite>)>,
    children: Query<&Children>,
    names: Query<&Name>,
    bounds: Query<&Aabb, With<Mesh3d>>,
) {
    let Some((shown, suite)) = stages.iter().find(|(shown, _)| shown.root == Some(ready.entity))
    else {
        return;
    };
    let Some(environment) = shown_environment(suite, shown) else {
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

type FogLights<'w, 's> = Query<
    'w,
    's,
    Has<DirectionalLight>,
    Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>,
>;

fn light_fog(
    mut commands: Commands,
    stages: Query<(
        Entity,
        &Graphics,
        Option<&ActiveLook>,
        &ShownEnvironment,
        Option<&ActiveSuite>,
    )>,
    cameras: Query<Entity, With<SceneCamera>>,
    stage_of: StageOf,
    volumes: Query<(), With<FogVolume>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    lights: FogLights,
) {
    for (stage, graphics, look, shown, suite) in &stages {
        // Only this stage's fog counts: the fog boxes are under its environment.
        let has_fog = shown
            .root
            .is_some_and(|root| children.iter_descendants(root).any(|e| volumes.contains(e)));
        let on = graphics.fog && has_fog;
        for camera in cameras.iter().filter(|&camera| stage_of.get(camera) == Some(stage)) {
            if on {
                commands.entity(camera).insert(scene_fog(graphics, 0.0));
            } else {
                commands.entity(camera).remove::<SceneFog>();
            }
        }
        let Some(root) = shown.root else {
            continue;
        };
        let look = look.map(|l| l.0.clone()).unwrap_or_default();
        let shadows = Resolved::new(&look, shown.name.as_deref()).shadows;
        let settings = shown_environment(suite, shown).map(|e| &e.settings);
        let skipped = settings.map(|s| s.no_volume_scatter.as_slice()).unwrap_or_default();
        let unshadowed = settings.map(|s| s.no_shadows.as_slice()).unwrap_or_default();
        let lit = Lit { on, shadows, skipped, unshadowed };
        light_volumes(&mut commands, root, lit, &children, &parents, &names, &lights);
    }
}

struct Lit<'a> {
    on: bool,
    shadows: bool,
    skipped: &'a [String],
    unshadowed: &'a [String],
}

fn light_volumes(
    commands: &mut Commands,
    root: Entity,
    Lit { on, shadows, skipped, unshadowed }: Lit,
    children: &Query<&Children>,
    parents: &Query<&ChildOf>,
    names: &Query<&Name>,
    lights: &FogLights,
) {
    for entity in children.iter_descendants(root) {
        let Ok(directional) = lights.get(entity) else {
            continue;
        };
        let skip = light_listed(skipped, entity, names, parents);
        let shadowed = shadows && !light_listed(unshadowed, entity, names, parents);
        // A sun lights fog through its shadow map, so without shadows it can't.
        if on && !skip && (shadowed || !directional) {
            commands.entity(entity).insert(VolumetricLight);
        } else {
            commands.entity(entity).remove::<VolumetricLight>();
        }
    }
}

// Every frame, once transforms are final, since fog boxes can move and scale.
fn sample_fog(
    volumes: Query<&GlobalTransform, With<FogVolume>>,
    mut fogs: Query<(Entity, &mut SceneFog)>,
    stage_of: StageOf,
    graphics: Query<&Graphics>,
) {
    // Starting each pixel's samples up to one step further along turns the stripes between
    // steps into fine noise. Bevy wants that as a distance, so the step is measured along the
    // longest way through any fog box: its diagonal.
    let longest = volumes.iter().map(|v| v.scale().length()).fold(0.0, f32::max);
    for (camera, mut fog) in &mut fogs {
        let Some(graphics) = stage_of.get(camera).and_then(|s| graphics.get(s).ok()) else {
            continue;
        };
        let steps = graphics.fog_quality.steps();
        let jitter = if graphics.fog_dithering { longest / steps as f32 } else { 0.0 };
        fog.set_if_neq(scene_fog(graphics, jitter));
    }
}

fn scene_fog(graphics: &Graphics, jitter: f32) -> SceneFog {
    SceneFog {
        steps: graphics.fog_quality.steps(),
        jitter,
        half_resolution: graphics.fog_resolution == FogResolution::Half,
    }
}
