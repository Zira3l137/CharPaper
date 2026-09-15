//! Picks the backend for the platform we are building for.
//!
//! The dispatch lives in the binary rather than in `charpaper-wallpaper` on
//! purpose: the abstraction crate must not depend on its implementers, or the
//! two would form a dependency cycle.
//!
//! Adding an OS means a new crate, a `[target.'cfg(...)'.dependencies]` entry
//! and one more pair of functions here.

use charpaper_wallpaper::WallpaperBackend;

#[cfg(windows)]
pub fn create_backend() -> Box<dyn WallpaperBackend> {
    Box::new(charpaper_windows::WindowsBackend::new())
}

#[cfg(not(windows))]
pub fn create_backend() -> Box<dyn WallpaperBackend> {
    Box::new(charpaper_wallpaper::UnsupportedBackend::new())
}

/// A read-only description of the desktop, for `--inspect`.
///
/// Callable before Bevy starts: it needs no window of our own, so it is the
/// safest possible first step when debugging a new machine.
#[cfg(windows)]
pub fn inspect_report() -> Vec<String> {
    charpaper_windows::inspect_report()
}

#[cfg(not(windows))]
pub fn inspect_report() -> Vec<String> {
    vec![format!("no wallpaper backend for {}; nothing to inspect", std::env::consts::OS)]
}
