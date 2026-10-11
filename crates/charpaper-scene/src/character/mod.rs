mod animation;
mod binding;
mod correctives;
mod cues;
mod expression;
mod skin;

use bevy::prelude::*;

pub use animation::CharacterClips;
pub(crate) use animation::PendingClips;
pub(crate) use cues::CueClock;
pub use expression::Expressions;
pub(crate) use skin::ShownSkin;
pub use skin::SkinObjects;

use crate::SceneSet;
use crate::assets::load_armature;
use crate::state::CharacterState;
use crate::state::prefer;
use crate::suite::ActiveSuite;
use crate::suite::Remembered;

// The armature, the one skin worn on it, and everything that animates them.
pub(crate) struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(binding::mark_ready)
            .add_systems(Update, (spawn_character, animation::load_clips).in_set(SceneSet::Fill))
            .add_systems(
                Update,
                (
                    (
                        animation::make_armature_animatable,
                        animation::build_graph,
                        animation::finish_once,
                        animation::play_selected,
                        cues::play_cues,
                    )
                        .chain(),
                    (
                        correctives::bind_correctives,
                        binding::bind_skins,
                        skin::list_skin_objects,
                        skin::apply_hidden_objects,
                    )
                        .chain(),
                    (skin::switch_skin, expression::spawn_skin_scenes).chain(),
                    (expression::setup_expressions, expression::play_expression).chain(),
                )
                    .in_set(SceneSet::Run),
            );
    }
}

// Parent of everything spawned for a stage's character, so it goes in one despawn. A child
// of its stage.
#[derive(Component)]
pub struct Character;

#[derive(Component)]
pub(crate) struct Armature;

pub(crate) fn spawn_character(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut stages: Query<(Entity, &ActiveSuite, &Remembered, &mut CharacterState), Added<ActiveSuite>>,
) {
    for (stage, suite, remembered, mut state) in &mut stages {
        let character = commands
            .spawn((
                Name::new(format!("Character {}", suite.name)),
                Character,
                Transform::default(),
                Visibility::default(),
                ChildOf(stage),
            ))
            .id();

        commands.spawn((
            Name::new("Suite armature"),
            Armature,
            ChildOf(character),
            WorldAssetRoot(load_armature(&assets, suite)),
        ));

        let picks = suite.remembered(remembered);
        state.hidden = picks.hidden;
        state.expressions = picks.expressions;
        state.skin = prefer(
            picks.skin,
            |skin| suite.skins.iter().any(|s| s.name == skin),
            suite.default_skin.clone(),
        );
    }
}
