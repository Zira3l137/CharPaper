use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::RawHandleWrapper;
use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::RawWindowHandle;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;

use crate::wallpaper::Backend;
use crate::wallpaper::WallpaperSettings;
use crate::wallpaper::input::PointerSourceResource;

#[derive(Resource, Default)]
struct AttachState {
    finished: bool,
    attempts: u32,
    countdown: u32,
}

pub(crate) struct AttachPlugin;

impl Plugin for AttachPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AttachState>()
            .add_systems(Startup, probe_desktop)
            .add_systems(Update, attach_window);
    }
}

// NonSendMarker keeps a system on the main thread, which owns the window.
fn probe_desktop(
    _main_thread: NonSendMarker,
    mut backend: ResMut<Backend>,
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

// Retried every few frames: the window handle doesn't exist on the first frame, and
// Explorer may still be starting up right after login.
fn attach_window(
    _main_thread: NonSendMarker,
    mut commands: Commands,
    mut backend: ResMut<Backend>,
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

    let handle = handles.single().ok().map(RawHandleWrapper::get_window_handle);
    let step = decide(&mut backend.0, &config, handle);

    // Shown on failure too: an invisible process with no way to close it is worse.
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
        Step::Done { message, input } => {
            info!("{message}");
            if let Some(source) = input {
                commands.insert_resource(PointerSourceResource::new(source));
            }
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

enum Step {
    Wait(String),
    Done { message: String, input: Option<Box<dyn PointerSource>> },
}

fn decide(
    backend: &mut Box<dyn WallpaperBackend>,
    config: &WallpaperConfig,
    handle: Option<RawWindowHandle>,
) -> Step {
    if !config.enabled || config.strategy == AttachStrategy::None {
        return Step::Done { message: "wallpaper attach disabled".to_string(), input: None };
    }

    let Some(raw) = handle else {
        return Step::Wait("window handle not ready".to_string());
    };

    if config.dry_run {
        debug!("[dry-run] would attach native handle {raw:?}");
        if config.forward_input {
            debug!("[dry-run] would then ask the backend to forward pointer input");
        }
        let message = "dry run: no windows were modified".to_string();
        return Step::Done { message, input: None };
    }

    match backend.attach(raw, config) {
        Ok(outcome) => {
            let how =
                outcome.strategy_used.map_or_else(|| "unknown".to_string(), |s| format!("{s:?}"));
            let message = format!("attached to desktop using {how}");
            Step::Done { message, input: start_forwarding(backend, config) }
        }
        Err(err) => Step::Wait(format!("{:#}", anyhow::Error::new(err))),
    }
}

// A failure is only logged: a wallpaper that can't be clicked beats no wallpaper.
fn start_forwarding(
    backend: &mut Box<dyn WallpaperBackend>,
    config: &WallpaperConfig,
) -> Option<Box<dyn PointerSource>> {
    if !config.forward_input {
        debug!("pointer input forwarding disabled");
        return None;
    }

    match backend.forward_input() {
        Ok(Some(source)) => {
            info!("forwarding desktop pointer input to the window");
            Some(source)
        }
        Ok(None) => {
            debug!("backend reports the window receives input on its own");
            None
        }
        Err(err) => {
            warn!("pointer input forwarding unavailable: {:#}", anyhow::Error::new(err));
            None
        }
    }
}
