use charpaper_wallpaper::AttachOutcome;
use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::DesktopActivity;
use charpaper_wallpaper::DesktopMonitor;
use charpaper_wallpaper::DesktopProbe;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::RawWindowHandle;
use charpaper_wallpaper::ScreenArea;
use charpaper_wallpaper::Wake;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;
use charpaper_wallpaper::WallpaperError;

use tracing::debug;

use crate::activity;
use crate::desktop;
use crate::monitors;
use crate::pointer::DesktopPointer;
use crate::sys;

#[derive(Default)]
pub struct WindowsBackend {
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
        area: ScreenArea,
        config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError> {
        let RawWindowHandle::Win32(win32) = handle else {
            return Err(WallpaperError::WrongHandleKind);
        };
        let hwnd: sys::Hwnd = win32.hwnd.get();

        debug!("our window HWND = {hwnd:#x}, going to {area:?}");
        let strategy = desktop::attach(hwnd, area, config)?;
        if strategy != AttachStrategy::None {
            self.attached_hwnd = Some(hwnd);
        }

        Ok(AttachOutcome { strategy_used: Some(strategy) })
    }

    fn place(&mut self, area: ScreenArea) -> Result<(), WallpaperError> {
        match self.attached_hwnd {
            Some(hwnd) => desktop::place(hwnd, sys::parent(hwnd), area),
            None => Ok(()),
        }
    }

    fn monitors(&mut self) -> Vec<DesktopMonitor> {
        monitors::monitors()
    }

    fn forward_input(
        &mut self,
        wake: Wake,
    ) -> Result<Option<Box<dyn PointerSource>>, WallpaperError> {
        let Some(hwnd) = self.attached_hwnd else {
            return Ok(None);
        };
        Ok(Some(Box::new(DesktopPointer::new(hwnd, wake)?)))
    }

    fn activity(&mut self) -> DesktopActivity {
        activity::desktop_activity(self.attached_hwnd)
    }

    fn cursor_position(&mut self) -> Option<[f32; 2]> {
        let point = sys::screen_to_client(self.attached_hwnd?, sys::cursor_pos()?)?;
        Some([point.x as f32, point.y as f32])
    }
}

pub fn inspect_report() -> Vec<String> {
    let shell = desktop::find_shell_windows();
    let mut out = vec![format!("recommended strategy: {:?}", shell.recommended_strategy())];
    out.extend(desktop::dump_window_tree());
    out
}
