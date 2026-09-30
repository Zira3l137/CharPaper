use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::Resource;
use charpaper_bake::BakeSettings;

use crate::gaze::GazeSettings;
use crate::gaze::GazeTuning;
use crate::render::RenderSettings;
use crate::state::Picks;

#[derive(Resource, Clone, Debug)]
pub struct SceneConfig {
    pub clear_color: [f32; 3],
    pub characters_dir: PathBuf,
    pub suite: Option<String>,
    pub animation_crossfade_secs: f32,
    pub remembered: BTreeMap<String, Picks>,
    pub bake: BakeSettings,
    pub render: RenderSettings,
    pub expression_fade_secs: f32,
    pub gaze: GazeSettings,
    pub gaze_tuning: GazeTuning,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            clear_color: [0.05, 0.06, 0.09],
            characters_dir: PathBuf::from("characters"),
            suite: None,
            animation_crossfade_secs: 0.25,
            remembered: BTreeMap::new(),
            bake: BakeSettings::default(),
            render: RenderSettings::default(),
            expression_fade_secs: 0.3,
            gaze: GazeSettings::default(),
            gaze_tuning: GazeTuning::default(),
        }
    }
}
