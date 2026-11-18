use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use bevy::prelude::*;
use bevy::winit::WinitSettings;
use charpaper_audio::Hush;
use charpaper_settings::RenderSettings;
use charpaper_ui::PanelHovered;

use crate::wallpaper::Paused;
use crate::wallpaper::ReadActivity;

const PAUSED_WAKE: Duration = Duration::from_secs(1);
// While the pointer is on the panel, whatever the frame limit and even while paused, so a
// click or a wheel turn shows at once.
const PANEL_FPS: f64 = 60.0;
// What is left of a wait is slept precisely. A condition variable's timeout is only as fine
// as Windows' timer tick, about 16 ms, where thread::sleep uses a high-resolution timer.
const PRECISE: Duration = Duration::from_millis(20);

// How often the app draws, and whether it plays sound, given the frame limit, the panel and
// whether the desktop says to rest.
//
// winit's reactive mode can't be trusted with the limit: on Windows Bevy asks for a redraw
// after each frame it runs on time, and the redraw comes back as a window event, which runs
// the next frame at once. So winit runs continuously, and each frame is held at its end until
// the next one is due.
pub struct PacingPlugin;

impl Plugin for PacingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinitSettings::game())
            .init_resource::<FrameAlarm>()
            .insert_resource(Pace { every: None, started: Instant::now() })
            .add_systems(
                Update,
                (
                    set_pace.run_if(
                        resource_changed::<RenderSettings>
                            .or_eager(resource_changed::<Paused>)
                            .or_eager(resource_changed::<PanelHovered>),
                    ),
                    hush_while_paused.run_if(resource_changed::<Paused>),
                )
                    .after(ReadActivity),
            )
            .add_systems(Last, hold_frame);
    }
}

// Rung by input forwarded from the desktop, which never reaches the window, so a click or a
// wheel turn ends the wait for the next frame instead of waiting it out, a second long while
// paused.
#[derive(Resource, Clone, Default)]
pub(crate) struct FrameAlarm(Arc<(Mutex<bool>, Condvar)>);

impl FrameAlarm {
    pub(crate) fn ring(&self) {
        let (rung, bell) = &*self.0;
        *rung.lock().unwrap_or_else(PoisonError::into_inner) = true;
        bell.notify_one();
    }

    // Until `until`, or a ring, whichever comes first. True for a ring.
    fn wait(&self, until: Instant) -> bool {
        let (rung, bell) = &*self.0;
        let mut guard = rung.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if *guard {
                *guard = false;
                return true;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left <= PRECISE {
                break;
            }
            guard =
                bell.wait_timeout(guard, left - PRECISE).unwrap_or_else(PoisonError::into_inner).0;
        }
        drop(guard);
        // A ring meanwhile ends the next wait instead.
        thread::sleep(until.saturating_duration_since(Instant::now()));
        false
    }
}

#[derive(Resource)]
struct Pace {
    // None draws as fast as vsync lets it.
    every: Option<Duration>,
    // When this frame was due to start. The next is due a period later, so the rate holds
    // without drifting by however late each wait ends.
    started: Instant,
}

// Whatever stops the drawing stops the sound too.
fn hush_while_paused(paused: Res<Paused>, mut hush: ResMut<Hush>) {
    hush.set_if_neq(Hush(paused.0.is_some()));
}

fn set_pace(
    settings: Res<RenderSettings>,
    paused: Res<Paused>,
    panel: Res<PanelHovered>,
    mut pace: ResMut<Pace>,
) {
    let every = |fps: f64| Some(Duration::from_secs_f64(1.0 / fps));
    pace.every = match (paused.0, settings.fps_limit.per_second()) {
        (_, Some(fps)) if panel.0 => every(fps.max(PANEL_FPS)),
        (_, None) if panel.0 => None,
        (Some(_), _) => Some(PAUSED_WAKE),
        (None, Some(fps)) => every(fps),
        (None, None) => None,
    };
}

// The last thing a frame does. Never on the way out, so quitting doesn't wait a period.
fn hold_frame(mut pace: ResMut<Pace>, alarm: Res<FrameAlarm>, mut exits: MessageReader<AppExit>) {
    let Some(every) = pace.every else {
        return;
    };
    if exits.read().count() > 0 {
        return;
    }
    let due = pace.started + every;
    // Running late starts the next frame now rather than catching up with a burst.
    let on_time = due > Instant::now() && !alarm.wait(due);
    pace.started = if on_time { due } else { Instant::now() };
}
