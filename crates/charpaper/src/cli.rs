//! Command-line surface.
//!
//! `clap` owns parsing, validation, `--help` and `--version`. Every flag here
//! is an *override*: the fields are `Option<T>` or `bool` so that
//! `AppConfig::default()` in `config.rs` stays the single source of truth for
//! defaults, rather than having them written down twice.

use std::path::PathBuf;

use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::LayeredMode;
use clap::Parser;
use clap::ValueEnum;

#[derive(Parser, Debug)]
#[command(name = "charpaper", version, about = "A Bevy live desktop wallpaper")]
pub struct Cli {
    /// Print this machine's desktop window layout and exit.
    ///
    /// Read-only: opens no window, allocates no GPU, modifies nothing. Start
    /// here when something does not work.
    #[arg(long)]
    pub inspect: bool,

    /// Check a character suite folder and exit.
    ///
    /// Reads only the structure of its files and opens no window. Exits with
    /// an error if the suite would fail to load; warnings do not fail it.
    #[arg(long, value_name = "DIR")]
    pub check_suite: Option<PathBuf>,

    /// Bake the missing reflection maps of every environment in a suite folder
    /// that has a panorama (`.hdr` or `.exr`), then exit.
    ///
    /// The app also does this itself the first time it shows such an
    /// environment. Existing maps are never touched; delete one to bake it
    /// again.
    #[arg(long, value_name = "DIR")]
    pub bake_environments: Option<PathBuf>,

    /// Which character suite to show: the name of a folder in `characters/`
    /// next to the executable. Defaults to the first in name order.
    #[arg(long, value_name = "NAME")]
    pub suite: Option<String>,

    /// Run in an ordinary window and log the full attach plan without touching
    /// any desktop window. With `--bake-environments`, only list what would be
    /// baked.
    #[arg(long)]
    pub dry_run: bool,

    /// Dump the desktop window tree during startup.
    #[arg(long)]
    pub tree: bool,

    /// Skip the wallpaper machinery entirely and run as an ordinary window.
    ///
    /// Use this while working on the scene.
    #[arg(long)]
    pub windowed: bool,

    /// How to attach to the desktop.
    #[arg(long, value_enum)]
    pub strategy: Option<AttachStrategy>,

    /// Whether to give our window the WS_EX_LAYERED extended style.
    ///
    /// Try `never` first if the log reports a successful attach but nothing
    /// renders.
    #[arg(long, value_enum)]
    pub layered: Option<LayeredMode>,

    /// Do not send the undocumented 0x052C message asking Explorer to create
    /// the background WorkerW.
    #[arg(long)]
    pub no_spawn_workerw: bool,

    /// Do not replay the mouse input the desktop keeps from the wallpaper.
    ///
    /// The wallpaper still renders; it just stops reacting to the mouse.
    #[arg(long)]
    pub no_input_forwarding: bool,

    /// Log level for charpaper's own crates.
    #[arg(long, value_enum, default_value = "info")]
    pub log_level: LogLevel,

    /// Log level for Bevy, wgpu and everything else.
    #[arg(long, value_enum, default_value = "warn")]
    pub bevy_log_level: LogLevel,
}

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

pub fn parse() -> Cli {
    Cli::parse()
}
