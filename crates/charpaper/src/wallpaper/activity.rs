use std::time::Duration;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use charpaper_scene::Screen;
use charpaper_scene::Unseen;
use charpaper_settings::RenderSettings;

use crate::wallpaper::Backend;
use crate::wallpaper::key;

const ACTIVITY_POLL: Duration = Duration::from_secs(1);

// Why the app should rest now, if it should: nobody can see or wants to see the wallpaper.
// The app's pacing decides what resting means.
#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub struct Paused(pub Option<&'static str>);

// Where the desktop's activity is read, so what acts on it can run after in the same frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ReadActivity;

pub(crate) struct ActivityPlugin;

impl Plugin for ActivityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Paused>().add_systems(Update, poll_activity.in_set(ReadActivity));
    }
}

// A covered monitor's screen rests on its own; only when every one is covered does the whole
// app pause.
fn poll_activity(
    _main_thread: NonSendMarker,
    mut commands: Commands,
    screens: Query<(Entity, &Screen, Has<Unseen>)>,
    time: Res<Time<Real>>,
    mut since: Local<Duration>,
    mut backend: ResMut<Backend>,
    settings: Res<RenderSettings>,
    mut paused: ResMut<Paused>,
) {
    *since += time.delta();
    if *since < ACTIVITY_POLL && !settings.is_changed() {
        return;
    }
    *since = Duration::ZERO;

    let activity = backend.0.activity();
    let mut all_covered = !screens.is_empty();
    for (entity, screen, was_unseen) in &screens {
        let unseen = settings.pause_when_covered && activity.covered.contains(&key(screen.window));
        all_covered &= unseen;
        if unseen != was_unseen {
            match unseen {
                true => commands.entity(entity).insert(Unseen),
                false => commands.entity(entity).remove::<Unseen>(),
            };
        }
    }
    let reason = if activity.away {
        Some("nobody is at the screen")
    } else if settings.pause_when_fullscreen && activity.fullscreen_app {
        Some("an app is fullscreen")
    } else if all_covered {
        Some("the desktop is covered")
    } else if settings.pause_on_battery && activity.on_battery {
        Some("running on battery")
    } else {
        None
    };
    if paused.0 != reason {
        match reason {
            Some(why) => info!("pausing rendering: {why}"),
            None => info!("resuming rendering"),
        }
        paused.0 = reason;
    }
}
