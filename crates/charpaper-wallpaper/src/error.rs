use thiserror::Error;

#[derive(Debug, Error)]
pub enum WallpaperError {
    #[error("no wallpaper backend for {0}")]
    Unsupported(&'static str),

    #[error("window handle is not the expected native type")]
    WrongHandleKind,

    #[error("could not find {0}")]
    DesktopNotFound(String),

    #[error("{what} failed")]
    // io::Error turns the raw code into readable text like "The parameter is incorrect".
    NativeCall {
        what: &'static str,
        #[source]
        source: std::io::Error,
    },
}

impl WallpaperError {
    pub fn native(what: &'static str, code: u32) -> Self {
        Self::NativeCall { what, source: std::io::Error::from_raw_os_error(code as i32) }
    }
}
