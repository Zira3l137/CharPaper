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
use charpaper_monitor::MonitorPlugin;
use charpaper_scene::ARMATURE_SOURCE;
use charpaper_scene::CHARACTERS_SOURCE;
use charpaper_scene::ScenePlugin;
use charpaper_settings::DisplayMode;
use charpaper_settings::STATE_FILE;
use charpaper_ui::SettingsPanelPlugin;

use crate::benchmark;
use crate::benchmark::BenchmarkPlugin;
use crate::config::AppConfig;
use crate::config::WindowConfig;
use crate::logging;
use crate::pacing::PacingPlugin;
use crate::save::StatePlugin;
use crate::wallpaper::WallpaperPlugin;

const CHARACTERS_DIR: &str = "characters";
const LOCALES_DIR: &str = "locales";
// The size of the window when it isn't attached, as with --windowed. Attached, it covers its
// monitor.
const WINDOWED_SIZE: (u32, u32) = (1280, 720);

// Everything the app reads and writes sits next to the executable: suites in
// characters/, translations in locales/, the viewer's choices in state.toml.
pub fn run(mut config: AppConfig) -> Result<()> {
    let exe_dir = exe_dir()?;
    config.scene.characters_dir = exe_dir.join(CHARACTERS_DIR);
    config.ui.locales_dir = exe_dir.join(LOCALES_DIR);
    let state_path = exe_dir.join(STATE_FILE);
    let loaded = charpaper_settings::load(&state_path);
    let mut state = loaded.state.clone();
    // A language or mode asked for on the command line becomes the one chosen, and is saved
    // as such.
    if config.ui.language.is_some() {
        state.ui.language = config.ui.language.clone();
    }
    config.ui.language = state.ui.language.clone();
    if let Some(mode) = config.mode {
        state.system.mode = mode;
    }
    if state.system.mode == DisplayMode::Windowed {
        config.run_windowed();
    }
    if config.benchmark {
        benchmark::unthrottle(&mut state.render);
    }
    let characters =
        config.scene.characters_dir.to_str().with_context(|| {
            format!("{} is not valid UTF-8", config.scene.characters_dir.display())
        })?;

    let source = || AssetSourceBuilder::platform_default(characters, None);
    let mut app = App::new();
    // Reported once the logger exists, which is only after DefaultPlugins.
    let mut log_problem = None;
    if config.log_file {
        match logging::open_file(&exe_dir) {
            Ok(file) => {
                app.insert_resource(file);
            }
            Err(err) => log_problem = Some(err),
        }
    }
    // Must come before DefaultPlugins, which builds the asset sources.
    app.register_asset_source(CHARACTERS_SOURCE, source())
        .register_asset_source(ARMATURE_SOURCE, source())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin { primary_window: Some(window(&config.window)), ..default() })
                .set(logging::plugin(config.log)),
        )
        // The plugins below start from defaults wherever these aren't inserted first.
        .insert_resource(state.system)
        .insert_resource(state.ui)
        .insert_resource(state.render)
        .insert_resource(state.screen)
        .insert_resource(state.audio)
        .insert_resource(state.stages)
        .add_plugins((
            SoundPlugin,
            MonitorPlugin,
            WallpaperPlugin { config: config.wallpaper.clone(), backend: config.backend.clone() },
            ScenePlugin { config: config.scene.clone() },
            SettingsPanelPlugin { config: config.ui.clone() },
            PacingPlugin,
        ));
    if let Some(err) = log_problem {
        warn!("cannot write {}: {err}", exe_dir.join(logging::LOG_FILE).display());
    }
    logging::log_panics();
    // GPU time of each of Bevy's render passes, which Tracy shows beside the CPU's.
    #[cfg(feature = "profile")]
    app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
    // The benchmark changes settings and the look as it goes, none of which may be saved.
    if config.benchmark {
        app.add_plugins(BenchmarkPlugin { results: exe_dir.join(benchmark::RESULTS_FILE) });
    } else {
        app.add_plugins(StatePlugin { path: state_path, loaded });
    }
    let exit = app.run();

    match exit {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("bevy exited with status {code}"),
    }
}

fn window(config: &WindowConfig) -> Window {
    Window {
        title: "charpaper".into(),
        resolution: WindowResolution::new(WINDOWED_SIZE.0, WINDOWED_SIZE.1),
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
