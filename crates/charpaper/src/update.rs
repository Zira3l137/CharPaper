use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use bevy::prelude::*;
use charpaper_settings::SystemSettings;
use charpaper_settings::UpdateAction;
use charpaper_settings::UpdateStatus;
use charpaper_update::Release;
use charpaper_update::Staged;
use charpaper_update::Version;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;

use crate::restart::Restart;
use crate::wallpaper::platform;

// Asks GitHub for a newer stable version once per start, and downloads it when the panel
// asks, each on a thread of its own. What was downloaded is installed by app::run once Bevy
// has stopped, as a running executable can't be replaced, and the app then starts again the
// way a new display mode does.
pub struct UpdatePlugin {
    pub app_dir: PathBuf,
    // Releases newer than this are offered. Not the build's own only when testing.
    pub current: Version,
    // Off for debug builds and the benchmark. Tidying runs either way.
    pub check: bool,
    pub installer: Installer,
    pub restart: Restart,
}

// Shared with app::run, which outlives the app.
#[derive(Resource, Clone, Default)]
pub struct Installer(Arc<Mutex<Option<Staged>>>);

impl Installer {
    pub fn take(&self) -> Option<Staged> {
        self.0.lock().ok()?.take()
    }
}

enum Report {
    Found(Release),
    Progress { done: u64, total: u64 },
    Staged(Staged),
    Failed(String),
}

#[derive(Resource)]
struct Worker {
    app_dir: PathBuf,
    current: Version,
    check: bool,
    restart: Restart,
    sender: Sender<Report>,
    receiver: Receiver<Report>,
}

// The release on offer, kept for downloading it and for its page.
#[derive(Resource, Default)]
struct Offered(Option<Release>);

impl Plugin for UpdatePlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = crossbeam_channel::unbounded();
        app.add_message::<UpdateAction>()
            .init_resource::<UpdateStatus>()
            .init_resource::<Offered>()
            .insert_resource(self.installer.clone())
            .insert_resource(Worker {
                app_dir: self.app_dir.clone(),
                current: self.current.clone(),
                check: self.check,
                restart: self.restart.clone(),
                sender,
                receiver,
            })
            .add_systems(Startup, start)
            .add_systems(Update, (receive, act).chain());
    }
}

// Tidying comes first on the same thread, so a download, which can only follow a check,
// never has its folder removed from under it.
fn start(worker: Res<Worker>, settings: Res<SystemSettings>) {
    let check = worker.check && settings.check_for_updates && charpaper_update::supported();
    let (app_dir, current) = (worker.app_dir.clone(), worker.current.clone());
    let sender = worker.sender.clone();
    let spawned = thread::Builder::new().name("update check".into()).spawn(move || {
        if let Err(err) = charpaper_update::tidy(&app_dir) {
            warn!("cannot remove what the last update left: {err}");
        }
        if !check {
            return;
        }
        match charpaper_update::check(&current) {
            Ok(Some(release)) => {
                let _ = sender.send(Report::Found(release));
            }
            Ok(None) => info!("CharPaper {current} is the latest version"),
            Err(err) => warn!("cannot check for a newer version: {err}"),
        }
    });
    if let Err(err) = spawned {
        warn!("cannot start checking for a newer version: {err}");
    }
}

fn receive(
    worker: Res<Worker>,
    installer: Res<Installer>,
    settings: Res<SystemSettings>,
    mut offered: ResMut<Offered>,
    mut status: ResMut<UpdateStatus>,
    mut exit: MessageWriter<AppExit>,
) {
    for report in worker.receiver.try_iter() {
        match report {
            Report::Found(release) => {
                let version = release.version.to_string();
                if settings.skipped_version.as_deref() == Some(version.as_str()) {
                    info!("CharPaper {version} is out, but was skipped");
                } else {
                    info!("CharPaper {version} is out");
                    *status = UpdateStatus::Available { version };
                    offered.0 = Some(release);
                }
            }
            Report::Progress { done, total } => {
                if let UpdateStatus::Downloading { done: was, total: of, .. } = &mut *status {
                    (*was, *of) = (done, total);
                }
            }
            Report::Staged(staged) => {
                let version = staged.version.to_string();
                info!("CharPaper {version} is downloaded; restarting to install it");
                if let Ok(mut slot) = installer.0.lock() {
                    *slot = Some(staged);
                }
                *status = UpdateStatus::Installing { version };
                // As the Quit button does: off the desktop, state.toml written.
                worker.restart.ask();
                exit.write(AppExit::Success);
            }
            Report::Failed(reason) => {
                warn!("cannot download the update: {reason}");
                if let Some(release) = &offered.0 {
                    let version = release.version.to_string();
                    *status = UpdateStatus::Failed { version, reason };
                }
            }
        }
    }
}

fn act(
    mut actions: MessageReader<UpdateAction>,
    worker: Res<Worker>,
    mut offered: ResMut<Offered>,
    mut status: ResMut<UpdateStatus>,
    mut settings: ResMut<SystemSettings>,
) {
    for action in actions.read() {
        let Some(release) = &offered.0 else {
            continue;
        };
        match action {
            UpdateAction::Install => {
                let idle =
                    matches!(*status, UpdateStatus::Available { .. } | UpdateStatus::Failed { .. });
                if !idle {
                    continue;
                }
                let version = release.version.to_string();
                *status = match download(release.clone(), &worker) {
                    Ok(()) => UpdateStatus::Downloading { version, done: 0, total: 0 },
                    Err(reason) => UpdateStatus::Failed { version, reason },
                };
            }
            UpdateAction::Skip => {
                info!("skipping CharPaper {}", release.version);
                settings.skipped_version = Some(release.version.to_string());
                *status = UpdateStatus::Idle;
                offered.0 = None;
            }
            UpdateAction::OpenPage => platform::open_in_browser(&release.page),
        }
    }
}

fn download(release: Release, worker: &Worker) -> Result<(), String> {
    let (app_dir, sender) = (worker.app_dir.clone(), worker.sender.clone());
    let spawned = thread::Builder::new().name("update download".into()).spawn(move || {
        let mut shown = None;
        let progress = |done: u64, total: u64| {
            // Once per whole percent rather than per chunk, so the panel redraws at most a
            // hundred times.
            let percent = (done * 100).checked_div(total);
            if percent != shown {
                shown = percent;
                let _ = sender.send(Report::Progress { done, total });
            }
        };
        let report = match charpaper_update::download(&release, &app_dir, progress) {
            Ok(staged) => Report::Staged(staged),
            Err(err) => Report::Failed(err.to_string()),
        };
        let _ = sender.send(report);
    });
    spawned.map(drop).map_err(|err| format!("cannot start downloading: {err}"))
}
