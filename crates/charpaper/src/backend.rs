use charpaper_wallpaper::WallpaperBackend;

// The only place the app picks an operating system. Supporting another one means a new
// backend crate, a target dependency in Cargo.toml, and one more pair of functions here.

#[cfg(windows)]
pub fn create_backend() -> Box<dyn WallpaperBackend> {
    Box::new(charpaper_windows::WindowsBackend::new())
}

#[cfg(not(windows))]
pub fn create_backend() -> Box<dyn WallpaperBackend> {
    Box::new(charpaper_wallpaper::UnsupportedBackend::new())
}

#[cfg(windows)]
pub fn inspect_report() -> Vec<String> {
    charpaper_windows::inspect_report()
}

#[cfg(not(windows))]
pub fn inspect_report() -> Vec<String> {
    vec![format!("no wallpaper backend for {}; nothing to inspect", std::env::consts::OS)]
}
