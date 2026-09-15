//! A backend that does nothing.
//!
//! Used on platforms with no wallpaper-layer implementation yet. It lets the
//! whole project compile and run on Linux and macOS today as an ordinary
//! window, which matters more than it sounds: it means you can develop the 3D
//! scene anywhere, and it means `cargo check` on a non-Windows CI box is
//! meaningful.

use super::{DesktopBackend, DesktopLayerConfig};

pub struct NullBackend;

impl DesktopBackend for NullBackend {
    fn name(&self) -> &'static str {
        "null (no desktop integration on this platform)"
    }

    fn attach(&mut self, _cfg: &DesktopLayerConfig) -> Result<String, String> {
        Err("no wallpaper-layer backend for this platform; run with RunMode::Windowed".into())
    }

    fn is_attached(&self) -> bool {
        false
    }
}
