use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::RawHandleWrapper;
use bevy::winit::EventLoopProxyWrapper;
use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::RawWindowHandle;
use charpaper_wallpaper::ScreenArea;
use charpaper_wallpaper::Wake;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;

use crate::wallpaper::Backend;
use crate::wallpaper::WallpaperSettings;
use crate::wallpaper::input::PointerOnDesktop;
use crate::wallpaper::input::PointerSourceResource;
use crate::wallpaper::monitors::Layout;
use crate::wallpaper::monitors::Target;

#[derive(Resource, Default)]
struct AttachState {
    finished: bool,
    attached: bool,
    attempts: u32,
    countdown: u32,
}

pub(crate) struct AttachPlugin;

impl Plugin for AttachPlugin {
    fn build(&self, app: &mut App) {
        let moved = resource_changed::<Target>.or_eager(resource_changed::<Layout>);
        app.init_resource::<AttachState>()
            .add_systems(Startup, probe_desktop)
            .add_systems(Update, (attach_window, follow_target.run_if(moved)).chain());
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
    mut windows: Query<(Option<&RawHandleWrapper>, &mut Window), With<PrimaryWindow>>,
    target: Res<Target>,
    on_desktop: Res<PointerOnDesktop>,
    proxy: Option<Res<EventLoopProxyWrapper>>,
) {
    if state.finished {
        return;
    }

    if state.countdown > 0 {
        state.countdown -= 1;
        return;
    }

    let handle =
        windows.single().ok().and_then(|(handle, _)| handle.map(|h| h.get_window_handle()));
    let area = target.0.as_ref().map(|monitor| monitor.area);
    let wake = on_desktop.waker(proxy.as_deref());
    let step = decide(&mut backend.0, &config, handle, area, wake);

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
        Step::Done { message, input, attached } => {
            info!("{message}");
            if let Some(source) = input {
                commands.insert_resource(PointerSourceResource::new(source));
            }
            state.finished = true;
            state.attached = attached;
            reveal = true;
        }
    }

    let Ok((_, mut window)) = windows.single_mut() else {
        return;
    };
    if let Some(monitor) = target.0.as_ref().filter(|_| state.attached) {
        match_scale(&mut window, monitor.scale);
    }
    if reveal && config.show_window_after_attach && !window.visible {
        window.visible = true;
    }
}

// After attaching, the window follows its monitor: to another one when the choice changes,
// and back into place when the layout shifts. A monitor added left of the primary one moves
// the desktop's corner that the window's position counts from, so even an unchanged area
// needs placing again.
fn follow_target(
    _main_thread: NonSendMarker,
    state: Res<AttachState>,
    mut backend: ResMut<Backend>,
    target: Res<Target>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(monitor) = target.0.as_ref().filter(|_| state.attached) else {
        return;
    };
    if let Err(err) = backend.0.place(monitor.area) {
        warn!("cannot move the wallpaper to {}: {:#}", monitor.name, anyhow::Error::new(err));
        return;
    }
    if let Ok(mut window) = windows.single_mut() {
        match_scale(&mut window, monitor.scale);
    }
}

// Windows tells only top-level windows about a monitor's scaling, never an attached one, so
// the panel would keep the size it had on the monitor the window was created on. Bevy lets
// the app state the scale instead.
fn match_scale(window: &mut Window, scale: f64) {
    let scale = Some(scale as f32);
    if window.resolution.scale_factor_override() != scale {
        window.resolution.set_scale_factor_override(scale);
    }
}

enum Step {
    Wait(String),
    Done { message: String, input: Option<Box<dyn PointerSource>>, attached: bool },
}

fn decide(
    backend: &mut Box<dyn WallpaperBackend>,
    config: &WallpaperConfig,
    handle: Option<RawWindowHandle>,
    area: Option<ScreenArea>,
    wake: Wake,
) -> Step {
    if !config.enabled || config.strategy == AttachStrategy::None {
        let message = "wallpaper attach disabled".to_string();
        return Step::Done { message, input: None, attached: false };
    }

    let Some(raw) = handle else {
        return Step::Wait("window handle not ready".to_string());
    };
    let Some(area) = area else {
        return Step::Wait("no monitor known yet".to_string());
    };

    if config.dry_run {
        debug!("[dry-run] would attach native handle {raw:?} over {area:?}");
        if config.forward_input {
            debug!("[dry-run] would then ask the backend to forward pointer input");
        }
        let message = "dry run: no windows were modified".to_string();
        return Step::Done { message, input: None, attached: false };
    }

    match backend.attach(raw, area, config) {
        Ok(outcome) => {
            let how =
                outcome.strategy_used.map_or_else(|| "unknown".to_string(), |s| format!("{s:?}"));
            let message = format!("attached to desktop using {how}");
            let attached = outcome.strategy_used.is_some_and(|s| s != AttachStrategy::None);
            Step::Done { message, input: start_forwarding(backend, config, wake), attached }
        }
        Err(err) => Step::Wait(format!("{:#}", anyhow::Error::new(err))),
    }
}

// A failure is only logged: a wallpaper that can't be clicked beats no wallpaper.
fn start_forwarding(
    backend: &mut Box<dyn WallpaperBackend>,
    config: &WallpaperConfig,
    wake: Wake,
) -> Option<Box<dyn PointerSource>> {
    if !config.forward_input {
        debug!("pointer input forwarding disabled");
        return None;
    }

    match backend.forward_input(wake) {
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
