//! The typed half of the project's error handling. Libraries return this; the
//! binary converts it to `anyhow::Error` at its boundary.

use thiserror::Error;

// Some variants are only ever constructed by one platform's backend, so on any
// given target a few look unused.
#[derive(Debug, Error)]
pub enum WallpaperError {
    /// This OS has no backend yet.
    #[error("no wallpaper backend for {0}")]
    Unsupported(&'static str),

    /// The window handle Bevy gave us isn't the kind this backend understands.
    #[error("window handle is not the expected native type")]
    WrongHandleKind,

    /// A required shell window could not be found.
    #[error("could not find {0}")]
    DesktopNotFound(String),

    /// The OS code is wrapped in an `io::Error` because that is what turns
    /// `87` into "The parameter is incorrect". `#[source]` keeps it one level
    /// down, so `Display` stays short.
    #[error("{what} failed")]
    NativeCall {
        what: &'static str,
        #[source]
        source: std::io::Error,
    },
}

impl WallpaperError {
    /// Build a [`WallpaperError::NativeCall`] from a raw OS error code.
    pub fn native(what: &'static str, code: u32) -> Self {
        Self::NativeCall { what, source: std::io::Error::from_raw_os_error(code as i32) }
    }
}
