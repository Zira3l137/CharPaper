//! Settings that describe *how* to attach, independent of any platform.

use clap::ValueEnum;

/// How we want the window glued to the desktop.
///
/// The `ValueEnum` derive is what lets `--strategy` take these by name, list
/// them in `--help` and reject anything else, with no parser of our own to
/// keep in sync.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachStrategy {
    /// Look at the machine and pick the right one. Almost always correct.
    Auto,

    /// Windows 10 / older Windows 11: parent to the top-level `WorkerW` window
    /// that sits behind the icon layer.
    #[value(name = "classic")]
    ClassicWorkerW,

    /// Newer Windows 11 ("raised desktop"): there is no top-level `WorkerW`.
    /// Become a layered child of `Progman`, z-ordered below the icons.
    #[value(name = "raised")]
    RaisedDesktopChild,

    /// Last resort: parent straight to `Progman`. Draws over the icons, which
    /// is wrong, but proves the pipeline works.
    #[value(name = "progman")]
    ProgmanDirect,

    /// Don't attach at all.
    None,
}

/// Whether our window gets the `WS_EX_LAYERED` extended style.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayeredMode {
    /// Add it only when the chosen strategy needs it (raised-desktop path).
    Auto,
    Always,
    Never,
}

/// Note there is no `Resource` derive here: that would drag Bevy into this
/// crate. The Bevy side wraps this in a newtype resource instead.
#[derive(Clone, Debug)]
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
