//! Platform-neutral wallpaper abstractions.
//!
//! This crate deliberately depends on nothing but `raw-window-handle`. It is
//! the seam between the engine half of the project and the operating-system
//! half: backends implement [`WallpaperBackend`] without knowing Bevy exists,
//! and the Bevy plugin drives them without knowing what an `HWND` is.

mod backend;
mod config;
mod error;
mod unsupported;

pub use backend::AttachOutcome;
pub use backend::DesktopProbe;
pub use backend::WallpaperBackend;
pub use config::AttachStrategy;
pub use config::LayeredMode;
pub use config::WallpaperConfig;
pub use error::WallpaperError;
// Re-exported so downstream crates match on the same `RawWindowHandle` type we
// do without having to declare the dependency themselves.
pub use raw_window_handle::RawWindowHandle;
pub use unsupported::UnsupportedBackend;
