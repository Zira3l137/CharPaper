use std::io;
use std::process::Command;
use std::process::Stdio;

use bevy::prelude::*;
use charpaper_wallpaper::WallpaperBackend;

use crate::wallpaper::platform;

// A process that is killed or crashes can't take its windows off the desktop, and on some
// desktops that leaves a frozen picture of them behind. So once a window is attached, a keeper
// is left running: this same executable with --keep-desktop, which waits for the app to end
// and tidies the desktop if it ended badly.

pub(super) fn start(backend: &dyn WallpaperBackend) {
    match spawn(backend, &[std::process::id().to_string()]) {
        Ok(()) => debug!("a keeper will tidy the desktop if the app is killed or crashes"),
        Err(err) => warn!("cannot start the desktop keeper, so a crash may leave a ghost: {err}"),
    }
}

// The keeper's whole run, before Bevy or logging start. Task Manager ends a process together
// with the ones it started, so the first keeper only starts a second and leaves. With no
// parent left, the second ends only on its own.
pub fn keep_desktop(app: u32, orphaned: bool) {
    let mut backend = platform::create_backend();
    if orphaned {
        backend.keep(app);
    } else {
        let _ = spawn(backend.as_ref(), &[app.to_string(), "--orphaned".into()]);
    }
}

fn spawn(backend: &dyn WallpaperBackend, args: &[String]) -> io::Result<()> {
    let mut keeper = Command::new(std::env::current_exe()?);
    keeper.arg("--keep-desktop").args(args);
    keeper.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    backend.spawn_keeper(&mut keeper)
}
