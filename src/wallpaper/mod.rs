//! The platform-neutral half of the wallpaper feature.
//!
//! Nothing in this file knows what Windows is. It defines:
//!
//!   * `WallpaperConfig` -- the settings,
//!   * `WallpaperBackend` -- the trait every OS backend implements,
//!   * `WallpaperPlugin`  -- the Bevy plugin that wires a backend into the app.
//!
//! Adding another OS later means writing one module that implements
//! `WallpaperBackend` and adding one `#[cfg]` line to `create_backend`. Nothing
//! else in the codebase changes.

#[cfg(not(target_os = "windows"))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::RawHandleWrapper;
use raw_window_handle::RawWindowHandle;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// How we want the window glued to the desktop.
///
/// See `docs/how-it-works.md` for what these actually mean on Windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachStrategy {
    /// Look at the machine and pick the right one. Almost always correct.
    Auto,

    /// Windows 10 / older Windows 11: parent to the top-level `WorkerW` window
    /// that sits behind the icon layer.
    ClassicWorkerW,

    /// Newer Windows 11 ("raised desktop"): there is no top-level `WorkerW`.
    /// Become a layered child of `Progman`, z-ordered below the icons.
    RaisedDesktopChild,

    /// Last resort: parent straight to `Progman`. Draws over the icons, which
    /// is wrong, but proves the pipeline works.
    ProgmanDirect,

    /// Don't attach at all.
    None,
}

impl AttachStrategy {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "auto" => Self::Auto,
            "classic" => Self::ClassicWorkerW,
            "raised" => Self::RaisedDesktopChild,
            "progman" => Self::ProgmanDirect,
            "none" => Self::None,
            _ => return None,
        })
    }
}

/// Whether our window gets the `WS_EX_LAYERED` extended style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayeredMode {
    /// Add it only when the chosen strategy needs it (raised-desktop path).
    Auto,
    Always,
    Never,
}

#[derive(Resource, Clone, Debug)]
pub struct WallpaperConfig {
    /// Master switch. `false` means "just be a normal window".
    pub enabled: bool,

    /// Do everything except the calls that actually modify a window. The log
    /// still tells you exactly what *would* have happened.
    pub dry_run: bool,

    /// Print the desktop window tree once at startup.
    pub dump_window_tree: bool,

    pub strategy: AttachStrategy,

    /// Ask the shell to create the background `WorkerW` window before we look
    /// for it (the famous undocumented `0x052C` message).
    pub spawn_worker_w: bool,

    pub layered: LayeredMode,

    /// Reveal the window once attach finishes. Pair with
    /// `WindowConfig::start_hidden` to avoid a flash of a floating window.
    pub show_window_after_attach: bool,

    /// The window handle does not exist on frame 0, and the shell sometimes
    /// needs a moment after login. Retry this many times before giving up.
    pub max_attempts: u32,

    /// Frames to wait between attempts. 30 frames is roughly half a second.
    pub frames_between_attempts: u32,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dry_run: false,
            dump_window_tree: false,
            strategy: AttachStrategy::Auto,
            spawn_worker_w: true,
            layered: LayeredMode::Auto,
            show_window_after_attach: true,
            max_attempts: 20,
            frames_between_attempts: 15,
        }
    }
}

// ---------------------------------------------------------------------------
// Backend contract
// ---------------------------------------------------------------------------

// Some variants are only ever constructed by one platform's backend, so on any
// given target a few look unused.
#[derive(Debug)]
pub enum WallpaperError {
    /// This OS has no backend yet.
    #[expect(unused)]
    Unsupported(&'static str),

    /// The window handle Bevy gave us isn't the kind this backend understands.
    WrongHandleKind,

    /// A required shell window could not be found.
    DesktopNotFound(String),

    /// A native call failed. Carries the OS error code where we have one.
    NativeCall { what: &'static str, code: u32 },
}

impl std::fmt::Display for WallpaperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(os) => write!(f, "no wallpaper backend for {os}"),
            Self::WrongHandleKind => write!(f, "window handle is not the expected native type"),
            Self::DesktopNotFound(what) => write!(f, "could not find {what}"),
            Self::NativeCall { what, code } => {
                write!(f, "{what} failed (OS error {code})")
            }
        }
    }
}

