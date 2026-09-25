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
mod state;

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use bevy::asset::io::AssetSourceBuilder;
use bevy::prelude::*;
use bevy::window::WindowLevel;
use bevy::window::WindowResolution;
use charpaper_scene::CHARACTERS_SOURCE;
use charpaper_scene::ScenePlugin;
use charpaper_suite::ORBIT_CAMERA;
use charpaper_suite::Suite;
use charpaper_ui::CustomUiPlugin;

use crate::config::AppConfig;
use crate::plugin::WallpaperPlugin;
use crate::state::STATE_FILE;
use crate::state::StatePlugin;

/// Folder next to the executable that holds one sub-folder per suite.
const CHARACTERS_DIR: &str = "characters";

fn main() -> Result<()> {
    let args = cli::parse();
    let mut config = AppConfig::from_cli(&args);

    // Handled before Bevy exists: no window, no GPU, read-only.
    if config.inspect_and_exit {
        for line in backend::inspect_report() {
            println!("{line}");
        }
        return Ok(());
    }

    if let Some(path) = &args.check_suite {
        return check_suite(path);
    }

    let exe_dir = exe_dir()?;
    config.scene.characters_dir = exe_dir.join(CHARACTERS_DIR);
    let state_path = exe_dir.join(STATE_FILE);
    let loaded = state::load(&state_path);
    config.scene.remembered = loaded.state.suites.clone();
    let characters =
        config.scene.characters_dir.to_str().with_context(|| {
            format!("{} is not valid UTF-8", config.scene.characters_dir.display())
        })?;

    let exit = App::new()
        // Sources are built when `AssetPlugin` is added, so this has to come
        // before `DefaultPlugins`; registered later it only logs an error.
        .register_asset_source(
            CHARACTERS_SOURCE,
            AssetSourceBuilder::platform_default(characters, None),
        )
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin { file_path: String::from("../../assets"), ..default() })
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
        // TODO: Read UI locales into config on startup if available
        .add_plugins(CustomUiPlugin { config: config.ui.clone(), state: loaded.state.ui.clone() })
        .add_plugins(StatePlugin { path: state_path, saved: loaded.state, problem: loaded.problem })
        .run();

    match exit {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("bevy exited with status {code}"),
    }
}

/// Where the app keeps everything it reads and writes: suites in
/// `characters/`, choices in `state.toml`. Under `cargo run` that is
/// `target/<profile>/`.
fn exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("cannot locate the executable")?;
    Ok(exe.parent().context("the executable has no parent folder")?.to_path_buf())
}

/// Handled before Bevy exists, like `--inspect`, so it prints straight to
/// stdout rather than through the log.
fn check_suite(path: &Path) -> Result<()> {
    let suite =
        Suite::load(path).with_context(|| format!("cannot load suite at {}", path.display()))?;

    let skins: Vec<&str> = suite.skins.iter().map(|s| s.name.as_str()).collect();
    println!("suite    {:?}", suite.name);
    println!("model    {}", suite.model.display());
    println!("clips    {} animation file(s)", suite.animations.len());
    println!("skins    [{}], default {:?}", skins.join(", "), suite.default_skin);
    let cameras: Vec<&str> = std::iter::once(ORBIT_CAMERA)
        .chain(suite.cameras.iter().map(|c| c.name.as_str()))
        .collect();
    let default_camera = suite.default_camera.as_deref().unwrap_or(ORBIT_CAMERA);
    println!("cameras  [{}], default {default_camera:?}", cameras.join(", "));
    let show =
        |path: &Option<PathBuf>| path.as_ref().map_or("none".into(), |p| p.display().to_string());
    println!(
        "env      scene {}, skybox {}",
        show(&suite.environment.scene),
        show(&suite.environment.skybox)
    );

    let report = charpaper_suite::inspect(&suite);
    println!("{report}");
    if report.errors() > 0 {
        bail!("suite {:?} has {} error(s)", suite.name, report.errors());
    }
    Ok(())
}
