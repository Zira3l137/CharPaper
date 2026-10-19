use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use bevy::log::BoxedLayer;
use bevy::log::DEFAULT_FILTER;
use bevy::log::Level;
use bevy::log::LogPlugin;
use bevy::log::tracing_subscriber::Layer;
use bevy::log::tracing_subscriber::fmt;
use bevy::prelude::*;
use clap::ValueEnum;

pub const LOG_FILE: &str = "charpaper.log";
// The run before, so the log of a crash survives the start that follows it.
const PREVIOUS_LOG_FILE: &str = "charpaper.previous.log";

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

// Waits for the LogPlugin, whose layer hook is a plain fn and can only reach the app.
#[derive(Resource)]
pub struct LogFile(File);

pub fn open_file(dir: &Path) -> io::Result<LogFile> {
    let path = dir.join(LOG_FILE);
    let _ = fs::rename(&path, dir.join(PREVIOUS_LOG_FILE));
    File::create(path).map(LogFile)
}

// With a LogFile inserted before DefaultPlugins, the log goes to the file as well.
pub fn plugin(levels: LogLevels) -> LogPlugin {
    let (ours, engine) = (levels.ours.as_directive(), levels.engine.as_directive());
    // A target matches by prefix, so `charpaper` covers every `charpaper_*` crate too.
    let filter = format!("{engine},{DEFAULT_FILTER},charpaper={ours}");

    // LogPlugin puts `level` in front of `filter`, where our own leading level overrides it.
    // TRACE only means the plugin itself never caps anything.
    LogPlugin { filter, level: Level::TRACE, custom_layer: file_layer, ..Default::default() }
}

// The filter above covers this layer too: it decides for the whole logger, not one output.
fn file_layer(app: &mut App) -> Option<BoxedLayer> {
    let LogFile(file) = app.world_mut().remove_resource::<LogFile>()?;
    Some(fmt::layer().with_ansi(false).with_writer(Mutex::new(file)).boxed())
}

// A panic aborts at once, and a release build has no console to print it to, so it goes in
// the log too. The default hook still prints it afterwards.
pub fn log_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        error!("{info}");
        default(info);
    }));
}
