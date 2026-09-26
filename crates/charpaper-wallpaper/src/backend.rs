//! The contract every operating system backend implements.

use raw_window_handle::RawWindowHandle;

use crate::AttachStrategy;
use crate::config::WallpaperConfig;
use crate::error::WallpaperError;
use crate::input::PointerSource;

/// What a read-only look at the desktop found.
#[derive(Debug, Default)]
pub struct DesktopProbe {
    /// What `AttachStrategy::Auto` would resolve to on this machine.
    pub recommended: Option<AttachStrategy>,
}

/// What else is going on on screen, for pausing the wallpaper while nobody can
/// see it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DesktopActivity {
    /// A game, video or presentation fills the screen.
    pub fullscreen_app: bool,
    /// Ordinary windows hide the whole desktop, a maximized one for example.
    pub covered: bool,
    /// The machine runs on battery rather than mains power.
    pub on_battery: bool,
}

/// What actually happened during an attach.
#[derive(Debug, Default)]
pub struct AttachOutcome {
    pub strategy_used: Option<AttachStrategy>,
}

/// One implementation per operating system.
///
/// `Send + Sync` so it can live in a Bevy `Resource`; callers pin themselves to
/// the main thread separately.
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

    /// Start recovering the pointer input that attaching took from our window.
    ///
    /// `Ok(None)` means there is nothing to recover: we are not attached, or
    /// this platform keeps delivering input to the window on its own. That is
    /// also the default, so backends that never swallow input skip this.
    fn forward_input(&mut self) -> Result<Option<Box<dyn PointerSource>>, WallpaperError> {
        Ok(None)
    }

    /// Look at what else is on screen. Polled about once a second, so it has
    /// to be cheap. The default knows nothing, which never pauses anything.
    fn activity(&mut self) -> DesktopActivity {
        DesktopActivity::default()
    }
}
