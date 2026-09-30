use charpaper_scene::SceneConfig;
use charpaper_ui::UiConfig;
use charpaper_wallpaper::WallpaperConfig;

use crate::cli::Cli;
use crate::logging::LogLevels;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub scene: SceneConfig,
    pub wallpaper: WallpaperConfig,
    pub ui: UiConfig,
    pub log: LogLevels,
}

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub start_hidden: bool,
    pub decorations: bool,
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
    pub fn from_cli(cli: &Cli) -> Self {
        let mut cfg = Self {
            window: WindowConfig::default(),
            scene: SceneConfig::default(),
            wallpaper: WallpaperConfig::default(),
            ui: UiConfig::default(),
            log: LogLevels { ours: cli.log_level, engine: cli.bevy_log_level },
        };

        cfg.wallpaper.dump_window_tree = cli.tree;
        cfg.scene.suite = cli.suite.clone();
        cfg.ui.language = cli.language.clone();

        if cli.no_spawn_workerw {
            cfg.wallpaper.spawn_worker_w = false;
        }

        if cli.no_input_forwarding {
            cfg.wallpaper.forward_input = false;
        }

        if let Some(strategy) = cli.strategy {
            cfg.wallpaper.strategy = strategy;
        }

        if let Some(layered) = cli.layered {
            cfg.wallpaper.layered = layered;
        }

        // A dry run never reparents, so a hidden borderless window would stay invisible.
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
