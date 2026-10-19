use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use charpaper_settings::StartAtLogin;

use crate::wallpaper::platform;

// Keeps the OS's "start at sign-in" in step with the panel's switch: read once at startup, and
// written whenever the switch changes.
pub struct LoginPlugin {
    pub exe: PathBuf,
}

#[derive(Resource)]
struct Executable(PathBuf);

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StartAtLogin(read(&self.exe)))
            .insert_resource(Executable(self.exe.clone()))
            .add_systems(Update, apply.run_if(resource_changed::<StartAtLogin>));
    }
}

// An unreadable setting shows as off, which switching on writes over.
fn read(exe: &Path) -> Option<bool> {
    match platform::starts_at_login(exe)? {
        Ok(on) => Some(on),
        Err(err) => {
            warn!("cannot tell whether CharPaper starts at sign-in: {err}");
            Some(false)
        }
    }
}

// What failed to be written is put back as the OS has it.
fn apply(mut start: ResMut<StartAtLogin>, exe: Res<Executable>, mut applied: Local<Option<bool>>) {
    let Some(on) = start.0 else {
        return;
    };
    // The first time is the value read at startup.
    let applied = applied.get_or_insert(on);
    if on == *applied {
        return;
    }
    match platform::set_starts_at_login(&exe.0, on) {
        Ok(()) => {
            info!("CharPaper {} at sign-in", if on { "now starts" } else { "no longer starts" });
            *applied = on;
        }
        Err(err) => {
            warn!("cannot change whether CharPaper starts at sign-in: {err}");
            let actual = read(&exe.0).unwrap_or(*applied);
            *applied = actual;
            start.0 = Some(actual);
        }
    }
}
