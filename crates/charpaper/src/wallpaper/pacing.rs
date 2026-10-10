use std::time::Duration;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::winit::UpdateMode;
use bevy::winit::WinitSettings;
use charpaper_audio::Hush;
use charpaper_scene::RenderSettings;
use charpaper_ui::PanelHovered;

use crate::wallpaper::Backend;

const PAUSED_WAKE: Duration = Duration::from_secs(1);
const ACTIVITY_POLL: Duration = Duration::from_secs(1);
// While the pointer is on the panel, whatever the frame limit and even while paused, so a
// click or a wheel turn shows at once.
const PANEL_FPS: f64 = 60.0;

#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub struct Paused(pub Option<&'static str>);

pub(crate) struct PacingPlugin;

impl Plugin for PacingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Paused>().add_systems(
            Update,
            (
                poll_activity,
                apply_update_mode.run_if(
                    resource_changed::<RenderSettings>
                        .or_eager(resource_changed::<Paused>)
                        .or_eager(resource_changed::<PanelHovered>),
                ),
                hush_while_paused.run_if(resource_changed::<Paused>),
            )
                .chain(),
        );
    }
}

fn poll_activity(
    _main_thread: NonSendMarker,
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
    let reason = if activity.away {
        Some("nobody is at the screen")
    } else if settings.pause_when_fullscreen && activity.fullscreen_app {
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

// Whatever stops the drawing stops the sound too.
fn hush_while_paused(paused: Res<Paused>, mut hush: ResMut<Hush>) {
    hush.set_if_neq(Hush(paused.0.is_some()));
}

// In reactive mode winit sleeps until the wait runs out or an event arrives, so between
// frames nothing runs and the GPU draws nothing.
fn apply_update_mode(
    settings: Res<RenderSettings>,
    paused: Res<Paused>,
    panel: Res<PanelHovered>,
    mut winit: ResMut<WinitSettings>,
) {
    let every = |fps: f64| UpdateMode::reactive_low_power(Duration::from_secs_f64(1.0 / fps));
    let mode = match (paused.0, settings.fps_limit.per_second()) {
        (_, Some(fps)) if panel.0 => every(fps.max(PANEL_FPS)),
        (_, None) if panel.0 => UpdateMode::Continuous,
        (Some(_), _) => UpdateMode::reactive_low_power(PAUSED_WAKE),
        (None, Some(fps)) => every(fps),
        (None, None) => UpdateMode::Continuous,
    };
    winit.focused_mode = mode;
    winit.unfocused_mode = mode;
}
