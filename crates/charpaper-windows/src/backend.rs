//! The Windows implementation of `WallpaperBackend`.
//!
//! Thin on purpose: it translates between the platform-neutral trait and the
//! Win32 work in `desktop.rs`.

use charpaper_wallpaper::AttachOutcome;
use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::DesktopActivity;
use charpaper_wallpaper::DesktopProbe;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::RawWindowHandle;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;
use charpaper_wallpaper::WallpaperError;

use tracing::debug;

use crate::activity;
use crate::desktop;
use crate::pointer::DesktopPointer;
use crate::sys;

#[derive(Default)]
pub struct WindowsBackend {
    /// Only set once the window really is inside the desktop. Also what a
    /// future `detach` (or a re-attach after Explorer restarts) will need.
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
        // On Windows the payload is an `HWND` stored as a `NonZeroIsize`.
        let RawWindowHandle::Win32(win32) = handle else {
            return Err(WallpaperError::WrongHandleKind);
        };
        let hwnd: sys::Hwnd = win32.hwnd.get();

        debug!("our window HWND = {hwnd:#x}");
        let strategy = desktop::attach(hwnd, config)?;
        if strategy != AttachStrategy::None {
            self.attached_hwnd = Some(hwnd);
        }

        Ok(AttachOutcome { strategy_used: Some(strategy) })
    }

    fn forward_input(&mut self) -> Result<Option<Box<dyn PointerSource>>, WallpaperError> {
        let Some(hwnd) = self.attached_hwnd else {
            return Ok(None);
        };
        Ok(Some(Box::new(DesktopPointer::new(hwnd)?)))
    }

    fn activity(&mut self) -> DesktopActivity {
        activity::desktop_activity()
    }
}

/// Used by `--inspect`: a full read-only report, printed before Bevy starts.
pub fn inspect_report() -> Vec<String> {
    let shell = desktop::find_shell_windows();
    let mut out = vec![format!("recommended strategy: {:?}", shell.recommended_strategy())];
    out.extend(desktop::dump_window_tree());
    out
}
