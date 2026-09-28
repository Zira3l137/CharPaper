use raw_window_handle::RawWindowHandle;

use crate::AttachStrategy;
use crate::config::WallpaperConfig;
use crate::error::WallpaperError;
use crate::input::PointerSource;

#[derive(Debug, Default)]
pub struct DesktopProbe {
    pub recommended: Option<AttachStrategy>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DesktopActivity {
    pub fullscreen_app: bool,
    pub covered: bool,
    pub on_battery: bool,
}

#[derive(Debug, Default)]
pub struct AttachOutcome {
    pub strategy_used: Option<AttachStrategy>,
}

// Send + Sync so it fits in a Bevy Resource; callers keep to the main thread themselves.
pub trait WallpaperBackend: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn probe(&mut self, config: &WallpaperConfig) -> Result<DesktopProbe, WallpaperError>;

    fn attach(
        &mut self,
        handle: RawWindowHandle,
        config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError>;

    // Ok(None) means there is nothing to recover: the window gets its input on its own.
    fn forward_input(&mut self) -> Result<Option<Box<dyn PointerSource>>, WallpaperError> {
        Ok(None)
    }

    // Polled about once a second, so keep it cheap.
    fn activity(&mut self) -> DesktopActivity {
        DesktopActivity::default()
    }
}
