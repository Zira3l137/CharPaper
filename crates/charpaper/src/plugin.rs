//! Wires a [`WallpaperBackend`] into the Bevy app.
//!
//! This is the only place where the engine half and the operating-system half
//! of the project meet, which is why it lives in the binary rather than in
//! either domain crate.

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::RawHandleWrapper;
use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::RawWindowHandle;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;

use crate::backend::create_backend;

/// `WallpaperConfig` lives in a Bevy-free crate, so it cannot derive
/// `Resource` itself. `Deref` means systems still read it as if it did.
#[derive(Resource, Deref)]
struct WallpaperSettings(WallpaperConfig);

/// Holds the backend so Bevy systems can reach it.
#[derive(Resource)]
struct BackendResource(Box<dyn WallpaperBackend>);

/// Tracks the retry loop. Bevy systems run every frame; this is how we
/// remember that we already succeeded and should stop trying.
#[derive(Resource, Default)]
struct AttachState {
    finished: bool,
    attempts: u32,
    countdown: u32,
}

pub struct WallpaperPlugin {
    pub config: WallpaperConfig,
}

impl Plugin for WallpaperPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WallpaperSettings(self.config.clone()))
            .insert_resource(BackendResource(create_backend()))
            .init_resource::<AttachState>()
            .add_systems(Startup, probe_desktop)
            .add_systems(Update, attach_window);
    }
}

/// Runs once at startup: asks the backend what it sees and logs it.
///
/// `NonSendMarker` is a zero-sized system parameter whose only job is to force
/// this system onto the main thread. Bevy normally spreads systems across a
/// thread pool; native window calls must not be spread around.
fn probe_desktop(
    _main_thread: NonSendMarker,
    mut backend: ResMut<BackendResource>,
    config: Res<WallpaperSettings>,
) {
    info!("wallpaper backend: {}", backend.0.name());

    match backend.0.probe(&config) {
        Ok(probe) => {
            if let Some(rec) = probe.recommended {
                info!("auto strategy resolves to: {rec:?}");
            }
        }
        Err(err) => warn!("desktop probe failed: {err}"),
    }
}

/// Runs every frame until it succeeds (or runs out of attempts).
///
/// Why not `Startup`? Because on frame zero the OS window may not exist yet:
/// Bevy adds the `RawHandleWrapper` component the moment winit creates the real
/// window, and `Commands` are applied at the end of a schedule, so the component
/// shows up a frame or two in. Polling in `Update` and latching a `finished`
/// flag is the simplest thing that is always correct.
///
/// The retry loop also covers a second case: right after login, Explorer can
/// still be starting up, so `Progman` exists but the background `WorkerW` does
/// not yet. Retrying for a few seconds costs nothing and fixes it.
fn attach_window(
    _main_thread: NonSendMarker,
    mut backend: ResMut<BackendResource>,
    config: Res<WallpaperSettings>,
    mut state: ResMut<AttachState>,
    handles: Query<&RawHandleWrapper, With<PrimaryWindow>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if state.finished {
        return;
    }

    if state.countdown > 0 {
        state.countdown -= 1;
        return;
    }

    // Pull the handle out here rather than handing the `Query` to a helper --
    // `RawWindowHandle` is `Copy`, so this keeps the helper free of ECS types.
    let handle = handles.single().ok().map(RawHandleWrapper::get_window_handle);
    let step = decide(&mut backend.0, &config, handle);

    // `reveal` means "we are done trying; make the window visible". Note it is
    // set on failure too. A hidden window that never appears would leave an
    // invisible process running with no way to close it, which is a much worse
    // failure than an ugly floating window.
    let mut reveal = false;

    match step {
        Step::Wait(reason) => {
            state.attempts += 1;
            if state.attempts >= config.max_attempts {
                warn!("giving up after {} attempts: {reason}", state.attempts);
                warn!("continuing as an ordinary window");
                state.finished = true;
                reveal = true;
            } else {
                debug!("attempt {} not ready: {reason}", state.attempts);
                state.countdown = config.frames_between_attempts;
            }
        }
        Step::Done(message) => {
            info!("{message}");
            state.finished = true;
            reveal = true;
        }
    }

    if reveal && config.show_window_after_attach {
        if let Ok(mut window) = windows.single_mut() {
            if !window.visible {
                window.visible = true;
            }
        }
    }
}

/// The outcome of one attempt. Split out so the system above never has to hold
/// a mutable `Query` and a backend borrow at the same time.
enum Step {
    /// Not ready (or failed); try again next time.
    Wait(String),
    /// Stop trying, with a line to log.
    Done(String),
}

fn decide(
    backend: &mut Box<dyn WallpaperBackend>,
    config: &WallpaperConfig,
    handle: Option<RawWindowHandle>,
) -> Step {
    if !config.enabled || config.strategy == AttachStrategy::None {
        return Step::Done("wallpaper attach disabled".to_string());
    }

    // The window may simply not exist yet on the first frames.
    let Some(raw) = handle else {
        return Step::Wait("window handle not ready".to_string());
    };

    if config.dry_run {
        debug!("[dry-run] would attach native handle {raw:?}");
        return Step::Done("dry run: no windows were modified".to_string());
    }

    match backend.attach(raw, config) {
        Ok(outcome) => {
            let how =
                outcome.strategy_used.map_or_else(|| "unknown".to_string(), |s| format!("{s:?}"));
            Step::Done(format!("attached to desktop using {how}"))
        }
        // `{:#}` on an `anyhow::Error` walks the source chain, so a failed
        // Win32 call logs "SetParent failed: The parameter is incorrect."
        // rather than losing everything below the top message.
        Err(err) => Step::Wait(format!("{:#}", anyhow::Error::new(err))),
    }
}
