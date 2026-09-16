//! Entry point.
//!
//! `ScenePlugin` never asks what OS it is on and `WallpaperPlugin` never asks
//! what is being drawn; that separation is the cross-platform strategy.

mod backend;
mod cli;
mod config;
mod input;
mod logging;
mod plugin;

use anyhow::Result;
use anyhow::bail;
use bevy::prelude::*;
use bevy::window::WindowLevel;
use bevy::window::WindowResolution;
use charpaper_scene::ScenePlugin;
use charpaper_ui::CustomUiPlugin;

use crate::config::AppConfig;
use crate::plugin::WallpaperPlugin;

fn main() -> Result<()> {
    let args = cli::parse();
    let config = AppConfig::from_cli(&args);

    // Handled before Bevy exists: no window, no GPU, read-only.
    if config.inspect_and_exit {
        for line in backend::inspect_report() {
            println!("{line}");
        }
        return Ok(());
    }

    let exit = App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: config.window.title.clone(),
                        resolution: WindowResolution::new(
                            config.window.width,
                            config.window.height,
                        ),

                        decorations: config.window.decorations,
                        resizable: false,

                        // WallpaperPlugin reveals it once attach settles, so there is
                        // no flash of a floating window mid-reparent.
                        visible: !config.window.start_hidden,

                        skip_taskbar: config.window.skip_taskbar,

                        // Not topmost: the desktop layer is below everything, and
                        // asking for topmost fights the reparenting.
                        window_level: WindowLevel::Normal,

                        ..default()
                    }),
                    ..default()
                })
                .set(logging::plugin(args.log_level, args.bevy_log_level)),
        )
        .add_plugins(WallpaperPlugin { config: config.wallpaper.clone() })
        .add_plugins(ScenePlugin { config: config.scene.clone() })
        .add_plugins(CustomUiPlugin { config: config.ui.clone() })
        .run();

    match exit {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("bevy exited with status {code}"),
    }
}
