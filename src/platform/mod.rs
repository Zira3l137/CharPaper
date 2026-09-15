//! Platform abstraction for "put this window in the desktop wallpaper layer".
//!
//! ## Why this file exists
//!
//! The Windows implementation is a pile of undocumented window-manager tricks.
//! The Linux and macOS implementations will be *completely different* piles of
//! tricks (X11 override-redirect windows, Wayland's layer-shell protocol,
//! NSWindow levels). The one thing they have in common is the shape of the
//! problem, which is what this trait captures.
//!
//! The rule that keeps the port cheap: nothing outside `platform::` may ever see
//! an `HWND`, a Win32 constant, or a Windows error code. If you find yourself
//! wanting to return an `HWND` from here, add a method instead.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub mod null;

#[cfg(target_os = "windows")]
pub mod win32_sys;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub mod windows_inspect;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Where the window should live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunMode {
    /// Ordinary resizable window. No OS integration at all. This is the mode to
    /// develop the actual 3D scene in — it has a title bar you can close and it
    /// works identically on every platform.
    Windowed,
    /// Reparented into the desktop wallpaper layer.
    Wallpaper,
}

/// How much screen real estate to cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// Just the primary monitor. Other monitors keep their normal wallpaper.
    /// Start here: one camera, one viewport, no multi-monitor complications.
    PrimaryMonitor,
    /// The union of every monitor (the "virtual desktop"). Note that this is a
    /// single rectangle, so odd monitor arrangements leave dead zones inside it,
    /// and mismatched resolutions mean one camera stretched across everything.
    /// Doing this properly needs one viewport per monitor — a later milestone.
    VirtualDesktop,
}

/// Where to sit relative to the desktop icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZOrder {
    /// The real target: bottom of the sibling z-order, beneath the icon view.
    BelowIcons,
    /// Diagnostic only. Top of the sibling z-order, covering the icons.
    ///
    /// Use this to answer "are we rendering at all?". If the scene shows up here
    /// but not with `BelowIcons`, rendering works and the icon layer is opaque
    /// over us. If nothing shows up either way, the problem is the render
    /// surface, not the z-order.
    AboveIcons,
}

#[derive(Resource, Clone, Debug)]
pub struct DesktopLayerConfig {
    pub mode: RunMode,
    pub coverage: Coverage,
    /// Do all the discovery and log what *would* happen, but perform no
    /// reparenting, restyling or resizing. The app stays an ordinary window.
    /// Use this first on any machine you have not tried before — it tells you
    /// whether the WorkerW trick works on that Windows build without touching
    /// the user's desktop.
    pub dry_run: bool,
    /// Log each attach attempt and the window hierarchy that was found.
    pub verbose: bool,
    /// Dump the entire desktop window tree once, before touching anything.
    /// Pair it with `dry_run: true` when investigating a machine where the
    /// strategy ladder lands somewhere unexpected.
    pub inspect: bool,
    /// How often to verify we are still attached and re-attach if not.
    /// Explorer restarts, wallpaper changes and display reconfigurations all
    /// silently orphan us.
    pub reattach_check_seconds: f32,
    /// Enable the Ctrl+Alt+Shift+Q panic button. Strongly recommended while
    /// developing: a wallpaper window has no title bar and cannot be closed by
    /// any normal means.
    pub quit_hotkey: bool,
    pub window_title: String,
    pub z_order: ZOrder,
    pub nudge_after_attach: bool,
    pub heartbeat_seconds: f32,
}

impl Default for DesktopLayerConfig {
    fn default() -> Self {
        Self {
            mode: RunMode::Wallpaper,
            coverage: Coverage::PrimaryMonitor,
            dry_run: false,
            verbose: true,
            inspect: false,
            reattach_check_seconds: 2.0,
            window_title: "wallpaper-cube".to_string(),
            z_order: ZOrder::BelowIcons,
            nudge_after_attach: true,
            heartbeat_seconds: 5.0,
            quit_hotkey: true,
        }
    }
}

// ---------------------------------------------------------------------------
// The backend trait
// ---------------------------------------------------------------------------

/// One implementation per windowing system.
///
/// `Send + Sync` is required because this lives in a Bevy `Resource`. Raw
/// pointers are neither, which is why the Windows backend stores its handles as
/// `isize` and converts at the call site rather than holding an `HWND` field.
pub trait DesktopBackend: Send + Sync + 'static {
    /// Human-readable name, for logging.
    fn name(&self) -> &'static str;

    /// Try to put our window into the wallpaper layer.
    ///
    /// Returns a description of what happened on success, or an error string on
    /// failure. Failure is expected and normal — at login, Explorer often has
    /// not created the target window yet — so callers should retry rather than
    /// treat it as fatal.
    fn attach(&mut self, cfg: &DesktopLayerConfig) -> Result<String, String>;

    /// Cheap check: are we still where we put ourselves?
    fn is_attached(&self) -> bool;

    /// One-line dump of platform-level window state, for the heartbeat log.
    fn diagnostics(&self) -> String {
        String::new()
    }

    /// Has the user asked to quit via a platform-level hotkey?
    fn quit_requested(&self, _cfg: &DesktopLayerConfig) -> bool {
        false
    }
}

#[derive(Resource)]
struct Backend(Box<dyn DesktopBackend>);

