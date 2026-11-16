// An environment's sun shines from its sky's sun, the sky's brightest spot, turned with the
// sky, so that its light and shadows match the sun in the background whatever Blender had it
// at and however the sky is turned in the app. Each sky's sun is found once, on the CPU, in
// the skybox Bevy keeps after loading it; see charpaper_bake::sun_direction.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::render::render_resource::TextureDataOrder;
use bevy::render::render_resource::TextureFormat;
use bevy::transform::TransformSystems;

use crate::environment::ShownEnvironment;
use crate::environment::shown_environment;
use crate::look::LightTree;
use crate::sky::SceneSky;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;

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
    suns: Res<SkySuns>,
    stages: Query<(Entity, Option<&ActiveSuite>, &ShownEnvironment)>,
    skies: Query<(Entity, &SceneSky)>,
    stage_of: StageOf,
    tree: LightTree,
    places: Query<&GlobalTransform>,
    mut lights: Query<(&DirectionalLight, &mut Transform)>,
) {
    for (stage, suite, shown) in &stages {
        let follows = shown_environment(suite, shown)
            .is_some_and(|environment| environment.settings.sun_follows_sky != Some(false));
        let Some(root) = shown.root.filter(|_| follows) else {
            continue;
        };
        let sky = skies.iter().find(|&(camera, _)| stage_of.get(camera) == Some(stage));
        let Some((_, sky)) = sky else {
            continue;
        };
        let Some(&Some(sun)) = suns.0.get(&sky.image.id()) else {
            continue;
        };
        let towards_sun = sky.rotation * sun;

        let brightest = tree
            .children
            .iter_descendants(root)
            .filter_map(|entity| {
                lights.get(entity).ok().map(|(light, _)| (entity, light.illuminance))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let Some((entity, _)) = brightest else {
            continue;
        };
        let above = tree
            .parents
            .get(entity)
            .ok()
            .and_then(|parent| places.get(parent.parent()).ok())
            .map_or(Quat::IDENTITY, |place| place.compute_transform().rotation);
        let facing = Transform::default().looking_to(-towards_sun, Vec3::Y).rotation;
        let rotation = above.inverse() * facing;
        if let Ok((_, mut transform)) = lights.get_mut(entity)
            && transform.rotation.angle_between(rotation) > 1e-5
        {
            transform.rotation = rotation;
        }
    }
}
