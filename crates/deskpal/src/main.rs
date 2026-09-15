//! Entry point.
//!
//! The whole app is three plugins bolted onto a Bevy `App`:
//!
//! ```text
//!   DefaultPlugins   -- windowing, rendering, input, assets, logging
//!   ScenePlugin      -- what we draw            (deskpal-scene)
//!   WallpaperPlugin  -- where the window lives  (deskpal-wallpaper + backend)
//! ```
//!
//! Keeping those last two apart is the whole cross-platform strategy. The scene
//! never asks what OS it is on, and the wallpaper code never asks what is being
//! drawn.
//!
//! Start here if you are new to the project:
//!
//! ```text
//!   cargo run -- --inspect     what does this machine's desktop look like?
//!   cargo run -- --windowed    just run the scene in a normal window
//!   cargo run                  the real thing
//! ```

mod backend;
mod config;
mod plugin;

use bevy::prelude::*;
use bevy::window::WindowLevel;
use bevy::window::WindowResolution;
use deskpal_scene::ScenePlugin;

use crate::config::AppConfig;
use crate::plugin::WallpaperPlugin;

fn main() {
    let config = AppConfig::from_args();

    // `--inspect` is deliberately handled before Bevy exists. It opens no
    // window, allocates no GPU, and only reads. When something goes wrong on a
    // machine you cannot reproduce on, this is the first thing to run.
    if config.inspect_and_exit {
        for line in backend::inspect_report() {
            println!("{line}");
        }
        return;
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: config.window.title.clone(),
                resolution: WindowResolution::new(config.window.width, config.window.height),

                // No title bar, no border, not resizable by the user. A
                // wallpaper is furniture, not an application window.
                decorations: config.window.decorations,
                resizable: false,

                // Start hidden so the user never sees a normal window flash up
                // in the middle of the screen before we reparent it. The
                // wallpaper plugin flips this to `true` once attach finishes
                // (and also if attach fails, so we can never end up with an
                // invisible process you cannot close).
                visible: !config.window.start_hidden,

                // Keep it out of the taskbar and Alt-Tab. Bevy forwards this to
                // winit, which does the Win32 work for us -- one of several
                // places where we get to skip writing FFI by hand.
                skip_taskbar: config.window.skip_taskbar,

                // Explicitly *not* always-on-top. The desktop layer is below
                // everything; asking for topmost would fight the reparenting.
                window_level: WindowLevel::Normal,

                ..default()
            }),
            ..default()
        }))
        .add_plugins(WallpaperPlugin { config: config.wallpaper.clone() })
        .add_plugins(ScenePlugin { config: config.scene.clone() })
        .run();
}
