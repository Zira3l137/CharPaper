//! The Windows implementation of `WallpaperBackend`.
//!
//! Thin on purpose: it translates between the platform-neutral trait and the
//! Win32 work in `desktop.rs`.

use charpaper_wallpaper::AttachOutcome;
use charpaper_wallpaper::DesktopProbe;
use charpaper_wallpaper::RawWindowHandle;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;
use charpaper_wallpaper::WallpaperError;

use tracing::debug;

use crate::desktop;
use crate::sys;

#[derive(Default)]
pub struct WindowsBackend {
    /// Remembered so a future `detach` (or a re-attach after Explorer
    /// restarts) has something to work with. Written but not yet read.
    attached_hwnd: Option<sys::Hwnd>,
}

impl WindowsBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WallpaperBackend for WindowsBackend {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn probe(&mut self, config: &WallpaperConfig) -> Result<DesktopProbe, WallpaperError> {
        let shell = desktop::find_shell_windows();

        if config.dump_window_tree {
            for line in desktop::dump_window_tree() {
                debug!("{line}");
            }
        }

        if shell.progman == 0 {
            return Err(WallpaperError::DesktopNotFound("Progman".to_string()));
        }

        Ok(DesktopProbe { recommended: Some(shell.recommended_strategy()) })
    }

    fn attach(
        &mut self,
        handle: RawWindowHandle,
        config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError> {
        // Bevy hands us a platform-agnostic enum; on Windows the payload is an
        // `HWND` stored as a `NonZeroIsize`.
        let RawWindowHandle::Win32(win32) = handle else {
            return Err(WallpaperError::WrongHandleKind);
        };
        let hwnd: sys::Hwnd = win32.hwnd.get();

        debug!("our window HWND = {hwnd:#x}");
        let strategy = desktop::attach(hwnd, config)?;
        self.attached_hwnd = Some(hwnd);

        Ok(AttachOutcome { strategy_used: Some(strategy) })
    }
}

/// Used by `--inspect`: a full read-only report, printed before Bevy starts.
pub fn inspect_report() -> Vec<String> {
    let shell = desktop::find_shell_windows();
    let mut out = vec![format!("recommended strategy: {:?}", shell.recommended_strategy())];
    out.extend(desktop::dump_window_tree());
    out
}
