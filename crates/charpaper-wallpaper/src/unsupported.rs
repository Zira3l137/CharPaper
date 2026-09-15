//! Backend used on every OS we haven't implemented yet.
//!
//! It does nothing and says so. The app still runs, you just get an ordinary
//! window instead of a wallpaper. This is what keeps `cargo check` honest on a
//! non-Windows machine.
//!
//! To add Linux support later: add a `charpaper-linux` crate that implements the
//! same trait, and extend the `#[cfg]` dispatch in `charpaper/src/backend.rs`.
//! Nothing else changes.

use raw_window_handle::RawWindowHandle;

use crate::backend::AttachOutcome;
use crate::backend::DesktopProbe;
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
