//! Application-level settings.
//!
//! Each domain crate owns and defaults its own settings; this type just
//! composes them and adds the handful that belong to the application itself.

use deskpal_scene::SceneConfig;
use deskpal_wallpaper::AttachStrategy;
use deskpal_wallpaper::LayeredMode;
use deskpal_wallpaper::WallpaperConfig;

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
            title: "deskpal".to_string(),
            width: 1280,
            height: 720,
            start_hidden: true,
            decorations: false,
            skip_taskbar: true,
        }
    }
}

impl AppConfig {
    /// Build the config from defaults, then apply command-line overrides.
    ///
    /// Unknown flags are reported and ignored rather than fatal, so a typo
    /// never leaves you staring at a silent process.
    pub fn from_args() -> Self {
        let mut cfg = Self::default();

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }

                "--inspect" => cfg.inspect_and_exit = true,
                "--tree" => cfg.wallpaper.dump_window_tree = true,

                "--dry-run" => {
                    cfg.wallpaper.dry_run = true;
                    cfg.wallpaper.dump_window_tree = true;
                    // In dry-run we never reparent, so the window would be
                    // invisible forever if we left it hidden.
                    cfg.window.start_hidden = false;
                    cfg.window.decorations = true;
                }

                "--windowed" => {
                    cfg.wallpaper.enabled = false;
                    cfg.window.start_hidden = false;
                    cfg.window.decorations = true;
                    cfg.window.skip_taskbar = false;
                }

                "--no-spawn-workerw" => cfg.wallpaper.spawn_worker_w = false,
                "--no-layered" => cfg.wallpaper.layered = LayeredMode::Never,
                "--force-layered" => cfg.wallpaper.layered = LayeredMode::Always,

                other => {
                    if let Some(value) = other.strip_prefix("--strategy=") {
                        match AttachStrategy::parse(value) {
                            Some(s) => cfg.wallpaper.strategy = s,
                            None => eprintln!(
                                "deskpal: unknown strategy {value:?}; \
                                 expected auto|classic|raised|progman|none"
                            ),
                        }
                    } else {
                        eprintln!("deskpal: ignoring unknown argument {other:?}");
                    }
                }
            }
        }

        cfg
    }
}
fn print_help() {
    println!(
        "\
deskpal -- a Bevy live wallpaper

USAGE:
    deskpal [FLAGS]

DIAGNOSTICS (safe, read-only):
    --inspect             Print what this machine's desktop window layout looks
                          like, then exit. Never opens a window, never modifies
                          anything. Start here when something doesn't work.
    --dry-run             Run the app in a normal decorated window, print the
                          full attach plan, but never touch the desktop.
    --tree                Dump the desktop window tree during startup.

MODES:
    --windowed            Skip the wallpaper machinery entirely and run as an
                          ordinary window. Use this while working on the scene.

ATTACH TUNING (Windows):
    --strategy=VALUE      auto (default) | classic | raised | progman | none
    --no-spawn-workerw    Don't send the 0x052C message to Progman.
    --no-layered          Never add WS_EX_LAYERED to our window.
    --force-layered       Always add WS_EX_LAYERED to our window.

    -h, --help            Show this message."
    );
}
