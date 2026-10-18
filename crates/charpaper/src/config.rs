use charpaper_scene::SceneConfig;
use charpaper_ui::UiConfig;
use charpaper_wallpaper::WallpaperConfig;

use crate::cli::Cli;
use crate::logging::LogLevels;
use crate::wallpaper::BackendOptions;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub scene: SceneConfig,
    pub wallpaper: WallpaperConfig,
    pub backend: BackendOptions,
    pub ui: UiConfig,
    pub log: LogLevels,
    pub benchmark: bool,
}

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub start_hidden: bool,
    pub decorations: bool,
    pub skip_taskbar: bool,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { start_hidden: true, decorations: false, skip_taskbar: true, vsync: true }
    }
}

impl AppConfig {
    pub fn from_cli(cli: &Cli) -> Self {
        let mut cfg = Self {
            window: WindowConfig::default(),
            scene: SceneConfig::default(),
            wallpaper: WallpaperConfig::default(),
            backend: cli.backend(),
            ui: UiConfig::default(),
            log: LogLevels { ours: cli.log_level, engine: cli.bevy_log_level },
            benchmark: cli.benchmark,
        };

        cfg.scene.suite = cli.suite.clone();
        cfg.ui.language = cli.language.clone();

        if cli.no_input_forwarding {
            cfg.wallpaper.forward_input = false;
        }

        // A dry run never reparents, so a hidden borderless window would stay invisible.
        if cli.dry_run {
            cfg.wallpaper.dry_run = true;
            cfg.window.start_hidden = false;
            cfg.window.decorations = true;
        }

        if cli.windowed {
            cfg.wallpaper.enabled = false;
            cfg.window.start_hidden = false;
            cfg.window.decorations = true;
            cfg.window.skip_taskbar = false;
        }

        // Vsync would hold every frame to the monitor's refresh, hiding what it really costs.
        if cli.benchmark {
            cfg.window.vsync = false;
        }

        cfg
    }
}
