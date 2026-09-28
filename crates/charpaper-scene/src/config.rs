//! Settings for the scene, owned by the crate that uses them.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::Resource;
use charpaper_bake::BakeSettings;

use crate::RenderSettings;

use crate::Picks;

/// Derives `Resource` so the systems can read it with `Res<SceneConfig>`.
///
/// In Bevy 0.19 `Resource` is built on top of `Component`, so it has to be
/// *derived* -- a hand-written `impl Resource` will not compile.
#[derive(Resource, Clone, Debug)]
pub struct SceneConfig {
    /// Background colour, linear sRGB 0.0-1.0.
    pub clear_color: [f32; 3],

    /// The folder holding one sub-folder per character suite. The app must
    /// register the same folder as the [`crate::CHARACTERS_SOURCE`] asset
    /// source, or nothing in it can be loaded.
    pub characters_dir: PathBuf,

    /// Folder name of the suite to show. `None` shows the first in name order.
    pub suite: Option<String>,

    /// How long switching animations blends the old into the new, in seconds.
    /// 0 switches instantly.
    pub animation_crossfade_secs: f32,

    /// The viewer's last choices, keyed by suite folder name. They win over
    /// the suite's own defaults wherever the suite still offers them.
    pub remembered: BTreeMap<String, Picks>,

    /// How environment maps are baked from a panorama when they are missing.
    pub bake: BakeSettings,

    /// Resolution, anti-aliasing, frame rate and pausing, as last saved.
    pub render: RenderSettings,

    /// How long switching expressions blends the old into the new, and how
    /// long going back to neutral takes, in seconds. 0 switches instantly.
    pub expression_fade_secs: f32,
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
        }
    }
}
