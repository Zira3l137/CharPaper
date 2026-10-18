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

pub fn plugin(levels: LogLevels) -> LogPlugin {
    let (ours, engine) = (levels.ours.as_directive(), levels.engine.as_directive());
    // A target matches by prefix, so `charpaper` covers every `charpaper_*` crate too.
    let filter = format!("{engine},{DEFAULT_FILTER},charpaper={ours}");

    // LogPlugin puts `level` in front of `filter`, where our own leading level overrides it.
    // TRACE only means the plugin itself never caps anything.
    LogPlugin { filter, level: Level::TRACE, ..Default::default() }
}
