use bevy::log::DEFAULT_FILTER;
use bevy::log::Level;
use bevy::log::LogPlugin;
use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_directive(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

// Ours are charpaper's own crates; the engine is Bevy, wgpu and everything else.
#[derive(Clone, Copy, Debug)]
pub struct LogLevels {
    pub ours: LogLevel,
    pub engine: LogLevel,
}

const OUR_CRATES: [&str; 7] = [
    "charpaper",
    "charpaper_bake",
    "charpaper_scene",
    "charpaper_suite",
    "charpaper_ui",
    "charpaper_wallpaper",
    "charpaper_windows",
];

pub fn plugin(levels: LogLevels) -> LogPlugin {
    let ours = levels.ours.as_directive();
    let mut filter = format!("{},{DEFAULT_FILTER}", levels.engine.as_directive());
    filter.extend(OUR_CRATES.map(|krate| format!("{krate}={ours},")));

    // LogPlugin puts `level` in front of `filter`, where our own leading level overrides it.
    // TRACE only means the plugin itself never caps anything.
    LogPlugin { filter, level: Level::TRACE, ..Default::default() }
}