/// What a read-only look at the desktop found.
#[derive(Debug, Default)]
pub struct DesktopProbe {
    /// Human-readable lines to log. One per line, no trailing newlines.
    pub report: Vec<String>,
    /// What `AttachStrategy::Auto` would resolve to on this machine.
    pub recommended: Option<AttachStrategy>,
}

/// What actually happened during an attach.
#[derive(Debug, Default)]
pub struct AttachOutcome {
    pub strategy_used: Option<AttachStrategy>,
    pub notes: Vec<String>,
}

/// One implementation per operating system.
///
/// `Send + Sync` so it can live in a Bevy `Resource`. The systems that call
/// into it are pinned to the main thread separately (see `NonSendMarker`
/// below), because window handles on Windows belong to the thread that made
/// them.
pub trait WallpaperBackend: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    /// Look at the desktop without changing anything. Safe to call any time.
    fn probe(&mut self, config: &WallpaperConfig) -> Result<DesktopProbe, WallpaperError>;

    /// Put `handle` behind the desktop icons.
    fn attach(
        &mut self,
        handle: RawWindowHandle,
        config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError>;
}

#[cfg(target_os = "windows")]
pub fn create_backend() -> Box<dyn WallpaperBackend> {
    Box::new(windows::WindowsBackend::new())
}

#[cfg(not(target_os = "windows"))]
pub fn create_backend() -> Box<dyn WallpaperBackend> {
    Box::new(unsupported::UnsupportedBackend::new())
}

/// A read-only description of the desktop, for `--inspect`.
///
/// Deliberately callable before Bevy starts: it needs no window of our own, so
/// it is the safest possible first step when debugging a new machine.
#[cfg(target_os = "windows")]
pub fn inspect_report() -> Vec<String> {
    windows::inspect_report()
}

#[cfg(not(target_os = "windows"))]
pub fn inspect_report() -> Vec<String> {
    vec![format!("no wallpaper backend for {}; nothing to inspect", std::env::consts::OS)]
}

// ---------------------------------------------------------------------------
// Bevy plugin
// ---------------------------------------------------------------------------

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
        app.insert_resource(self.config.clone())
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
    config: Res<WallpaperConfig>,
) {
    info!("wallpaper backend: {}", backend.0.name());

    match backend.0.probe(&config) {
        Ok(probe) => {
            for line in &probe.report {
                info!("{line}");
            }
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
    config: Res<WallpaperConfig>,
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
        Step::Done(message, notes) => {
            for note in notes {
                info!("{note}");
            }
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
    /// Stop trying. Carries a summary line plus any backend notes to log.
    Done(String, Vec<String>),
}

fn decide(
    backend: &mut Box<dyn WallpaperBackend>,
    config: &WallpaperConfig,
    handle: Option<RawWindowHandle>,
) -> Step {
    if !config.enabled || config.strategy == AttachStrategy::None {
        return Step::Done("wallpaper attach disabled".to_string(), Vec::new());
    }

    // The window may simply not exist yet on the first frames.
    let Some(raw) = handle else {
        return Step::Wait("window handle not ready".to_string());
    };

    if config.dry_run {
        return Step::Done(
            "dry run: no windows were modified".to_string(),
            vec![format!("[dry-run] would attach native handle {raw:?}")],
        );
    }

    match backend.attach(raw, config) {
        Ok(outcome) => {
            let how =
                outcome.strategy_used.map_or_else(|| "unknown".to_string(), |s| format!("{s:?}"));
            Step::Done(format!("attached to desktop using {how}"), outcome.notes)
        }
        Err(err) => Step::Wait(err.to_string()),
    }
}
