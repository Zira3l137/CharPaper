//! Keeps `suite.toml` in step with the look the viewer edits in the settings
//! panel, and puts the author's version back on request.
//!
//! Only settings the manifest has a key for live here. Everything else the
//! viewer picks is app state and goes to `state.toml` instead, so each setting
//! has exactly one home.

use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::LookBackup;
use charpaper_scene::RestoreLook;
use charpaper_suite::Suite;

pub struct LookFilePlugin;

impl Plugin for LookFilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, find_backup.run_if(resource_added::<ActiveSuite>)).add_systems(
            Last,
            (restore, save.run_if(resource_exists_and_changed::<ActiveLook>)).chain(),
        );
    }
}

fn find_backup(suite: Res<ActiveSuite>, mut backup: ResMut<LookBackup>) {
    backup.exists = charpaper_suite::has_backup(&suite.root);
}

/// Also runs when the look first appears and after a restore. Both match the
/// file already, so `save_look` finds nothing to write and makes no backup.
fn save(suite: Option<Res<ActiveSuite>>, look: Res<ActiveLook>, mut backup: ResMut<LookBackup>) {
    let Some(suite) = suite else {
        return;
    };
    match charpaper_suite::save_look(&suite.root, &look) {
        Ok(true) => {
            debug!("saved the look to {}", suite.root.display());
            backup.exists = true;
        }
        Ok(false) => {}
        Err(err) => warn!("cannot save the look: {err}"),
    }
}

fn restore(
    mut requests: MessageReader<RestoreLook>,
    suite: Option<Res<ActiveSuite>>,
    look: Option<ResMut<ActiveLook>>,
    mut backup: ResMut<LookBackup>,
) {
    if requests.read().count() == 0 {
        return;
    }
    let (Some(suite), Some(mut look)) = (suite, look) else {
        return;
    };
    if let Err(err) = charpaper_suite::restore_look(&suite.root) {
        warn!("cannot restore the suite's settings: {err}");
        return;
    }
    // The author's file may set values the edited one did not, so the whole
    // look is read again rather than patched.
    match Suite::load(&suite.root) {
        Ok(fresh) => {
            **look = fresh.look();
            backup.exists = false;
            info!("restored the suite's own settings");
        }
        Err(err) => warn!("restored the suite's settings, but cannot read them back: {err}"),
    }
}
