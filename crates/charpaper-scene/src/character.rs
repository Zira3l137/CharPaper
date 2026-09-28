use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bevy::prelude::*;
use charpaper_suite::ORBIT_CAMERA;
use serde::Deserialize;
use serde::Serialize;

use crate::Expressions;
use crate::binding::Bound;
use crate::binding::SkinPart;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;
use crate::suite::load_armature;
use crate::suite::load_gltf;

// What the viewer asked for. Systems compare it with what is shown and catch up, so
// writing here is how the UI switches things.
#[derive(Resource, Default, Debug)]
pub struct CharacterState {
    pub suite: Option<String>,
    pub skin: Option<String>,
    pub animation: Option<String>,
    // None is the orbit camera.
    pub camera: Option<String>,
    pub environment: Option<String>,
    pub hidden: BTreeMap<String, BTreeSet<String>>,
    // Per skin. Neutral is an empty name, so a merge still overwrites an older pick.
    pub expressions: BTreeMap<String, String>,
}

impl CharacterState {
    pub fn expression(&self) -> Option<&str> {
        let skin = self.skin.as_ref()?;
        self.expressions.get(skin).map(String::as_str).filter(|e| !e.is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Picks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    // ORBIT_CAMERA for the orbit camera. Unlike in CharacterState, None means "no preference".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub hidden: BTreeMap<String, BTreeSet<String>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub expressions: BTreeMap<String, String>,
}

impl Picks {
    // An unset choice keeps the older value: the scene fills choices in over several
    // frames, and a gap must not erase what was remembered.
    pub fn merge(&mut self, newer: Picks) {
        let keep = |new: Option<String>, old: &mut Option<String>| *old = new.or(old.take());
        keep(newer.skin, &mut self.skin);
        keep(newer.animation, &mut self.animation);
        keep(newer.camera, &mut self.camera);
        keep(newer.environment, &mut self.environment);
        self.hidden.extend(newer.hidden);
        self.expressions.extend(newer.expressions);
    }

    pub fn from_state(state: &CharacterState) -> Self {
        Self {
            skin: state.skin.clone(),
            animation: state.animation.clone(),
            camera: Some(state.camera.clone().unwrap_or_else(|| ORBIT_CAMERA.to_string())),
            environment: state.environment.clone(),
            hidden: state.hidden.clone(),
            expressions: state.expressions.clone(),
        }
    }
}

pub(crate) fn prefer(
    remembered: Option<String>,
    offered: impl Fn(&str) -> bool,
    default: Option<String>,
) -> Option<String> {
    match remembered {
        Some(pick) if offered(&pick) => Some(pick),
        Some(pick) => {
            debug!("remembered choice {pick:?} is no longer offered; using the default");
            default
        }
        None => default,
    }
}

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
pub struct Character;

#[derive(Component)]
pub(crate) struct Armature;

#[derive(Component)]
pub(crate) struct SkinFile(pub Handle<Gltf>);

#[derive(Component)]
pub(crate) struct SkinRoot {
    pub name: String,
}

pub(crate) fn spawn_character(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
    remembered: Res<Remembered>,
    mut state: ResMut<CharacterState>,
) {
    let Some(suite) = suite else {
        return;
    };

    let character = commands
        .spawn((
            Name::new(format!("Character {}", suite.name)),
            Character,
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    commands.spawn((
        Name::new("Suite armature"),
        Armature,
        ChildOf(character),
        WorldAssetRoot(load_armature(&assets, &suite)),
    ));

    let picks = suite.remembered(&remembered);
    state.hidden = picks.hidden;
    state.expressions = picks.expressions;
    state.skin = prefer(
        picks.skin,
        |skin| suite.skins.iter().any(|s| s.name == skin),
        suite.default_skin.clone(),
    );
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
