//! Command-line surface.
//!
//! `clap` owns parsing, validation, `--help` and `--version`. Every flag here
//! is an *override*: the fields are `Option<T>` or `bool` so that
//! `AppConfig::default()` in `config.rs` stays the single source of truth for
//! defaults, rather than having them written down twice.

use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::LayeredMode;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "charpaper", version, about = "A Bevy live desktop wallpaper")]
pub struct Cli {
    /// Print this machine's desktop window layout and exit.
    ///
    /// Read-only: opens no window, allocates no GPU, modifies nothing. Start
    /// here when something does not work.
    #[arg(long)]
    pub inspect: bool,

    /// Run in an ordinary window and log the full attach plan without touching
    /// any desktop window.
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
}

pub fn parse() -> Cli {
    Cli::parse()
}
