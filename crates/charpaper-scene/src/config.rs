use std::path::PathBuf;

use bevy::prelude::Resource;

#[derive(Resource, Clone, Debug, Default)]
pub struct SceneConfig {
    pub characters_dir: PathBuf,
    // Stage A's suite, overriding the one saved.
    pub suite: Option<String>,
}
