use clap::Args;
use clap::ValueEnum;

// The ways of putting a window behind the desktop icons. Which one works depends on the
// Windows build; `Auto` picks it from the desktop's windows.
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
}

// For finding out why attaching fails on some machine; the defaults are what everyone runs.
#[derive(Args, Clone, Debug)]
pub struct Options {
    /// How to attach to the desktop.
    #[arg(long, value_enum, default_value = "auto")]
    pub strategy: AttachStrategy,

    /// Leave out the WS_EX_LAYERED style the raised desktop of newer Windows 11 asks for.
    ///
    /// Try this first if the log reports a successful attach but nothing renders.
    #[arg(long)]
    pub not_layered: bool,

    /// Do not send the undocumented 0x052C message asking Explorer to create
    /// the background WorkerW.
    #[arg(long)]
    pub no_spawn_workerw: bool,

    /// Log the desktop's window tree at startup.
    #[arg(long)]
    pub tree: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            strategy: AttachStrategy::Auto,
            not_layered: false,
            no_spawn_workerw: false,
            tree: false,
        }
    }
}
