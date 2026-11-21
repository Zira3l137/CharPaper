use bevy::prelude::*;

use crate::character::skin::SkinFile;
use crate::character::skin::SkinRoot;

// The expressions the worn skin can play, for the panel's picker.
#[derive(Component, Default, Debug, PartialEq)]
pub struct Expressions(pub Vec<String>);

pub(crate) fn spawn_skin_scenes(
    mut commands: Commands,
    skins: Query<(Entity, &SkinRoot, &SkinFile), Without<WorldAssetRoot>>,
    gltfs: Res<Assets<Gltf>>,
    assets: Res<AssetServer>,
) {
    for (entity, skin, file) in &skins {
        if assets.load_state(file.0.id()).is_failed() {
            warn!("skin {:?}: its file failed to load", skin.name);
            commands.entity(entity).remove::<SkinFile>();
            continue;
        }
        let Some(gltf) = gltfs.get(&file.0) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned())
        else {
            warn!("skin {:?}: its file holds no scene", skin.name);
            commands.entity(entity).remove::<SkinFile>();
            continue;
        };
        commands.entity(entity).insert(WorldAssetRoot(scene));
    }
}
