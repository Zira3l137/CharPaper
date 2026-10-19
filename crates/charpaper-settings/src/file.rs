use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

use crate::audio::Volumes;
use crate::render::RenderSettings;
use crate::screen::ScreenSettings;
use crate::stage::StageSeeds;
use crate::system::SystemSettings;
use crate::ui::UiState;

pub const STATE_FILE: &str = "state.toml";

// Everything state.toml holds. The app inserts each part as a resource at startup.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct SavedState {
    pub system: SystemSettings,
    pub ui: UiState,
    pub render: RenderSettings,
    pub screen: ScreenSettings,
    pub audio: Volumes,
    pub stages: StageSeeds,
}

// Problems are returned, not logged, because this runs before Bevy's logger exists.
#[derive(Clone)]
pub struct Loaded {
    pub state: SavedState,
    pub problem: Option<String>,
}

pub fn load(path: &Path) -> Loaded {
    let fresh = |problem: String| Loaded { state: SavedState::default(), problem: Some(problem) };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Loaded { state: SavedState::default(), problem: None };
        }
        Err(err) => return fresh(format!("cannot read {}: {err}; starting fresh", path.display())),
    };
    match toml::from_str::<SavedState>(&text) {
        Ok(state) => Loaded { state, problem: None },
        Err(err) => {
            // Moved aside rather than overwritten, in case it was edited by hand.
            let aside = path.with_extension("toml.bad");
            let moved = fs::rename(path, &aside).is_ok();
            let fate = if moved {
                format!("moved it to {}", aside.display())
            } else {
                "ignored it".into()
            };
            fresh(format!("{} is not valid ({err}); {fate} and started fresh", path.display()))
        }
    }
}

// Written to a temporary file and renamed, so a crash mid-write can't truncate it.
pub fn write(path: &Path, state: &SavedState) -> io::Result<()> {
    let text = toml::to_string_pretty(state).map_err(io::Error::other)?;
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}
