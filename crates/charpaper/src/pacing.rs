//! How often the app updates and renders: the frame-rate limit, and pausing
//! while nobody can see the wallpaper.
//!
//! Both go through winit's update mode rather than skipping work inside a
//! frame. In reactive mode winit sleeps until the wait runs out or a window
//! event arrives, so between frames the app uses no CPU and the GPU has
//! nothing to draw.

use std::time::Duration;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::winit::UpdateMode;
use bevy::winit::WinitSettings;
use charpaper_scene::RenderSettings;

use crate::plugin::BackendResource;

/// How often a paused app still wakes: often enough to resume within a second
/// of the desktop showing again, rarely enough to cost nothing.
const PAUSED_WAKE: Duration = Duration::from_secs(1);
/// How often the backend is asked what else is on screen.
const ACTIVITY_POLL: Duration = Duration::from_secs(1);

/// Why rendering is paused, if it is.
#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub struct Paused(pub Option<&'static str>);

pub struct PacingPlugin;

impl Plugin for PacingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Paused>().add_systems(
            Update,
            (
                poll_activity,
                apply_update_mode.run_if(
                    resource_changed::<RenderSettings>.or_eager(resource_changed::<Paused>),
                ),
            )
                .chain(),
        );
    }
}

/// On the main thread, like everything else that talks to the backend.
fn poll_activity(
    _main_thread: NonSendMarker,
    time: Res<Time<Real>>,
    mut since: Local<Duration>,
    mut backend: ResMut<BackendResource>,
    settings: Res<RenderSettings>,
    mut paused: ResMut<Paused>,
) {
    *since += time.delta();
    if *since < ACTIVITY_POLL && !settings.is_changed() {
        return;
    }
    *since = Duration::ZERO;

    let activity = backend.0.activity();
    let reason = if settings.pause_when_fullscreen && activity.fullscreen_app {
        Some("an app is fullscreen")
    } else if settings.pause_when_covered && activity.covered {
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

/// The wallpaper's window never has focus, so both modes get the same value.
fn apply_update_mode(
    settings: Res<RenderSettings>,
    paused: Res<Paused>,
    mut winit: ResMut<WinitSettings>,
) {
    let mode = match (paused.0, settings.fps_limit.per_second()) {
        (Some(_), _) => UpdateMode::reactive_low_power(PAUSED_WAKE),
        (None, Some(fps)) => UpdateMode::reactive_low_power(Duration::from_secs_f64(1.0 / fps)),
        (None, None) => UpdateMode::Continuous,
    };
    winit.focused_mode = mode;
    winit.unfocused_mode = mode;
}
