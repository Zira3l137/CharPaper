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
mod soundscape;
mod span;
mod stage;
mod state;
mod suite;

use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;

pub use assets::ARMATURE_SOURCE;
pub use assets::CHARACTERS_SOURCE;
pub use character::Character;
pub use character::CharacterClips;
pub use character::Expressions;
pub use character::SkinObjects;
pub use config::SceneConfig;
pub use gaze::CursorAt;
pub use gaze::CursorPosition;
pub use gaze::GazeSettings;
pub use gaze::GazeTuning;
pub use look::ActiveLook;
pub use look::LookEdited;
pub use look::Resolved;
pub use look::RestoreLook;
pub use render::AntiAliasing;
pub use render::DepthOfFieldQuality;
pub use render::FogQuality;
pub use render::FpsLimit;
pub use render::RENDER_SCALES;
pub use render::RenderSettings;
pub use screens::ChosenMonitor;
pub use screens::MonitorEntry;
pub use screens::Monitors;
pub use screens::Screen;
pub use screens::ScreenPlace;
pub use screens::ScreenSettings;
pub use screens::ScreenUi;
pub use screens::ShowsStage;
pub use screens::StageId;
pub use screens::Unseen;
pub use stage::Focused;
pub use stage::FocusedMut;
pub use stage::FocusedStage;
pub use stage::Stage;
pub use stage::StageKey;
pub use stage::StageMemories;
pub use stage::StageSeed;
pub use stage::focused_added;
pub use stage::focused_changed;
pub use stage::focused_has;
pub use stage::stage_memory_changed;
pub use state::CharacterState;
pub use state::Picks;
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

pub struct ScenePlugin {
    pub config: SceneConfig,
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        let config = &self.config;
        let [r, g, b] = config.clear_color;

        app.insert_resource(config.clone())
            .insert_resource(config.render.clone())
            .insert_resource(stage::StageSeeds(config.stages.clone()))
            .insert_resource(ClearColor(Color::srgb(r, g, b)))
            // No built-in lighting: only what the environment ships lights the character.
            .insert_resource(GlobalAmbientLight::NONE)
            .configure_sets(Update, (SceneSet::Switch, SceneSet::Fill, SceneSet::Run).chain())
            .add_plugins((
                stage::StagePlugin,
                suite::SuitePlugin,
                character::CharacterPlugin,
                camera::CameraPlugin,
                environment::EnvironmentPlugin,
                fog::FogPlugin,
                look::LookPlugin,
                lut::LutPlugin,
                render::RenderPlugin,
                shadows::ShadowsPlugin,
                gaze::GazePlugin,
                soundscape::SoundscapePlugin,
                span::SpanPlugin,
            ));
    }
}
