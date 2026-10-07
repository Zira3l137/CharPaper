use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::LookBackup;
use charpaper_scene::RestoreLook;
use charpaper_suite::Look;
use charpaper_suite::Suite;

use crate::save::Unsaved;

// Look edits live in the suite's own suite.toml; everything else the viewer picks goes to
// state.toml, so each setting has exactly one home.
pub struct LookFilePlugin;

impl Plugin for LookFilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnsavedLook>()
            .add_systems(Update, find_backup.run_if(resource_added::<ActiveSuite>))
            .add_systems(
                Last,
                (
                    restore,
                    note_change.run_if(resource_exists_and_changed::<ActiveLook>),
                    save_settled,
                )
                    .chain(),
            );
    }
}

// A copy with the folder it belongs to, not just a flag: switching suites replaces the look
// before the last suite's final edits would have been written.
#[derive(Resource, Default)]
struct UnsavedLook(Option<Unsaved<(PathBuf, Look)>>);

fn find_backup(suite: Res<ActiveSuite>, mut backup: ResMut<LookBackup>) {
    backup.exists = charpaper_suite::has_backup(&suite.root);
}

fn note_change(
    time: Res<Time<Real>>,
    suite: Option<Res<ActiveSuite>>,
    look: Res<ActiveLook>,
    mut unsaved: ResMut<UnsavedLook>,
) {
    let Some(suite) = suite else {
        return;
    };
    if let Some(left) = unsaved.0.take_if(|u| u.value.0 != suite.root) {
        let (root, look) = &left.value;
        write(root, look);
    }
    unsaved.0 = Some(Unsaved::new((suite.root.clone(), look.0.clone()), time.elapsed()));
}

fn save_settled(
    time: Res<Time<Real>>,
    mut exits: MessageReader<AppExit>,
    suite: Option<Res<ActiveSuite>>,
    mut unsaved: ResMut<UnsavedLook>,
    mut backup: ResMut<LookBackup>,
) {
    let quitting = exits.read().count() > 0;
    let now = time.elapsed();
    let Some(due) = unsaved.0.take_if(|u| u.is_due(now, quitting)) else {
        return;
    };
    let (root, look) = &due.value;
    if write(root, look) && suite.is_some_and(|suite| &suite.root == root) {
        backup.exists = true;
    }
}

// True when suite.toml changed, which also leaves the author's version as a backup.
fn write(root: &Path, look: &Look) -> bool {
    match charpaper_suite::save_look(root, look) {
        Ok(changed) => {
            if changed {
                debug!("saved the look to {}", root.display());
            }
            changed
        }
        Err(err) => {
            warn!("cannot save the look: {err}");
            false
        }
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
    // The author's file may set values the edited one didn't, so reload the whole look.
    match Suite::load(&suite.root) {
        Ok(fresh) => {
            **look = fresh.look();
            backup.exists = false;
            info!("restored the suite's own settings");
        }
        Err(err) => warn!("restored the suite's settings, but cannot read them back: {err}"),
    }
}
