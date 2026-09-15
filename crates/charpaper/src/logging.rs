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
const OUR_CRATES: [&str; 4] =
    ["charpaper", "charpaper_scene", "charpaper_wallpaper", "charpaper_windows"];

pub fn plugin(ours: LogLevel, engine: LogLevel) -> LogPlugin {
    let mut filter = format!("{},{}", engine.as_directive(), DEFAULT_FILTER);
    for krate in OUR_CRATES {
        filter.push_str(&format!("{krate}={},", ours.as_directive()));
    }

    // `LogPlugin::level` only becomes the leading bare directive of `filter`,
    // which the bare directive we write above then supersedes. TRACE here just
    // means the plugin never caps what the filter lets through.
    LogPlugin { filter, level: Level::TRACE, ..Default::default() }
}
