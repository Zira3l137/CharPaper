//! What else is on screen: the answers behind "pause rendering when…".
//!
//! Each check errs towards not pausing. A wallpaper that keeps running when it
//! could have rested costs some power; one that freezes while visible looks
//! broken.

use charpaper_wallpaper::DesktopActivity;

use crate::sys;
use crate::sys::Hwnd;
use crate::sys::Rect;

/// Windows that belong to the desktop itself, so never cover or fill it.
const SHELL_CLASSES: [&str; 4] = ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

pub fn desktop_activity() -> DesktopActivity {
    DesktopActivity {
        fullscreen_app: fullscreen_app(),
        covered: covered(),
        on_battery: on_battery(),
    }
}

/// Windows' own answer first: it knows about exclusive-fullscreen Direct3D and
/// presentation mode. Borderless "fullscreen" games often go unreported,
/// though, so a foreground window filling its whole monitor counts too.
fn fullscreen_app() -> bool {
    let mut state = 0;
    let ok = unsafe { sys::SHQueryUserNotificationState(&mut state) } == 0;
    if ok
        && matches!(
            state,
            sys::QUNS_BUSY | sys::QUNS_RUNNING_D3D_FULL_SCREEN | sys::QUNS_PRESENTATION_MODE
        )
    {
        return true;
    }

    let foreground = unsafe { sys::GetForegroundWindow() };
    if foreground == 0 || is_shell(foreground) {
        return false;
    }
    let monitor = unsafe { sys::MonitorFromWindow(foreground, sys::MONITOR_DEFAULTTONEAREST) };
    match (sys::window_rect(foreground), monitor_info(monitor)) {
        (Some(window), Some(info)) => contains(&window, &info.rc_monitor),
        _ => false,
    }
}

/// Whether some ordinary window hides the primary monitor's whole work area,
/// the way a maximized one does. Only the primary monitor is looked at: with
/// the wallpaper spread over several, rendering pauses once the primary one is
/// covered even if another still shows it.
fn covered() -> bool {
    let primary =
        unsafe { sys::MonitorFromPoint(sys::Point { x: 0, y: 0 }, sys::MONITOR_DEFAULTTOPRIMARY) };
    let Some(info) = monitor_info(primary) else {
        return false;
    };
    sys::top_level_windows().into_iter().any(|hwnd| {
        is_ordinary(hwnd) && sys::window_rect(hwnd).is_some_and(|r| contains(&r, &info.rc_work))
    })
}

fn on_battery() -> bool {
    let mut status = sys::SystemPowerStatus::default();
    let ok = unsafe { sys::GetSystemPowerStatus(&mut status) } != 0;
    ok && status.ac_line_status == 0
}

/// Visible to the eye, not merely to the API: not minimized, not a tool
/// window, and not cloaked. Windows on other virtual desktops and suspended
/// store apps report themselves visible while the compositor hides them, and
/// would otherwise keep the wallpaper paused forever.
fn is_ordinary(hwnd: Hwnd) -> bool {
    if unsafe { sys::IsWindowVisible(hwnd) } == 0 || unsafe { sys::IsIconic(hwnd) } != 0 {
        return false;
    }
    if sys::has_ex_style(hwnd, sys::WS_EX_TOOLWINDOW) || is_shell(hwnd) {
        return false;
    }
    let mut cloaked = 0u32;
    let size = std::mem::size_of::<u32>() as u32;
    // An `HRESULT`: non-zero means the question failed, and then the window
    // counts as shown.
    let result =
        unsafe { sys::DwmGetWindowAttribute(hwnd, sys::DWMWA_CLOAKED, &mut cloaked, size) };
    result != 0 || cloaked == 0
}

fn is_shell(hwnd: Hwnd) -> bool {
    let class = sys::class_name(hwnd);
    SHELL_CLASSES.contains(&class.as_str())
}

fn monitor_info(monitor: sys::HMonitor) -> Option<sys::MonitorInfo> {
    let mut info = sys::MonitorInfo {
        cb_size: std::mem::size_of::<sys::MonitorInfo>() as u32,
        ..Default::default()
    };
    let ok = monitor != 0 && unsafe { sys::GetMonitorInfoW(monitor, &mut info) } != 0;
    ok.then_some(info)
}

/// A maximized window's rectangle reaches a few pixels past the screen edge,
/// its invisible resize borders, so "covers" means "contains", not "equals".
fn contains(outer: &Rect, inner: &Rect) -> bool {
    outer.left <= inner.left
        && outer.top <= inner.top
        && outer.right >= inner.right
        && outer.bottom >= inner.bottom
}
