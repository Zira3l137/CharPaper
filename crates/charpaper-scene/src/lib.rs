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
mod shadows;
mod soundscape;
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
pub use gaze::CursorPosition;
pub use gaze::GazeSettings;
pub use gaze::GazeTuning;
pub use look::ActiveLook;
pub use look::LookBackup;
pub use look::Resolved;
pub use look::RestoreLook;
pub use render::AntiAliasing;
pub use render::DepthOfFieldQuality;
pub use render::FogQuality;
pub use render::FpsLimit;
pub use render::RENDER_SCALES;
pub use render::RenderSettings;
pub use state::CharacterState;
pub use state::Picks;
pub use suite::ActiveSuite;
pub use suite::AvailableSuites;

// Every frame, in this order: swap the suite if another one was picked, fill in a suite
// that was just loaded, then let every part catch up with CharacterState.
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
            .insert_resource(config.gaze.clone())
            .insert_resource(suite::Remembered(config.remembered.clone()))
            .insert_resource(ClearColor(Color::srgb(r, g, b)))
            // No built-in lighting: only what the environment ships lights the character.
            .insert_resource(GlobalAmbientLight::NONE)
            .init_resource::<CharacterState>()
            .configure_sets(Update, (SceneSet::Switch, SceneSet::Fill, SceneSet::Run).chain())
            .configure_sets(Update, SceneSet::Fill.run_if(resource_added::<ActiveSuite>))
            .add_plugins((
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
            ));
    }
}
