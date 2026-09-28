//! Log configuration.
//!
//! Bevy's `LogPlugin` is a `tracing` subscriber, so there is no second logging
//! stack to set up: every crate in the workspace emits through `tracing` and
//! lands in it. All this module does is build the `EnvFilter` string that
//! separates our output from the engine's.

use bevy::log::DEFAULT_FILTER;
use bevy::log::Level;
use bevy::log::LogPlugin;

use crate::cli::LogLevel;

/// `EnvFilter` matches the crate name as the compiler spells it, hence the
/// underscores.
const OUR_CRATES: [&str; 7] = [
    "charpaper",
    "charpaper_bake",
    "charpaper_scene",
    "charpaper_suite",
    "charpaper_ui",
    "charpaper_wallpaper",
    "charpaper_windows",
];

pub fn plugin(ours: LogLevel, engine: LogLevel) -> LogPlugin {
    let ours = ours.as_directive();
    let mut filter = format!("{},{DEFAULT_FILTER}", engine.as_directive());
    filter.extend(OUR_CRATES.map(|krate| format!("{krate}={ours},")));

    // `LogPlugin::level` only becomes the leading bare directive of `filter`,
    // which the bare directive we write above then supersedes. TRACE here just
    // means the plugin never caps what the filter lets through.
    LogPlugin { filter, level: Level::TRACE, ..Default::default() }
}
