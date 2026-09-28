use bevy::prelude::*;

use crate::assets::load_gltf;
use crate::character::Character;
use crate::character::binding::Bound;
use crate::character::binding::SkinPart;
use crate::character::expression::Expressions;
use crate::state::CharacterState;
use crate::suite::ActiveSuite;

#[derive(Resource, Default, Debug)]
pub struct SkinObjects(pub(crate) Vec<(String, Entity)>);

impl SkinObjects {
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(name, _)| name.as_str())
    }
}

#[derive(Component)]
pub(crate) struct ObjectsListed;

#[derive(Resource, Default)]
pub(crate) struct ShownSkin {
    pub name: Option<String>,
    pub root: Option<Entity>,
}

#[derive(Component)]
pub(crate) struct SkinFile(pub Handle<Gltf>);

#[derive(Component)]
pub(crate) struct SkinRoot {
    pub name: String,
}

// Despawning the old skin drops the last handles to its assets, which frees them. Pieces it
// moved onto the armature are no longer under its root, so they go separately.
pub(crate) fn switch_skin(
    mut commands: Commands,
    state: Res<CharacterState>,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownSkin>,
    mut objects: ResMut<SkinObjects>,
    mut expressions: ResMut<Expressions>,
    character: Query<Entity, With<Character>>,
    parts: Query<(Entity, &SkinPart)>,
) {
    if shown.name == state.skin {
        return;
    }
    let (Some(suite), Ok(character)) = (suite, character.single()) else {
        return;
    };

    if let Some(old) = shown.root.take() {
        for (part, _) in parts.iter().filter(|(_, part)| part.0 == old) {
            commands.entity(part).despawn();
        }
        commands.entity(old).despawn();
    }
    shown.name = state.skin.clone();
    objects.0.clear();
    expressions.0.clear();

    let Some(skin) =
        state.skin.as_ref().and_then(|name| suite.skins.iter().find(|s| &s.name == name))
    else {
        return;
    };
    let root = commands
        .spawn((
            Name::new(format!("Skin {}", skin.name)),
            SkinRoot { name: skin.name.clone() },
            SkinFile(load_gltf(&assets, &suite, &skin.file)),
            ChildOf(character),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    shown.root = Some(root);
}

// A mesh object is a node with mesh primitives below it: what Blender calls an object.
pub(crate) fn list_skin_objects(
    mut commands: Commands,
    skins: Query<(Entity, &SkinRoot), (With<Bound>, Without<ObjectsListed>)>,
    parts: Query<(Entity, &SkinPart)>,
    children: Query<&Children>,
    names: Query<&Name>,
    meshes: Query<(), With<Mesh3d>>,
    mut objects: ResMut<SkinObjects>,
) {
    for (root, skin) in &skins {
        let branches = std::iter::once(root)
            .chain(parts.iter().filter(|(_, part)| part.0 == root).map(|(entity, _)| entity));
        let mut found: Vec<(String, Entity)> = branches
            .flat_map(|branch| std::iter::once(branch).chain(children.iter_descendants(branch)))
            .filter(|&e| children.get(e).is_ok_and(|c| (**c).iter().any(|&c| meshes.contains(c))))
            .filter_map(|e| names.get(e).ok().map(|name| (name.to_string(), e)))
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found.dedup_by(|a, b| a.0 == b.0);
        let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
        info!("skin {:?}: {} mesh object(s) {names:?}", skin.name, found.len());
        objects.0 = found;
        commands.entity(root).insert(ObjectsListed);
    }
}

pub(crate) fn apply_hidden_objects(
    state: Res<CharacterState>,
    objects: Res<SkinObjects>,
    mut visibility: Query<&mut Visibility>,
) {
    let hidden = state.skin.as_ref().and_then(|skin| state.hidden.get(skin));
    for (name, entity) in &objects.0 {
        if let Ok(mut visibility) = visibility.get_mut(*entity) {
            visibility.set_if_neq(match hidden.is_some_and(|h| h.contains(name)) {
                true => Visibility::Hidden,
                false => Visibility::Inherited,
            });
        }
    }
}
