use charpaper_wallpaper::DesktopActivity;
use charpaper_wallpaper::WindowKey;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Dwm::DWMWA_CLOAKED;
use windows_sys::Win32::Graphics::Dwm::DwmGetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST;
use windows_sys::Win32::Graphics::Gdi::MonitorFromWindow;
use windows_sys::Win32::System::Power::GetSystemPowerStatus;
use windows_sys::Win32::System::Power::SYSTEM_POWER_STATUS;
use windows_sys::Win32::UI::Shell::QUERY_USER_NOTIFICATION_STATE;
use windows_sys::Win32::UI::Shell::QUNS_BUSY;
use windows_sys::Win32::UI::Shell::QUNS_NOT_PRESENT;
use windows_sys::Win32::UI::Shell::QUNS_PRESENTATION_MODE;
use windows_sys::Win32::UI::Shell::QUNS_RUNNING_D3D_FULL_SCREEN;
use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;
use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::IsIconic;
use windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;

use crate::sys;
use crate::sys::Hwnd;

const SHELL_CLASSES: [&str; 4] = ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

// Every check leans towards not pausing: a wallpaper frozen while visible looks broken.
pub fn desktop_activity(windows: &[(WindowKey, Hwnd)]) -> DesktopActivity {
    let state = notification_state();
    DesktopActivity {
        away: state == Some(QUNS_NOT_PRESENT),
        fullscreen_app: fullscreen_app(state),
        covered: covered(windows),
        on_battery: on_battery(),
    }
}

// Windows' own summary of what the user is doing. The lock screen and the screen saver live on
// a desktop of their own, where none of our window checks can see them; this answer is the
// only one that covers them.
fn notification_state() -> Option<QUERY_USER_NOTIFICATION_STATE> {
    let mut state = 0;
    let ok = unsafe { SHQueryUserNotificationState(&mut state) } == 0;
    ok.then_some(state)
}

fn fullscreen_app(state: Option<QUERY_USER_NOTIFICATION_STATE>) -> bool {
    if matches!(state, Some(QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)) {
        return true;
    }

    // Borderless "fullscreen" games often go unreported, so a foreground window filling its
    // monitor counts too.
    let foreground = Hwnd(unsafe { GetForegroundWindow() });
    if foreground.is_null() || is_shell(foreground) {
        return false;
    }
    let monitor = unsafe { MonitorFromWindow(foreground.0, MONITOR_DEFAULTTONEAREST) };
    match (sys::window_rect(foreground), sys::monitor_info(monitor)) {
        (Some(window), Some(info)) => contains(&window, &info.rcMonitor),
        _ => false,
    }
}

// Each of our windows by the part of its monitor that the taskbar leaves free.
fn covered(windows: &[(WindowKey, Hwnd)]) -> Vec<WindowKey> {
    let work_areas: Vec<(WindowKey, RECT)> = windows
        .iter()
        .filter_map(|&(key, hwnd)| {
            let monitor = unsafe { MonitorFromWindow(hwnd.0, MONITOR_DEFAULTTONEAREST) };
            Some((key, sys::monitor_info(monitor)?.rcWork))
        })
        .collect();
    if work_areas.is_empty() {
        return Vec::new();
    }
    let shown: Vec<RECT> = sys::top_level_windows()
        .into_iter()
        .filter(|&hwnd| is_ordinary(hwnd))
        .filter_map(sys::window_rect)
        .collect();
    work_areas
        .into_iter()
        .filter(|(_, work)| shown.iter().any(|window| contains(window, work)))
        .map(|(key, _)| key)
        .collect()
}

fn on_battery() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    let ok = unsafe { GetSystemPowerStatus(&mut status) } != 0;
    ok && status.ACLineStatus == 0
}

fn is_ordinary(hwnd: Hwnd) -> bool {
    if unsafe { IsWindowVisible(hwnd.0) } == 0 || unsafe { IsIconic(hwnd.0) } != 0 {
        return false;
    }
    if sys::has_ex_style(hwnd, WS_EX_TOOLWINDOW) || is_shell(hwnd) {
        return false;
    }
    // Windows on other virtual desktops and suspended store apps count as visible but are
    // cloaked. On failure, treat the window as shown.
    let mut cloaked = 0u32;
    let size = size_of::<u32>() as u32;
    let value = &mut cloaked as *mut u32 as *mut _;
    let result = unsafe { DwmGetWindowAttribute(hwnd.0, DWMWA_CLOAKED as u32, value, size) };
    result != 0 || cloaked == 0
}

fn is_shell(hwnd: Hwnd) -> bool {
    let class = sys::class_name(hwnd);
    SHELL_CLASSES.contains(&class.as_str())
}

// A maximized window reaches a few pixels past the screen edge, so this is not equality.
fn contains(outer: &RECT, inner: &RECT) -> bool {
    outer.left <= inner.left
        && outer.top <= inner.top
        && outer.right >= inner.right
        && outer.bottom >= inner.bottom
}
