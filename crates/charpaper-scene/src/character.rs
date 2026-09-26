//! Spawns the active suite's character: the armature, and the one skin worn.
//!
//! Only the worn skin is in memory. Switching loads the next one from disk
//! and frees the last, which costs a moment of loading per switch and saves
//! holding every outfit's meshes and textures at once.

use bevy::prelude::*;
use charpaper_suite::ORBIT_CAMERA;
use serde::Deserialize;
use serde::Serialize;

use crate::SceneConfig;
use crate::binding::SkinPart;
use crate::suite::ActiveSuite;
use crate::suite::load_scene;

/// What the viewer has picked. Systems react to changes, so writing here is
/// how the UI will switch things.
#[derive(Resource, Default, Debug)]
pub struct CharacterState {
    /// `None` shows no skin at all: just the bare armature, which is invisible.
    pub skin: Option<String>,
    /// A name from [`crate::CharacterClips`]. `None` stops the animation and
    /// leaves the character in whatever pose it was in.
    pub animation: Option<String>,
    /// A camera from `cameras/`, by file name. `None` is the orbit camera,
    /// which the mouse only moves while it is the one in use.
    pub camera: Option<String>,
    /// An environment by name. `None` shows none, which leaves the character
    /// unlit.
    pub environment: Option<String>,
}

/// The viewer's choices for one suite, as kept between runs.
///
/// Each is a preference only. One the suite no longer offers, such as a
/// renamed skin or a deleted camera, is ignored in favour of the suite's
/// default rather than leaving the character bare.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Picks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    /// [`ORBIT_CAMERA`] for the orbit camera. Unlike in [`CharacterState`],
    /// `None` here means "no preference", not "orbit".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
}

impl Picks {
    pub fn from_state(state: &CharacterState) -> Self {
        Self {
            skin: state.skin.clone(),
            animation: state.animation.clone(),
            camera: Some(state.camera.clone().unwrap_or_else(|| ORBIT_CAMERA.to_string())),
            environment: state.environment.clone(),
        }
    }
}

/// A remembered choice if `offered` still accepts it, otherwise `default`.
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

/// The skin on screen, as opposed to [`CharacterState::skin`], the one asked
/// for.
#[derive(Resource, Default)]
pub(crate) struct ShownSkin {
    pub name: Option<String>,
    pub root: Option<Entity>,
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
    config: Res<SceneConfig>,
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

    state.skin = prefer(
        suite.remembered(&config).skin,
        |skin| suite.skins.iter().any(|s| s.name == skin),
        suite.default_skin.clone(),
    );
}

/// Replaces the skin on screen with the one asked for. The old one is
/// despawned, which drops the last handles to its meshes and textures, so
/// they leave memory; the new one loads from disk. Pieces the old skin moved
/// onto the armature are not below its root any more and go separately.
pub(crate) fn switch_skin(
    mut commands: Commands,
    state: Res<CharacterState>,
    suite: Option<Res<ActiveSuite>>,
    assets: Res<AssetServer>,
    mut shown: ResMut<ShownSkin>,
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

    let Some(skin) =
        state.skin.as_ref().and_then(|name| suite.skins.iter().find(|s| &s.name == name))
    else {
        return;
    };
    let root = commands
        .spawn((
            Name::new(format!("Skin {}", skin.name)),
            SkinRoot { name: skin.name.clone() },
            ChildOf(character),
            WorldAssetRoot(load_scene(&assets, &suite, &skin.file)),
        ))
        .id();
    shown.root = Some(root);
}
