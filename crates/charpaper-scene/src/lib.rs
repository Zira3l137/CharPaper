mod ambient_occlusion;
mod animated;
mod assets;
mod camera;
mod character;
mod config;
mod environment;
mod fog;
mod gaze;
mod look;
mod lut;
mod render;
mod screens;
mod shadows;
mod shape_keys;
mod soundscape;
mod span;
mod stage;
mod state;
mod suite;

pub use assets::ARMATURE_SOURCE;
pub use assets::CHARACTERS_SOURCE;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
pub use character::Character;
pub use character::CharacterClips;
pub use character::Expressions;
pub use character::SkinObjects;
use charpaper_settings::RenderSettings;
use charpaper_settings::StageSeeds;
pub use config::SceneConfig;
pub use gaze::CursorAt;
pub use gaze::CursorPosition;
pub use look::ActiveLook;
pub use look::LookEdited;
pub use look::Resolved;
pub use look::RestoreLook;
pub use screens::Screen;
pub use screens::ScreenPlace;
pub use screens::ScreenSync;
pub use screens::ScreenUi;
pub use screens::ShowsStage;
pub use screens::Unseen;
pub use soundscape::PlayTrack;
pub use soundscape::Soundscape;
pub use stage::Focused;
pub use stage::FocusedMut;
pub use stage::FocusedStage;
pub use stage::Stage;
pub use stage::StageKey;
pub use stage::StageMemories;
pub use stage::focused_added;
pub use stage::focused_changed;
pub use stage::focused_has;
pub use stage::stage_memory_changed;
pub use state::CharacterState;
pub use suite::ActiveSuite;
pub use suite::AvailableSuites;

// Every frame, in this order: swap a stage's suite if another one was picked, fill in a suite
// that was just loaded, then let every part catch up with the stage's CharacterState.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SceneSet {
    Switch,
    Fill,
    Run,
}

// What a screen shows wherever no stage draws, such as before its suite has loaded.
const CLEAR_COLOR: Color = Color::srgb(0.05, 0.06, 0.09);

pub struct ScenePlugin {
    pub config: SceneConfig,
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        // The app inserts what state.toml holds before adding this plugin.
        app.insert_resource(self.config.clone())
            .init_resource::<RenderSettings>()
            .init_resource::<StageSeeds>()
            .insert_resource(ClearColor(CLEAR_COLOR))
            // No built-in lighting: only what the environment ships lights the character.
            .insert_resource(GlobalAmbientLight::NONE)
            .configure_sets(Update, (SceneSet::Switch, SceneSet::Fill, SceneSet::Run).chain())
            .add_plugins((
                stage::StagePlugin,
                ambient_occlusion::AmbientOcclusionPlugin,
                suite::SuitePlugin,
                character::CharacterPlugin,
                camera::CameraPlugin,
                environment::EnvironmentPlugin,
                animated::AnimatedPlugin,
                fog::FogPlugin,
                look::LookPlugin,
                lut::LutPlugin,
                render::RenderPlugin,
                shadows::ShadowsPlugin,
                shape_keys::ShapeKeysPlugin,
                gaze::GazePlugin,
                soundscape::SoundscapePlugin,
                span::SpanPlugin,
            ));
    }
}
