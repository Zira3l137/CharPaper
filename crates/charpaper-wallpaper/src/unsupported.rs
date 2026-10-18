use raw_window_handle::RawWindowHandle;

use crate::backend::ScreenArea;
use crate::backend::WallpaperBackend;
use crate::backend::WindowKey;
use crate::error::WallpaperError;

// For systems without a backend yet: the app runs as an ordinary window there.
#[derive(Default)]
pub struct UnsupportedBackend;

impl WallpaperBackend for UnsupportedBackend {
    fn name(&self) -> &'static str {
        "unsupported"
    }

    fn attach(
        &mut self,
        _window: WindowKey,
        _handle: RawWindowHandle,
        _area: ScreenArea,
    ) -> Result<(), WallpaperError> {
        Err(WallpaperError::Unsupported(std::env::consts::OS))
    }
}
