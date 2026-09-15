//! The contract every operating system backend implements.

use raw_window_handle::RawWindowHandle;

use crate::AttachStrategy;
use crate::config::WallpaperConfig;
use crate::error::WallpaperError;

/// What a read-only look at the desktop found.
#[derive(Debug, Default)]
pub struct DesktopProbe {
    /// What `AttachStrategy::Auto` would resolve to on this machine.
    pub recommended: Option<AttachStrategy>,
}

/// What actually happened during an attach.
#[derive(Debug, Default)]
pub struct AttachOutcome {
    pub strategy_used: Option<AttachStrategy>,
}

/// One implementation per operating system.
///
/// `Send + Sync` so it can live in a Bevy `Resource`. The systems that call
/// into it are pinned to the main thread separately (see `NonSendMarker`
/// below), because window handles on Windows belong to the thread that made
/// them.
pub trait WallpaperBackend: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    /// Look at the desktop without changing anything. Safe to call any time.
    fn probe(&mut self, config: &WallpaperConfig) -> Result<DesktopProbe, WallpaperError>;

    /// Put `handle` behind the desktop icons.
    fn attach(
        &mut self,
        handle: RawWindowHandle,
        config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError>;
}
