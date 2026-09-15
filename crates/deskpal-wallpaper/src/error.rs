//! The error type backends return.

// Some variants are only ever constructed by one platform's backend, so on any
// given target a few look unused.
#[derive(Debug)]
pub enum WallpaperError {
    /// This OS has no backend yet.
    #[expect(unused)]
    Unsupported(&'static str),

    /// The window handle Bevy gave us isn't the kind this backend understands.
    WrongHandleKind,

    /// A required shell window could not be found.
    DesktopNotFound(String),

    /// A native call failed. Carries the OS error code where we have one.
    NativeCall { what: &'static str, code: u32 },
}

impl std::fmt::Display for WallpaperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(os) => write!(f, "no wallpaper backend for {os}"),
            Self::WrongHandleKind => write!(f, "window handle is not the expected native type"),
            Self::DesktopNotFound(what) => write!(f, "could not find {what}"),
            Self::NativeCall { what, code } => {
                write!(f, "{what} failed (OS error {code})")
            }
        }
    }
}
