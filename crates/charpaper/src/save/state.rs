use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use charpaper_audio::Volumes;
use charpaper_scene::GazeSettings;
use charpaper_scene::Graphics;
use charpaper_scene::Picks;
use charpaper_scene::RenderSettings;
use charpaper_scene::ScreenSettings;
use charpaper_scene::StageId;
use charpaper_scene::StageMemories;
use charpaper_scene::StageSeed;
use charpaper_scene::stage_memory_changed;
use charpaper_ui::UiState;
use serde::Deserialize;
use serde::Serialize;

use crate::save::Unsaved;

pub const STATE_FILE: &str = "state.toml";

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct SavedState {
    pub ui: UiState,
    pub render: RenderSettings,
    pub screen: ScreenSettings,
    pub audio: Volumes,
    pub stages: BTreeMap<StageId, StageSeed>,
    // Where the one scene kept its choices before there were stages. Read once, into stage A.
    #[serde(skip_serializing)]
    suite: Option<String>,
    #[serde(skip_serializing)]
    gaze: Option<GazeSettings>,
    #[serde(skip_serializing)]
    suites: BTreeMap<String, Picks>,
}

impl SavedState {
    fn adopt_single_scene(mut self) -> Self {
        let (suite, gaze, suites) =
            (self.suite.take(), self.gaze.take(), std::mem::take(&mut self.suites));
        if self.stages.is_empty() && (suite.is_some() || gaze.is_some() || !suites.is_empty()) {
            let seed = StageSeed { suite, gaze: gaze.unwrap_or_default(), suites, ..default() };
            self.stages.insert(StageId::FIRST, seed);
        }
        self
    }

    // Before each stage was drawn its own way, [render] held how the one scene was. Every stage
    // saved then starts from that.
    fn adopt_shared_graphics(mut self, text: &str) -> Self {
        let Some(graphics) = shared_graphics(text) else {
            return self;
        };
        if self.stages.is_empty() {
            self.stages.insert(StageId::FIRST, StageSeed::default());
        }
        for seed in self.stages.values_mut() {
            seed.graphics.get_or_insert_with(|| graphics.clone());
        }
        self
    }
}

const SHARED_GRAPHICS: [&str; 7] = [
    "render_scale",
    "anti_aliasing",
    "depth_of_field",
    "fog",
    "fog_quality",
    "fog_dithering",
    "fog_resolution",
];

fn shared_graphics(text: &str) -> Option<Graphics> {
    let file: toml::Table = toml::from_str(text).ok()?;
    let render = file.get("render")?.as_table()?;
    if !SHARED_GRAPHICS.iter().any(|key| render.contains_key(*key)) {
        return None;
    }
    toml::Value::Table(render.clone()).try_into().ok()
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
        Ok(state) => {
            let state = state.adopt_single_scene().adopt_shared_graphics(&text);
            Loaded { state, problem: None }
        }
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

// Saved once changes settle rather than on exit: a logoff, shutdown or killed process never
// runs Bevy's exit path.
pub struct StatePlugin {
    pub path: PathBuf,
    pub loaded: Loaded,
}

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StateFile {
            path: self.path.clone(),
            saved: self.loaded.state.clone(),
            unsaved: None,
            problem: self.loaded.problem.clone(),
            write_failed: false,
        })
        .add_systems(Startup, report_problem)
        .add_systems(
            Last,
            (
                note_change.run_if(
                    stage_memory_changed
                        .or_eager(resource_changed::<UiState>)
                        .or_eager(resource_changed::<RenderSettings>)
                        .or_eager(resource_changed::<ScreenSettings>)
                        .or_eager(resource_changed::<Volumes>),
                ),
                save_settled,
            )
                .chain(),
        );
    }
}

#[derive(Resource)]
struct StateFile {
    path: PathBuf,
    saved: SavedState,
    unsaved: Option<Unsaved<SavedState>>,
    problem: Option<String>,
    write_failed: bool,
}

fn report_problem(mut file: ResMut<StateFile>) {
    if let Some(problem) = file.problem.take() {
        warn!("{problem}");
    }
}

// The stages' memories hold every stage, those whose monitors are unplugged included, so
// none is dropped from the file while it is away.
fn note_change(
    time: Res<Time<Real>>,
    mut file: ResMut<StateFile>,
    ui: Res<UiState>,
    render: Res<RenderSettings>,
    screen: Res<ScreenSettings>,
    volumes: Res<Volumes>,
    stages: StageMemories,
) {
    let next = SavedState {
        ui: ui.clone(),
        render: render.clone(),
        screen: screen.clone(),
        audio: volumes.clone(),
        stages: stages.all(),
        ..default()
    };
    file.unsaved = (next != file.saved).then(|| Unsaved::new(next, time.elapsed()));
}

fn save_settled(
    time: Res<Time<Real>>,
    mut exits: MessageReader<AppExit>,
    mut file: ResMut<StateFile>,
) {
    let quitting = exits.read().count() > 0;
    let now = time.elapsed();
    let Some(due) = file.unsaved.take_if(|u| u.is_due(now, quitting)) else {
        return;
    };

    match write(&file.path, &due.value) {
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
    file.saved = due.value;
}

// Written to a temporary file and renamed, so a crash mid-write can't truncate it.
fn write(path: &Path, state: &SavedState) -> anyhow::Result<()> {
    let text = toml::to_string_pretty(state)?;
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)?;
    Ok(())
}
