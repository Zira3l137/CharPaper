mod backend;
mod cli;
mod config;
mod input;
mod logging;
mod look_file;
mod pacing;
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
use charpaper_bake::BakeSettings;
use charpaper_scene::ARMATURE_SOURCE;
use charpaper_scene::CHARACTERS_SOURCE;
use charpaper_scene::ScenePlugin;
use charpaper_suite::ORBIT_CAMERA;
use charpaper_suite::Suite;
use charpaper_ui::SettingsPanelPlugin;

use crate::config::AppConfig;
use crate::look_file::LookFilePlugin;
use crate::pacing::PacingPlugin;
use crate::plugin::WallpaperPlugin;
use crate::state::STATE_FILE;
use crate::state::StatePlugin;

const CHARACTERS_DIR: &str = "characters";

fn main() -> Result<()> {
    let args = cli::parse();
    let mut config = AppConfig::from_cli(&args);

    if config.inspect_and_exit {
        for line in backend::inspect_report() {
            println!("{line}");
        }
        return Ok(());
    }

    if let Some(path) = &args.check_suite {
        return check_suite(path);
    }

    if let Some(path) = &args.bake_environments {
        return bake_environments(path, &config.scene.bake, args.dry_run);
    }

    let exe_dir = exe_dir()?;
    config.scene.characters_dir = exe_dir.join(CHARACTERS_DIR);
    let state_path = exe_dir.join(STATE_FILE);
    let loaded = state::load(&state_path);
    config.scene.remembered = loaded.state.suites.clone();
    if config.scene.suite.is_none() {
        config.scene.suite = loaded.state.suite.clone();
    }
    config.scene.render = loaded.state.render.clone();
    let characters =
        config.scene.characters_dir.to_str().with_context(|| {
            format!("{} is not valid UTF-8", config.scene.characters_dir.display())
        })?;

    let exit = App::new()
        // Must come before DefaultPlugins, which builds the asset sources.
        .register_asset_source(
            CHARACTERS_SOURCE,
            AssetSourceBuilder::platform_default(characters, None),
        )
        .register_asset_source(
            ARMATURE_SOURCE,
            AssetSourceBuilder::platform_default(characters, None),
        )
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
                        // The wallpaper plugin shows it once attached, so no floating window flashes by.
                        visible: !config.window.start_hidden,
                        skip_taskbar: config.window.skip_taskbar,
                        // Not topmost: that fights the reparenting.
                        window_level: WindowLevel::Normal,

                        ..default()
                    }),
                    ..default()
                })
                .set(logging::plugin(args.log_level, args.bevy_log_level)),
        )
        .add_plugins(WallpaperPlugin { config: config.wallpaper.clone() })
        .add_plugins(PacingPlugin)
        .add_plugins(ScenePlugin { config: config.scene.clone() })
        .add_plugins(SettingsPanelPlugin { config: config.ui.clone(), state: loaded.state.ui.clone() })
        .add_plugins(LookFilePlugin)
        .add_plugins(StatePlugin { path: state_path, saved: loaded.state, problem: loaded.problem })
        .run();

    match exit {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("bevy exited with status {code}"),
    }
}

// Under `cargo run` this is `target/<profile>/`.
fn exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("cannot locate the executable")?;
    Ok(exe.parent().context("the executable has no parent folder")?.to_path_buf())
}

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
    let environments: Vec<&str> = suite.environments.iter().map(|e| e.name.as_str()).collect();
    println!("envs     [{}], default {:?}", environments.join(", "), suite.default_environment);

    let report = charpaper_suite::inspect(&suite);
    println!("{report}");
    if report.errors() > 0 {
        bail!("suite {:?} has {} error(s)", suite.name, report.errors());
    }
    Ok(())
}

fn bake_environments(path: &Path, settings: &BakeSettings, dry_run: bool) -> Result<()> {
    let suite =
        Suite::load(path).with_context(|| format!("cannot load suite at {}", path.display()))?;
    let mut any = false;
    for environment in &suite.environments {
        let Some(panorama) = &environment.panorama else {
            continue;
        };
        any = true;
        let folder = suite.absolute(&environment.folder());
        let missing = charpaper_bake::missing(&folder);
        let maps: Vec<&str> = missing.iter().map(|m| m.file_name()).collect();
        let name = &environment.name;
        if missing.is_empty() {
            println!("{name:<16} up to date");
        } else if dry_run {
            println!("{name:<16} would bake {} from {}", maps.join(", "), panorama.display());
        } else {
            println!("{name:<16} baking {} from {}", maps.join(", "), panorama.display());
            let started = std::time::Instant::now();
            charpaper_bake::bake(&suite.absolute(panorama), &folder, settings)
                .with_context(|| format!("cannot bake environment {name:?}"))?;
            println!("{name:<16} done in {:.1}s", started.elapsed().as_secs_f32());
        }
    }
    if !any {
        println!("no environment in {} has a panorama to bake from", path.display());
    }
    Ok(())
}
