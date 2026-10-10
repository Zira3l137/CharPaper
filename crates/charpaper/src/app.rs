use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use bevy::asset::io::AssetSourceBuilder;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy::window::WindowLevel;
use bevy::window::WindowResolution;
use charpaper_audio::SoundPlugin;
use charpaper_scene::ARMATURE_SOURCE;
use charpaper_scene::CHARACTERS_SOURCE;
use charpaper_scene::ScenePlugin;
use charpaper_ui::SettingsPanelPlugin;

use crate::benchmark;
use crate::benchmark::BenchmarkPlugin;
use crate::config::AppConfig;
use crate::config::WindowConfig;
use crate::logging;
use crate::save;
use crate::save::LookFilePlugin;
use crate::save::StatePlugin;
use crate::wallpaper::WallpaperPlugin;

const CHARACTERS_DIR: &str = "characters";
const LOCALES_DIR: &str = "locales";

// Everything the app reads and writes sits next to the executable: suites in
// characters/, translations in locales/, the viewer's choices in state.toml.
pub fn run(mut config: AppConfig) -> Result<()> {
    let exe_dir = exe_dir()?;
    config.scene.characters_dir = exe_dir.join(CHARACTERS_DIR);
    let state_path = exe_dir.join(save::STATE_FILE);
    let loaded = save::load(&state_path);
    config.scene.remembered = loaded.state.suites.clone();
    if config.scene.suite.is_none() {
        config.scene.suite = loaded.state.suite.clone();
    }
    config.scene.render = loaded.state.render.clone();
    if config.benchmark {
        benchmark::unthrottle(&mut config.scene.render);
    }
    config.scene.gaze = loaded.state.gaze.clone();
    config.ui.locales_dir = exe_dir.join(LOCALES_DIR);
    if config.ui.language.is_none() {
        config.ui.language = loaded.state.ui.language.clone();
    }
    let characters =
        config.scene.characters_dir.to_str().with_context(|| {
            format!("{} is not valid UTF-8", config.scene.characters_dir.display())
        })?;

    let source = || AssetSourceBuilder::platform_default(characters, None);
    let mut app = App::new();
    // Must come before DefaultPlugins, which builds the asset sources.
    app.register_asset_source(CHARACTERS_SOURCE, source())
        .register_asset_source(ARMATURE_SOURCE, source())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(window(&config.window)),
                    ..default()
                })
                .set(logging::plugin(config.log)),
        )
        .add_plugins((
            SoundPlugin { volumes: loaded.state.audio.clone() },
            WallpaperPlugin {
                config: config.wallpaper.clone(),
                screen: loaded.state.screen.clone(),
            },
            ScenePlugin { config: config.scene.clone() },
            SettingsPanelPlugin { config: config.ui.clone(), state: loaded.state.ui.clone() },
        ));
    // The benchmark changes settings and the look as it goes, none of which may be saved.
    if config.benchmark {
        app.add_plugins(BenchmarkPlugin { results: exe_dir.join(benchmark::RESULTS_FILE) });
    } else {
        app.add_plugins((LookFilePlugin, StatePlugin { path: state_path, loaded }));
    }
    let exit = app.run();

    match exit {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("bevy exited with status {code}"),
    }
}

fn window(config: &WindowConfig) -> Window {
    Window {
        title: config.title.clone(),
        resolution: WindowResolution::new(config.width, config.height),
        decorations: config.decorations,
        resizable: false,
        // The wallpaper plugin shows it once attached, so no floating window flashes by.
        visible: !config.start_hidden,
        skip_taskbar: config.skip_taskbar,
        // Not topmost: that fights the reparenting.
        window_level: WindowLevel::Normal,
        present_mode: if config.vsync { PresentMode::default() } else { PresentMode::AutoNoVsync },
        ..default()
    }
}

// Under `cargo run` this is `target/<profile>/`.
fn exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("cannot locate the executable")?;
    Ok(exe.parent().context("the executable has no parent folder")?.to_path_buf())
}
