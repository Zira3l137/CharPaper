// No Bevy here. This crate is the seam between the engine and the OS backends: backends
// implement WallpaperBackend without knowing Bevy exists, and the app drives them without
// knowing what an HWND is.

mod backend;
mod config;
mod error;
mod input;
mod unsupported;

pub use backend::AttachOutcome;
pub use backend::DesktopActivity;
pub use backend::DesktopProbe;
pub use backend::WallpaperBackend;
pub use config::AttachStrategy;
pub use config::LayeredMode;
pub use config::WallpaperConfig;
pub use error::WallpaperError;
pub use input::PointerButton;
pub use input::PointerEvent;
pub use input::PointerSource;
pub use raw_window_handle::RawWindowHandle;
pub use unsupported::UnsupportedBackend;
