// An environment's sun shines from its sky's sun, the sky's brightest spot, turned with the
// sky, so that its light and shadows match the sun in the background whatever Blender had it
// at and however the sky is turned in the app. Each sky's sun is found once, on the CPU, in
// the skybox Bevy keeps after loading it; see charpaper_bake::sun_direction. A sun stops
// following when the look says so, or the sky has no sun, and goes back to how it came.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::render::render_resource::TextureDataOrder;
use bevy::render::render_resource::TextureFormat;
use bevy::transform::TransformSystems;

use crate::environment::ShownEnvironment;
use crate::look::ActiveLook;
use crate::look::DEFAULT_SUN_FOLLOWS_SKY;
use crate::look::LightTree;
use crate::look::Resolved;
use crate::sky::SceneSky;
use crate::stage::StageOf;

pub(crate) struct SunPlugin;

impl Plugin for SunPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkySuns>().add_systems(
            PostUpdate,
            (find_sky_suns, follow_sky_sun).chain().before(TransformSystems::Propagate),
        );
    }
}

// Where each sky's sun is, Y up and before the sky is turned; none for a sky without one.
#[derive(Resource, Default)]
struct SkySuns(HashMap<AssetId<Image>, Option<Vec3>>);

// On a sun following its sky: how it was turned before.
#[derive(Component)]
struct HomeRotation(Quat);

fn find_sky_suns(mut suns: ResMut<SkySuns>, skies: Query<&SceneSky>, images: Res<Assets<Image>>) {
    for sky in &skies {
        let id = sky.image.id();
        if suns.0.contains_key(&id) {
            continue;
        }
        let Some(image) = images.get(id) else {
            continue;
        };
        let sun = sky_sun(image);
        match sun {
            Some(sun) => info!("the sky's sun is towards {sun:.3}"),
            None => info!("the sky has no sun to follow"),
        }
        suns.0.insert(id, sun);
    }
}

// Only a skybox as CharPaper bakes them, which Bevy keeps with its largest level first.
fn sky_sun(image: &Image) -> Option<Vec3> {
    let descriptor = &image.texture_descriptor;
    let baked = descriptor.format == TextureFormat::Rgb9e5Ufloat
        && descriptor.size.depth_or_array_layers == 6
        && descriptor.size.width == descriptor.size.height
        && (descriptor.mip_level_count == 1 || image.data_order == TextureDataOrder::MipMajor);
    if !baked {
        return None;
    }
    charpaper_bake::sun_direction(descriptor.size.width, image.data.as_deref()?)
        .map(Vec3::from_array)
}

// A directional light looks down its local -Z, so it is turned to look away from the sun.
// Its parents are whatever the glTF put above it, last frame's place standing in for theirs.
fn follow_sky_sun(
    mut commands: Commands,
    suns: Res<SkySuns>,
    stages: Query<(Entity, Option<&ActiveLook>, &ShownEnvironment)>,
    skies: Query<(Entity, &SceneSky)>,
    stage_of: StageOf,
    tree: LightTree,
    places: Query<&GlobalTransform>,
    mut lights: Query<(&DirectionalLight, &mut Transform, Option<&HomeRotation>)>,
) {
    for (stage, look, shown) in &stages {
        let Some(root) = shown.root else {
            continue;
        };
        let name = shown.name.as_deref();
        let follows = look
            .map_or(DEFAULT_SUN_FOLLOWS_SKY, |look| Resolved::new(&look.0, name).sun_follows_sky);
        let sky = skies.iter().find(|&(camera, _)| stage_of.get(camera) == Some(stage));
        let sun = sky.and_then(|(_, sky)| Some(sky.rotation * (*suns.0.get(&sky.image.id())?)?));
        let towards_sun = sun.filter(|_| follows);

        let environment_suns: Vec<(Entity, f32)> = tree
            .children
            .iter_descendants(root)
            .filter_map(|entity| lights.get(entity).ok().map(|(sun, ..)| (entity, sun.illuminance)))
            .collect();
        let brightest = environment_suns.iter().max_by(|a, b| a.1.total_cmp(&b.1)).map(|s| s.0);

        for &(entity, _) in &environment_suns {
            let Ok((_, mut transform, home)) = lights.get_mut(entity) else {
                continue;
            };
            let Some(towards_sun) = towards_sun.filter(|_| Some(entity) == brightest) else {
                if let Some(home) = home {
                    transform.rotation = home.0;
                    commands.entity(entity).remove::<HomeRotation>();
                }
                continue;
            };
            if home.is_none() {
                commands.entity(entity).insert(HomeRotation(transform.rotation));
            }
            let above = tree
                .parents
                .get(entity)
                .ok()
                .and_then(|parent| places.get(parent.parent()).ok())
                .map_or(Quat::IDENTITY, |place| place.compute_transform().rotation);
            let facing = Transform::default().looking_to(-towards_sun, Vec3::Y).rotation;
            let rotation = above.inverse() * facing;
            if transform.rotation.angle_between(rotation) > 1e-5 {
                transform.rotation = rotation;
            }
        }
    }
}
