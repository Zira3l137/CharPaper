use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;
use charpaper_scene::Picks;
use charpaper_scene::RenderSettings;
use charpaper_ui::UiState;
use serde::Deserialize;
use serde::Serialize;

pub const STATE_FILE: &str = "state.toml";

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct SavedState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<String>,
    pub ui: UiState,
    pub render: RenderSettings,
    pub suites: BTreeMap<String, Picks>,
}

// Problems are returned, not logged, because this runs before Bevy's logger exists.
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
    match toml::from_str(&text) {
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

// Saved on every change rather than on exit: a logoff, shutdown or killed process never
// runs Bevy's exit path.
pub struct StatePlugin {
    pub path: PathBuf,
    pub saved: SavedState,
    pub problem: Option<String>,
}

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StateFile {
            path: self.path.clone(),
            saved: self.saved.clone(),
            problem: self.problem.clone(),
            write_failed: false,
        })
        .add_systems(Startup, report_problem)
        .add_systems(
            Last,
            save.run_if(
                resource_changed::<CharacterState>
                    .or_eager(resource_changed::<UiState>)
                    .or_eager(resource_changed::<RenderSettings>),
            ),
        );
    }
}

#[derive(Resource)]
struct StateFile {
    path: PathBuf,
    saved: SavedState,
    problem: Option<String>,
    write_failed: bool,
}

fn report_problem(mut file: ResMut<StateFile>) {
    if let Some(problem) = file.problem.take() {
        warn!("{problem}");
    }
}

fn save(
    mut file: ResMut<StateFile>,
    ui: Res<UiState>,
    render: Res<RenderSettings>,
    character: Res<CharacterState>,
    suite: Option<Res<ActiveSuite>>,
) {
    let mut next = file.saved.clone();
    next.ui = ui.clone();
    next.render = render.clone();
    if character.suite.is_some() {
        next.suite = character.suite.clone();
    }
    if let Some(suite) = suite {
        next.suites.entry(suite.folder()).or_default().merge(Picks::from_state(&character));
    }
    if next == file.saved {
        return;
    }

    match write(&file.path, &next) {
        Ok(()) => {
            debug!("saved choices to {}", file.path.display());
            file.write_failed = false;
        }
        Err(err) if !file.write_failed => {
            warn!("cannot save choices to {}: {err}", file.path.display());
            file.write_failed = true;
        }
        Err(_) => {}
    }
    file.saved = next;
}

// Written to a temporary file and renamed, so a crash mid-write can't truncate it.
fn write(path: &Path, state: &SavedState) -> anyhow::Result<()> {
    let text = toml::to_string_pretty(state)?;
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)?;
    Ok(())
}
