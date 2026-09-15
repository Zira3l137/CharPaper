//! The error type backends return.
//!
//! This is the *typed* half of the project's error handling: a closed set of
//! things that can go wrong, which a caller can match on. `thiserror` writes
//! the `Display` and `Error` impls from the `#[error]` attributes, so adding a
//! variant no longer means editing a `match` somewhere else.
//!
//! Libraries return this; the binary converts it to `anyhow::Error` at its
//! boundary. That split is why no crate here depends on both.

use thiserror::Error;

// Some variants are only ever constructed by one platform's backend, so on any
// given target a few look unused.
#[allow(dead_code)]
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

    /// A native call failed.
    ///
    /// The OS code is wrapped in an `io::Error` rather than stored bare,
    /// because that is what turns `87` into "The parameter is incorrect".
    /// `#[source]` puts it one level down the chain, so `Display` stays short
    /// and the detail appears when something walks the chain.
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
