//! Spawns the active suite's character: the armature, and every skin with only
//! the chosen one visible.
//!
//! All skins are spawned up front and hidden, so switching is instant. The
//! cost is memory for skins nobody is looking at. Hidden meshes are skipped by
//! the renderer, so they do not cost frame time.

use bevy::prelude::*;

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
        WorldAssetRoot(load_scene(&assets, &suite, &suite.model, false)),
    ));

    for skin in &suite.skins {
        commands.spawn((
            Name::new(format!("Skin {}", skin.name)),
            SkinRoot { name: skin.name.clone() },
            ChildOf(character),
            Visibility::Hidden,
            WorldAssetRoot(load_scene(&assets, &suite, &skin.file, false)),
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
