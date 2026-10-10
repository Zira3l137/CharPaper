use raw_window_handle::RawWindowHandle;

use crate::backend::AttachOutcome;
use crate::backend::DesktopProbe;
use crate::backend::ScreenArea;
use crate::backend::WallpaperBackend;
use crate::config::WallpaperConfig;
use crate::error::WallpaperError;

pub struct UnsupportedBackend;

impl UnsupportedBackend {
    pub fn new() -> Self {
        Self
    }
}

impl WallpaperBackend for UnsupportedBackend {
    fn name(&self) -> &'static str {
        "unsupported"
    }

    fn probe(&mut self, _config: &WallpaperConfig) -> Result<DesktopProbe, WallpaperError> {
        tracing::info!("no wallpaper backend for {}; running as a normal window", std::env::consts::OS);
        Ok(DesktopProbe { recommended: None })
    }

    fn attach(
        &mut self,
        _handle: RawWindowHandle,
        _area: ScreenArea,
        _config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError> {
        Err(WallpaperError::Unsupported(std::env::consts::OS))
    }
}
