use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::Resource;

use crate::render::RenderSettings;
use crate::screens::StageId;
use crate::stage::StageSeed;

#[derive(Resource, Clone, Debug, Default)]
pub struct SceneConfig {
    pub characters_dir: PathBuf,
    // Stage A's suite, overriding the one saved.
    pub suite: Option<String>,
    pub stages: BTreeMap<StageId, StageSeed>,
    pub render: RenderSettings,
}
