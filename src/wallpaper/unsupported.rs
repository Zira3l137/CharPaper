//! Backend used on every OS we haven't implemented yet.
//!
//! It does nothing and says so. The app still runs, you just get an ordinary
//! window instead of a wallpaper. This is what keeps `cargo check` honest on a
//! non-Windows machine.
//!
//! To add Linux support later: copy this file to `linux.rs`, implement the two
//! methods for real, and change the `#[cfg]` lines in `mod.rs`. Nothing outside
//! this directory needs to know.

use raw_window_handle::RawWindowHandle;

use super::{AttachOutcome, DesktopProbe, WallpaperBackend, WallpaperConfig, WallpaperError};

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
        Ok(DesktopProbe {
            report: vec![format!(
                "no wallpaper backend for {}; running as a normal window",
                std::env::consts::OS
            )],
            recommended: None,
        })
    }

    fn attach(
        &mut self,
        _handle: RawWindowHandle,
        _config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError> {
        Err(WallpaperError::Unsupported(std::env::consts::OS))
    }
}
