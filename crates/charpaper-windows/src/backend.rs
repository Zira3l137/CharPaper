use std::io;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

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
use charpaper_wallpaper::WindowKey;

use tracing::debug;

use crate::activity;
use crate::desktop;
use crate::monitors;
use crate::pointer::DesktopPointer;
use crate::sys;

// Shared with the pointer, which has to know about windows attached after it started too.
#[derive(Clone, Default)]
pub(crate) struct AttachedWindows(Arc<Mutex<Vec<(WindowKey, sys::Hwnd)>>>);

impl AttachedWindows {
    // Only the app's main thread locks it, so it never waits.
    pub fn list(&self) -> MutexGuard<'_, Vec<(WindowKey, sys::Hwnd)>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, window: WindowKey) -> Option<sys::Hwnd> {
        self.list().iter().find(|(key, _)| *key == window).map(|&(_, hwnd)| hwnd)
    }
}

#[derive(Default)]
pub struct WindowsBackend {
    attached: AttachedWindows,
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
        window: WindowKey,
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
            let mut list = self.attached.list();
            list.retain(|(key, _)| *key != window);
            list.push((window, hwnd));
        }

        Ok(AttachOutcome { strategy_used: Some(strategy) })
    }

    fn place(&mut self, window: WindowKey, area: ScreenArea) -> Result<(), WallpaperError> {
        match self.attached.get(window) {
            Some(hwnd) => desktop::place(hwnd, sys::parent(hwnd), area),
            None => Ok(()),
        }
    }

    fn detach(&mut self, window: WindowKey) {
        let mut list = self.attached.list();
        let gone: Vec<sys::Hwnd> =
            list.iter().filter(|(key, _)| *key == window).map(|&(_, hwnd)| hwnd).collect();
        list.retain(|(key, _)| *key != window);
        drop(list);
        desktop::leave(&gone);
    }

    fn release(&mut self) {
        let gone: Vec<sys::Hwnd> = self.attached.list().drain(..).map(|(_, hwnd)| hwnd).collect();
        desktop::leave(&gone);
    }

    // The raised desktop of newer Windows 11 shows no ghost.
    fn needs_keeper(&self) -> bool {
        !desktop::find_shell_windows().raised_desktop
    }

    // Out of the app's job when the job allows it, since a job can end all its processes at
    // once. No console either, which a debug build would otherwise open.
    fn spawn_keeper(&self, keeper: &mut Command) -> io::Result<()> {
        let quiet = sys::CREATE_NO_WINDOW;
        match keeper.creation_flags(quiet | sys::CREATE_BREAKAWAY_FROM_JOB).spawn() {
            Ok(_) => Ok(()),
            Err(_) => keeper.creation_flags(quiet).spawn().map(drop),
        }
    }

    // Exit code 0 is a clean quit, which repainted already. Any other, or a process gone
    // before it could be watched, may have left the ghost.
    fn keep(&mut self, app: u32) {
        if sys::wait_for_exit(app) != Some(0) {
            desktop::repaint();
        }
    }

    fn monitors(&mut self) -> Vec<DesktopMonitor> {
        monitors::monitors()
    }

    fn forward_input(
        &mut self,
        wake: Wake,
    ) -> Result<Option<Box<dyn PointerSource>>, WallpaperError> {
        if self.attached.list().is_empty() {
            return Ok(None);
        }
        Ok(Some(Box::new(DesktopPointer::new(self.attached.clone(), wake)?)))
    }

    fn activity(&mut self) -> DesktopActivity {
        activity::desktop_activity(&self.attached.list())
    }

    fn cursor_position(&mut self) -> Option<[i32; 2]> {
        sys::cursor_pos().map(|point| [point.x, point.y])
    }
}

pub fn inspect_report() -> Vec<String> {
    let shell = desktop::find_shell_windows();
    let mut out = vec![format!("recommended strategy: {:?}", shell.recommended_strategy())];
    out.extend(desktop::dump_window_tree());
    out
}
