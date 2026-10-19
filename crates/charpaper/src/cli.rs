use std::path::PathBuf;

use clap::Parser;

use crate::logging::LogLevel;
use crate::wallpaper::BackendOptions;

#[derive(Parser, Debug)]
#[command(name = "charpaper", version, about = "A Bevy live desktop wallpaper")]
pub struct Cli {
    /// Print this machine's desktop window layout and exit.
    ///
    /// Read-only: opens no window, allocates no GPU, modifies nothing. Start
    /// here when something does not work.
    #[cfg(feature = "diagnostics")]
    #[arg(long)]
    pub inspect: bool,

    /// Bake the missing reflection maps of every environment in a suite folder
    /// that has a panorama (`.hdr` or `.exr`), then exit.
    ///
    /// The app also does this itself the first time it shows such an
    /// environment. Existing maps are never touched; delete one to bake it
    /// again.
    #[arg(long, value_name = "DIR")]
    pub bake_environments: Option<PathBuf>,

    /// Measure how long the scene takes to draw with each render setting, then
    /// exit.
    ///
    /// Shows the suite as the wallpaper or in a window, as last chosen,
    /// with no frame-rate limit or vsync, for about two minutes. Each
    /// result changes one setting from your saved ones, which stay unchanged.
    /// Results are printed and written to benchmark.txt next to the executable.
    #[arg(long)]
    pub benchmark: bool,

    /// Which character suite to show: the name of a folder in `characters/`
    /// next to the executable. Defaults to the first in name order.
    #[arg(long, value_name = "NAME")]
    pub suite: Option<String>,

    /// The panel's language, as a tag such as `de-DE`, for a translation in
    /// `locales/` next to the executable. Remembered like a choice made in the
    /// System tab. Defaults to the one last chosen, then to the system's.
    #[arg(long, value_name = "TAG")]
    pub language: Option<String>,

    /// Run in an ordinary window and log what attaching would do, without
    /// touching any desktop window. With `--bake-environments`, only list what
    /// would be baked.
    #[arg(long)]
    pub dry_run: bool,

    /// Run in an ordinary window rather than behind the desktop icons.
    ///
    /// Remembered like a choice made in the System tab, which switches back.
    /// Use this while working on the scene, or when the wallpaper won't show.
    #[arg(long)]
    pub windowed: bool,

    #[cfg(feature = "diagnostics")]
    #[command(flatten)]
    pub backend: BackendOptions,

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

    /// Also write the log to charpaper.log next to the executable, as release
    /// builds always do. The run before's log is kept as charpaper.previous.log.
    #[arg(long)]
    pub log_file: bool,

    // The app starts itself with these to leave a keeper behind; see wallpaper/keeper.rs.
    #[arg(long, hide = true, value_name = "PID")]
    pub keep_desktop: Option<u32>,
    #[arg(long, hide = true, requires = "keep_desktop")]
    pub orphaned: bool,
}

impl Cli {
    #[cfg(feature = "diagnostics")]
    pub fn backend(&self) -> BackendOptions {
        self.backend.clone()
    }

    #[cfg(not(feature = "diagnostics"))]
    pub fn backend(&self) -> BackendOptions {
        BackendOptions::default()
    }
}
