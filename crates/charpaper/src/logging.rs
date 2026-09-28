use bevy::log::DEFAULT_FILTER;
use bevy::log::Level;
use bevy::log::LogPlugin;

use crate::cli::LogLevel;

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

    // LogPlugin puts `level` in front of `filter`, where our own leading level overrides it.
    // TRACE only means the plugin itself never caps anything.
    LogPlugin { filter, level: Level::TRACE, ..Default::default() }
}
