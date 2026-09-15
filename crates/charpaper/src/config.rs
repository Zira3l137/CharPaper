//! Application-level settings.
//!
//! Each domain crate owns and defaults its own settings; this type just
//! composes them and adds the handful that belong to the application itself.

use charpaper_scene::SceneConfig;
use charpaper_wallpaper::WallpaperConfig;

use crate::cli::Cli;

/// Everything the app needs to start up.
#[derive(Clone, Debug, Default)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub scene: SceneConfig,
    pub wallpaper: WallpaperConfig,

    /// `--inspect`: print a report about the desktop window layout and exit
    /// immediately, without ever opening a window. Completely read-only.
    pub inspect_and_exit: bool,
}

/// Settings for the OS window Bevy creates for us.
#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,

    /// Initial size. Once we attach to the desktop we resize to fill the
    /// desktop area anyway, so this only matters in `--windowed` mode.
    pub width: u32,
    pub height: u32,

    /// Start hidden, then reveal after the attach succeeds. This avoids a
    /// visible flash of a normal window in the middle of your screen while we
    /// reparent it.
    pub start_hidden: bool,

    /// Wallpapers have no title bar and no border.
    pub decorations: bool,

    /// A wallpaper should not show up in the taskbar or in Alt-Tab.
    pub skip_taskbar: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "charpaper".to_string(),
            width: 1280,
            height: 720,
            start_hidden: true,
            decorations: false,
            skip_taskbar: true,
        }
    }
}

impl AppConfig {
    /// Apply command-line overrides on top of the defaults.
    pub fn from_cli(cli: &Cli) -> Self {
        // Struct-update syntax rather than `let mut cfg = default(); cfg.x = ..`,
        // which clippy rightly flags as a missed initialiser.
        let mut cfg = Self { inspect_and_exit: cli.inspect, ..Self::default() };

        cfg.wallpaper.dump_window_tree = cli.tree;

        if cli.no_spawn_workerw {
            cfg.wallpaper.spawn_worker_w = false;
        }

        if let Some(strategy) = cli.strategy {
            cfg.wallpaper.strategy = strategy;
        }

        if let Some(layered) = cli.layered {
            cfg.wallpaper.layered = layered;
        }

        // A dry run never reparents, so a hidden borderless window would just
        // be invisible. Show it like a normal one.
        if cli.dry_run {
            cfg.wallpaper.dry_run = true;
            cfg.wallpaper.dump_window_tree = true;
            cfg.window.start_hidden = false;
            cfg.window.decorations = true;
        }

        if cli.windowed {
            cfg.wallpaper.enabled = false;
            cfg.window.start_hidden = false;
            cfg.window.decorations = true;
            cfg.window.skip_taskbar = false;
        }

        cfg
    }
}