/// Zero-sized marker stored as *non-send data*, used to pin a system to the
/// main thread.
///
/// ## Why this exists
///
/// Bevy's multi-threaded executor runs systems on worker threads from a task
/// pool. That is fine for game logic and fatal for window manipulation.
///
/// Many Win32 calls do not merely read state — they synchronously dispatch
/// messages to the thread that *owns* the window, and block until that thread
/// processes them. `SetParent`, `SetWindowPos` and `GetWindowTextW` all do this.
/// Our window is owned by winit's event loop thread, which is the main thread,
/// and the main thread is busy inside `app.update()` waiting for the schedule to
/// finish. A worker thread that blocks on the main thread while the main thread
/// waits for it is a deadlock: no message pump, so the window never paints, and
/// Windows reports it as not responding.
///
/// Bevy guarantees that any system taking non-send data as a parameter runs on
/// the main thread. A ZST marker is the cheapest way to claim that guarantee.
///
/// (`insert_non_send` was called `insert_non_send_resource` before Bevy 0.19,
/// which renamed it as part of making ordinary resources into components. The
/// old name still works but is deprecated.)
struct MainThreadOnly;

#[derive(Resource, Default)]
struct AttachState {
    attached: bool,
    seconds_since_check: f32,
    failures: u32,
}

fn select_backend() -> Box<dyn DesktopBackend> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsWallpaperBackend::new())
    }
    // Add `#[cfg(target_os = "linux")]` etc. here as they are implemented.
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(null::NullBackend)
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DesktopLayerPlugin(pub DesktopLayerConfig);

impl Plugin for DesktopLayerPlugin {
    fn build(&self, app: &mut App) {
        let backend = select_backend();
        info!(
            "desktop layer: backend = {}, mode = {:?}, dry_run = {}",
            backend.name(),
            self.0.mode,
            self.0.dry_run
        );

        app.insert_resource(self.0.clone())
            .insert_resource(Backend(backend))
            .insert_resource(AttachState::default())
            .insert_non_send(MainThreadOnly)
            .add_systems(Update, (maintain_attachment, poll_quit_hotkey, heartbeat));
    }
}

/// Attach on the first frame, then re-attach whenever we get orphaned.
///
/// This runs in `Update` rather than `Startup` on purpose. On `Startup` the
/// winit window may not be fully created yet, and on Windows 11 24H2 the target
/// `WorkerW` frequently does not exist until several seconds after login. A
/// retrying system handles both without any blocking sleeps, and doubles as the
/// watchdog for Explorer restarts.
fn maintain_attachment(
    // Pins this system to the main thread. Load-bearing — see `MainThreadOnly`.
    // Removing it produces a deadlock, not a warning.
    _main_thread: NonSend<MainThreadOnly>,
    time: Res<Time>,
    cfg: Res<DesktopLayerConfig>,
    mut backend: ResMut<Backend>,
    mut state: ResMut<AttachState>,
) {
    if cfg.mode == RunMode::Windowed {
        return;
    }

    state.seconds_since_check += time.delta_secs();

    if state.attached {
        if state.seconds_since_check < cfg.reattach_check_seconds {
            return;
        }
        state.seconds_since_check = 0.0;

        // In dry-run mode there is nothing to verify — we never attached.
        if cfg.dry_run || backend.0.is_attached() {
            return;
        }

        warn!(
            "desktop layer: window was orphaned (Explorer restart or wallpaper change?), re-attaching"
        );
        state.attached = false;
    }

    match backend.0.attach(&cfg) {
        Ok(details) => {
            info!("desktop layer: attached — {details}");
            state.attached = true;
            state.failures = 0;
            state.seconds_since_check = 0.0;
        }
        Err(err) => {
            state.failures += 1;
            // Log the first few failures, then go quiet. At login this can fail
            // for a couple of seconds straight and we do not want to spam.
            if state.failures <= 3 || state.failures % 100 == 0 {
                warn!("desktop layer: attach attempt {} failed — {err}", state.failures);
            }
        }
    }
}

/// Periodically log what the platform and Bevy each believe about our window.
///
/// When a window exists but displays nothing, the useful question is whether the
/// two agree. A Win32 rect of 1920x1080 next to a Bevy resolution of 1280x720
/// means the resize never reached Bevy and the surface is the wrong size. Both
/// reporting 1920x1080 means the surface is right and the problem is elsewhere.
fn heartbeat(
    time: Res<Time>,
    cfg: Res<DesktopLayerConfig>,
    backend: Res<Backend>,
    mut state: Local<f32>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if cfg.heartbeat_seconds <= 0.0 {
        return;
    }
    *state += time.delta_secs();
    if *state < cfg.heartbeat_seconds {
        return;
    }
    *state = 0.0;

    let bevy_view = match windows.iter().next() {
        Some(w) => format!(
            "bevy {}x{} visible={} focused={}",
            w.resolution.physical_width(),
            w.resolution.physical_height(),
            w.visible,
            w.focused
        ),
        None => "bevy has no primary window".to_string(),
    };
    info!("heartbeat: {} | {}", backend.0.diagnostics(), bevy_view);
}

/// Bevy renamed buffered "events" to "messages" in 0.17, so the writer type is
/// `MessageWriter` and the method is `write`. On Bevy <= 0.16 this would be
/// `EventWriter` and `send`. If you are ever unsure, `std::process::exit(0)`
/// works too — it just skips Bevy's shutdown, which for this app is harmless.
fn poll_quit_hotkey(
    cfg: Res<DesktopLayerConfig>,
    backend: Res<Backend>,
    mut exit: MessageWriter<AppExit>,
) {
    if cfg.quit_hotkey && backend.0.quit_requested(&cfg) {
        info!("desktop layer: quit hotkey pressed, shutting down");
        exit.write(AppExit::Success);
    }
}
