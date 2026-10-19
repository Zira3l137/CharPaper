use std::ffi::OsString;
use std::io;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use bevy::prelude::*;
use charpaper_settings::DisplayMode;
use charpaper_settings::SystemSettings;

// A changed mode starts the app again rather than switching on the spot: a window is made for
// one mode, with its styles, its parent and the windows beside it on other monitors all set up
// for it. This run quits as the Quit button does, which takes the windows off the desktop and
// writes state.toml, and main starts the next once Bevy has finished.
pub struct RestartPlugin {
    pub restart: Restart,
}

// Shared with main, which outlives the app.
#[derive(Resource, Clone, Default)]
pub struct Restart(Arc<AtomicBool>);

impl Restart {
    pub fn asked(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Plugin for RestartPlugin {
    fn build(&self, app: &mut App) {
        // Before Last, where quitting takes the windows off the desktop and saves.
        app.insert_resource(self.restart.clone()).add_systems(PostUpdate, restart_on_new_mode);
    }
}

fn restart_on_new_mode(
    settings: Res<SystemSettings>,
    restart: Res<Restart>,
    mut started_as: Local<Option<DisplayMode>>,
    mut exit: MessageWriter<AppExit>,
) {
    let started_as = *started_as.get_or_insert(settings.mode);
    if settings.mode == started_as || restart.asked() {
        return;
    }
    info!("restarting to show the scene as {:?}", settings.mode);
    restart.0.store(true, Ordering::Relaxed);
    exit.write(AppExit::Success);
}

pub fn start_again() -> io::Result<()> {
    Command::new(std::env::current_exe()?).args(kept_args()).spawn().map(drop)
}

// This run's arguments, less those state.toml now answers instead: the mode, and the suite and
// language, which the panel may have changed since.
fn kept_args() -> Vec<OsString> {
    let mut args = std::env::args_os().skip(1);
    let mut kept = Vec::new();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        // `--suite=alice` holds its value; `--suite alice` has it in the next argument.
        let (name, joined) = text.split_once('=').map_or((&*text, false), |(n, _)| (n, true));
        match name {
            "--windowed" | "--dry-run" => {}
            "--suite" | "--language" => {
                if !joined {
                    args.next();
                }
            }
            _ => kept.push(arg.clone()),
        }
    }
    kept
}
