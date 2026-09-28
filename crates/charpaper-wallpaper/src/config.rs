use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachStrategy {
    /// Pick the right one for this machine.
    Auto,
    /// Windows 10 and older Windows 11: inside the top-level WorkerW behind the icons.
    #[value(name = "classic")]
    ClassicWorkerW,
    /// Newer Windows 11: a layered child of Progman, below the icons.
    #[value(name = "raised")]
    RaisedDesktopChild,
    /// Last resort: straight into Progman. Draws over the icons.
    #[value(name = "progman")]
    ProgmanDirect,
    /// Don't attach at all.
    None,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayeredMode {
    /// Only when the strategy needs it.
    Auto,
    Always,
    Never,
}

#[derive(Clone, Debug)]
pub struct WallpaperConfig {
    pub enabled: bool,
    pub dry_run: bool,
    pub dump_window_tree: bool,
    pub strategy: AttachStrategy,
    pub spawn_worker_w: bool,
    pub layered: LayeredMode,
    pub show_window_after_attach: bool,
    pub forward_input: bool,
    pub max_attempts: u32,
    pub frames_between_attempts: u32,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dry_run: false,
            dump_window_tree: false,
            strategy: AttachStrategy::Auto,
            spawn_worker_w: true,
            layered: LayeredMode::Auto,
            show_window_after_attach: true,
            forward_input: true,
            max_attempts: 20,
            frames_between_attempts: 15,
        }
    }
}
