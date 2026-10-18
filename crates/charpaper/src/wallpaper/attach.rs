use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::RawHandleWrapper;
use bevy::winit::EventLoopProxyWrapper;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::RawWindowHandle;
use charpaper_wallpaper::ScreenArea;
use charpaper_wallpaper::Wake;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;
use charpaper_wallpaper::WindowKey;

use crate::wallpaper::Backend;
use crate::wallpaper::WallpaperSettings;
use crate::wallpaper::input::PointerOnDesktop;
use crate::wallpaper::input::PointerSourceResource;
use crate::wallpaper::keeper;
use crate::wallpaper::key;
use crate::wallpaper::monitors::Layout;
use crate::wallpaper::windows::OnMonitor;

// A window that still can't attach after this many tries stays an ordinary window.
const MAX_ATTEMPTS: u32 = 20;
const FRAMES_BETWEEN_ATTEMPTS: u32 = 15;

// How far attaching a window got.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attachment {
    Pending { attempts: u32, countdown: u32 },
    // Behind the desktop icons.
    Attached,
    // An ordinary window, because attaching is off or kept failing.
    Loose,
}

impl Default for Attachment {
    fn default() -> Self {
        Self::Pending { attempts: 0, countdown: 0 }
    }
}

pub(crate) struct AttachPlugin;

impl Plugin for AttachPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_windows, place_windows).chain())
            .add_systems(Update, start_keeper.after(attach_windows))
            .add_systems(Last, leave_desktop);
    }
}

// Retried every few frames: a window's handle doesn't exist on its first frame, and Explorer
// may still be starting up right after login. NonSendMarker keeps a system on the main thread,
// which owns the windows.
fn attach_windows(
    _main_thread: NonSendMarker,
    mut commands: Commands,
    mut backend: ResMut<Backend>,
    config: Res<WallpaperSettings>,
    mut windows: Query<(
        Entity,
        &OnMonitor,
        &mut Attachment,
        Option<&RawHandleWrapper>,
        &mut Window,
    )>,
    on_desktop: Res<PointerOnDesktop>,
    proxy: Option<Res<EventLoopProxyWrapper>>,
    mut forwarding_started: Local<bool>,
) {
    for (window, on, mut attachment, handle, mut settings) in &mut windows {
        let Attachment::Pending { attempts, countdown } = *attachment else {
            continue;
        };
        if countdown > 0 {
            *attachment = Attachment::Pending { attempts, countdown: countdown - 1 };
            continue;
        }

        let handle = handle.map(RawHandleWrapper::get_window_handle);
        match decide(backend.0.as_mut(), &config, key(window), handle, on.0.area) {
            Step::Wait(reason) if attempts + 1 < MAX_ATTEMPTS => {
                debug!("attempt {} on {} not ready: {reason}", attempts + 1, on.0.name);
                let countdown = FRAMES_BETWEEN_ATTEMPTS;
                *attachment = Attachment::Pending { attempts: attempts + 1, countdown };
                continue;
            }
            Step::Wait(reason) => {
                warn!("giving up on {} after {} attempts: {reason}", on.0.name, attempts + 1);
                warn!("continuing as an ordinary window");
                *attachment = Attachment::Loose;
            }
            Step::Attached => {
                *attachment = Attachment::Attached;
                match_scale(&mut settings, on.0.scale);
            }
            Step::Loose(why) => {
                info!("{why}");
                *attachment = Attachment::Loose;
            }
        }
        // Shown on failure too: an invisible process with no way to close it is worse.
        if !settings.visible {
            settings.visible = true;
        }
    }

    // One source covers every window, those attached later included.
    if !*forwarding_started && windows.iter().any(|(_, _, a, _, _)| *a == Attachment::Attached) {
        *forwarding_started = true;
        let wake = on_desktop.waker(proxy.as_deref());
        if let Some(source) = start_forwarding(backend.0.as_mut(), &config, wake) {
            commands.insert_resource(PointerSourceResource::new(source));
        }
    }
}

// An attached window follows its monitor: to another one when it is given one, and back into
// place when the layout shifts. A monitor added left of the primary one moves the desktop's
// corner that a window's position counts from, so even an unchanged area needs placing again.
fn place_windows(
    _main_thread: NonSendMarker,
    mut backend: ResMut<Backend>,
    layout: Res<Layout>,
    mut windows: Query<(Entity, Ref<OnMonitor>, &Attachment, &mut Window)>,
) {
    for (window, on, attachment, mut settings) in &mut windows {
        if *attachment != Attachment::Attached || !(on.is_changed() || layout.is_changed()) {
            continue;
        }
        if let Err(err) = backend.0.place(key(window), on.0.area) {
            warn!("cannot move a window to {}: {:#}", on.0.name, anyhow::Error::new(err));
            continue;
        }
        match_scale(&mut settings, on.0.scale);
    }
}

// Once, when the first window is attached.
fn start_keeper(
    _main_thread: NonSendMarker,
    backend: Res<Backend>,
    windows: Query<&Attachment>,
    mut started: Local<bool>,
) {
    if *started || !windows.iter().any(|a| *a == Attachment::Attached) {
        return;
    }
    *started = true;
    if backend.0.needs_keeper() {
        keeper::start(backend.0.as_ref());
    }
}

// In the frame that quits, before the windows close.
fn leave_desktop(
    _main_thread: NonSendMarker,
    mut exits: MessageReader<AppExit>,
    mut backend: ResMut<Backend>,
) {
    if exits.read().count() > 0 {
        backend.0.release();
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
    Attached,
    // Stays an ordinary window, for this reason.
    Loose(&'static str),
}

fn decide(
    backend: &mut dyn WallpaperBackend,
    config: &WallpaperConfig,
    window: WindowKey,
    handle: Option<RawWindowHandle>,
    area: ScreenArea,
) -> Step {
    if !config.enabled {
        return Step::Loose("wallpaper attach disabled");
    }

    let Some(raw) = handle else {
        return Step::Wait("window handle not ready".to_string());
    };

    if config.dry_run {
        debug!("[dry-run] would attach native handle {raw:?} over {area:?}");
        if config.forward_input {
            debug!("[dry-run] would then ask the backend to forward pointer input");
        }
        return Step::Loose("dry run: no windows were modified");
    }

    match backend.attach(window, raw, area) {
        Ok(()) => Step::Attached,
        Err(err) => Step::Wait(format!("{:#}", anyhow::Error::new(err))),
    }
}

// A failure is only logged: a wallpaper that can't be clicked beats no wallpaper.
fn start_forwarding(
    backend: &mut dyn WallpaperBackend,
    config: &WallpaperConfig,
    wake: Wake,
) -> Option<Box<dyn PointerSource>> {
    if !config.forward_input {
        debug!("pointer input forwarding disabled");
        return None;
    }

    match backend.forward_input(wake) {
        Ok(Some(source)) => {
            info!("forwarding desktop pointer input to the windows");
            Some(source)
        }
        Ok(None) => {
            debug!("backend reports the windows receive input on their own");
            None
        }
        Err(err) => {
            warn!("pointer input forwarding unavailable: {:#}", anyhow::Error::new(err));
            None
        }
    }
}
