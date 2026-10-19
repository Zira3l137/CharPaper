use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use charpaper_scene::StageMemories;
use charpaper_scene::stage_memory_changed;
use charpaper_settings::Loaded;
use charpaper_settings::RenderSettings;
use charpaper_settings::SavedState;
use charpaper_settings::ScreenSettings;
use charpaper_settings::SystemSettings;
use charpaper_settings::UiState;
use charpaper_settings::Volumes;

// How long changes must stop before they are written, so stepping a setting with the wheel
// writes the file once rather than on every step.
const SETTLE: Duration = Duration::from_secs(1);

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
                        .or_eager(resource_changed::<SystemSettings>)
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
    // A change not on disk yet, and when it was made.
    unsaved: Option<(SavedState, Duration)>,
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
    system: Res<SystemSettings>,
    ui: Res<UiState>,
    render: Res<RenderSettings>,
    screen: Res<ScreenSettings>,
    volumes: Res<Volumes>,
    stages: StageMemories,
) {
    let next = SavedState {
        system: system.clone(),
        ui: ui.clone(),
        render: render.clone(),
        screen: screen.clone(),
        audio: volumes.clone(),
        stages: stages.all(),
    };
    file.unsaved = (next != file.saved).then(|| (next, time.elapsed()));
}

// Quitting writes at once: the app is gone before the settle time is up.
fn save_settled(
    time: Res<Time<Real>>,
    mut exits: MessageReader<AppExit>,
    mut file: ResMut<StateFile>,
) {
    let quitting = exits.read().count() > 0;
    let now = time.elapsed();
    let due = |(_, changed_at): &mut (SavedState, Duration)| {
        quitting || now.saturating_sub(*changed_at) >= SETTLE
    };
    let Some((state, _)) = file.unsaved.take_if(due) else {
        return;
    };

    match charpaper_settings::write(&file.path, &state) {
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
    file.saved = state;
}
