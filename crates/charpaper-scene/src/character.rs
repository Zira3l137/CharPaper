//! Spawns the active suite's character: the armature, and every skin with only
//! the chosen one visible.
//!
//! All skins are spawned up front and hidden, so switching is instant. The
//! cost is memory for skins nobody is looking at. Hidden meshes are skipped by
//! the renderer, so they do not cost frame time.

use std::path::Path;

use bevy::asset::AssetPath;
use bevy::gltf::GltfLoaderSettings;
use bevy::prelude::*;
use charpaper_suite::Suite;

use crate::CHARACTERS_SOURCE;
use crate::binding::SkinPart;
use crate::suite::ActiveSuite;

/// What the viewer has picked. Systems react to changes, so writing here is
/// how the UI will switch things.
#[derive(Resource, Default, Debug)]
pub struct CharacterState {
    /// `None` shows no skin at all: just the bare armature, which is invisible.
    pub skin: Option<String>,
}

/// Parent of everything spawned from the suite, so replacing the suite is one
/// despawn.
#[derive(Component)]
pub struct Character;

#[derive(Component)]
pub(crate) struct Armature;

#[derive(Component)]
pub(crate) struct SkinRoot {
    pub name: String,
}

pub(crate) fn spawn_character(
    mut commands: Commands,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
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
        WorldAssetRoot(load_scene(&assets, &suite, &suite.model)),
    ));

    for skin in &suite.skins {
        commands.spawn((
            Name::new(format!("Skin {}", skin.name)),
            SkinRoot { name: skin.name.clone() },
            ChildOf(character),
            Visibility::Hidden,
            WorldAssetRoot(load_scene(&assets, &suite, &skin.file)),
        ));
    }

    state.skin = suite.default_skin.clone();
}

pub(crate) fn show_selected_skin(
    state: Res<CharacterState>,
    mut skins: Query<(Entity, &SkinRoot, &mut Visibility)>,
    mut parts: Query<(&SkinPart, &mut Visibility), Without<SkinRoot>>,
) {
    let mut selected = None;
    for (entity, skin, mut visibility) in &mut skins {
        let shown = state.skin.as_deref() == Some(skin.name.as_str());
        if shown {
            selected = Some(entity);
        }
        visibility.set_if_neq(visibility_for(shown));
    }
    for (part, mut visibility) in &mut parts {
        visibility.set_if_neq(visibility_for(Some(part.0) == selected));
    }
}

pub(crate) fn visibility_for(shown: bool) -> Visibility {
    match shown {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    }
}

/// Scene 0 rather than the file's default scene: Bevy has no label for the
/// default one, and Blender always exports the active scene as scene 0.
///
/// Cameras and lights in these files are ignored. Cameras have their own
/// folder, and lighting belongs to `suite.toml`; a stray lamp exported with a
/// skin would otherwise light the scene only while that skin is loaded.
fn load_scene(assets: &AssetServer, suite: &Suite, file: &Path) -> Handle<WorldAsset> {
    let folder = suite.root.file_name().unwrap_or_default();
    let path =
        AssetPath::from_path_buf(Path::new(folder).join(file)).with_source(CHARACTERS_SOURCE);
    assets
        .load_builder()
        .with_settings(|s: &mut GltfLoaderSettings| {
            s.load_cameras = false;
            s.load_lights = false;
        })
        .load(GltfAssetLabel::Scene(0).from_asset(path))
}
