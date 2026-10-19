use std::io;
use std::path::Path;

use charpaper_wallpaper::WallpaperBackend;

// The only place the app picks an operating system. Supporting another one means a new
// backend crate, a target dependency in Cargo.toml, and one more set of items here.

// The backend's own command-line flags, for finding out why attaching fails on some machine.
#[cfg(windows)]
pub type BackendOptions = charpaper_windows::Options;

#[cfg(not(windows))]
#[derive(clap::Args, Clone, Debug, Default)]
pub struct BackendOptions {}

#[cfg(windows)]
pub fn create_backend(options: BackendOptions) -> Box<dyn WallpaperBackend> {
    Box::new(charpaper_windows::WindowsBackend::new(options))
}

#[cfg(not(windows))]
pub fn create_backend(_options: BackendOptions) -> Box<dyn WallpaperBackend> {
    Box::new(charpaper_wallpaper::UnsupportedBackend)
}

#[cfg(all(windows, feature = "diagnostics"))]
pub fn inspect_report() -> Vec<String> {
    charpaper_windows::inspect_report()
}

#[cfg(all(not(windows), feature = "diagnostics"))]
pub fn inspect_report() -> Vec<String> {
    vec![format!("no wallpaper backend for {}; nothing to inspect", std::env::consts::OS)]
}

// Whether `exe` starts when the user signs in. None where the app can't arrange it, which
// hides the switch.
#[cfg(windows)]
pub fn starts_at_login(exe: &Path) -> Option<io::Result<bool>> {
    Some(charpaper_windows::starts_at_login(exe))
}

#[cfg(windows)]
pub fn set_starts_at_login(exe: &Path, on: bool) -> io::Result<()> {
    charpaper_windows::set_starts_at_login(exe, on)
}

#[cfg(not(windows))]
pub fn starts_at_login(_exe: &Path) -> Option<io::Result<bool>> {
    None
}

#[cfg(not(windows))]
pub fn set_starts_at_login(_exe: &Path, _on: bool) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}
