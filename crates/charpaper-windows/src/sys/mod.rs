mod ffi;

use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;

pub use ffi::*;

// Keep the Vec alive while its pointer is used: `wide(s).as_ptr()` inline dangles.
pub fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    OsString::from_wide(&buffer[..end]).to_string_lossy().into_owned()
}

#[cfg(target_pointer_width = "64")]
pub fn get_window_long_ptr(hwnd: Hwnd, index: i32) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, index) }
}

#[cfg(target_pointer_width = "32")]
pub fn get_window_long_ptr(hwnd: Hwnd, index: i32) -> isize {
    unsafe { GetWindowLongW(hwnd, index) as isize }
}

// 0 means either failure or an old value of 0. Clearing the error code first lets
// callers tell them apart.
#[cfg(target_pointer_width = "64")]
pub fn set_window_long_ptr(hwnd: Hwnd, index: i32, value: isize) -> isize {
    unsafe {
        SetLastError(0);
        SetWindowLongPtrW(hwnd, index, value)
    }
}

#[cfg(target_pointer_width = "32")]
pub fn set_window_long_ptr(hwnd: Hwnd, index: i32, value: isize) -> isize {
    unsafe {
        SetLastError(0);
        SetWindowLongW(hwnd, index, value as i32) as isize
    }
}

pub fn last_error() -> u32 {
    unsafe { GetLastError() }
}

pub fn class_name(hwnd: Hwnd) -> String {
    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    from_wide(&buffer[..len as usize])
}

pub fn window_title(hwnd: Hwnd) -> String {
    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    from_wide(&buffer[..len as usize])
}

pub fn window_rect(hwnd: Hwnd) -> Option<Rect> {
    let mut rect = Rect::default();
    let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
    (ok != 0).then_some(rect)
}

pub fn is_window(hwnd: Hwnd) -> bool {
    unsafe { IsWindow(hwnd) != 0 }
}

pub fn window_from_point(point: Point) -> Hwnd {
    unsafe { WindowFromPoint(point) }
}

pub fn root_ancestor(hwnd: Hwnd) -> Hwnd {
    unsafe { GetAncestor(hwnd, GA_ROOT) }
}

// The window a child sits in. GetParent would answer with the owner for some windows instead.
pub fn parent(hwnd: Hwnd) -> Hwnd {
    unsafe { GetAncestor(hwnd, GA_PARENT) }
}

pub fn cursor_pos() -> Option<Point> {
    let mut point = Point::default();
    let ok = unsafe { GetCursorPos(&mut point) };
    (ok != 0).then_some(point)
}

pub fn screen_to_client(hwnd: Hwnd, screen: Point) -> Option<Point> {
    let mut point = screen;
    let ok = unsafe { ScreenToClient(hwnd, &mut point) };
    (ok != 0).then_some(point)
}

pub fn has_ex_style(hwnd: Hwnd, style: isize) -> bool {
    hwnd != 0 && (get_window_long_ptr(hwnd, GWL_EXSTYLE) & style) != 0
}

pub fn find_window(class: &str) -> Hwnd {
    let class = wide(class);
    unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }
}

pub fn find_window_ex(parent: Hwnd, after: Hwnd, class: &str) -> Hwnd {
    let class = wide(class);
    unsafe { FindWindowExW(parent, after, class.as_ptr(), std::ptr::null()) }
}

// `lparam` must be a `*mut Vec<Hwnd>` that outlives the enumeration.
unsafe extern "system" fn collect_callback(hwnd: Hwnd, lparam: LParam) -> Bool {
    let out = unsafe { &mut *(lparam as *mut Vec<Hwnd>) };
    out.push(hwnd);
    1
}

pub fn top_level_windows() -> Vec<Hwnd> {
    let mut out: Vec<Hwnd> = Vec::new();
    unsafe { EnumWindows(collect_callback, &mut out as *mut Vec<Hwnd> as LParam) };
    out
}

pub fn child_windows(parent: Hwnd) -> Vec<Hwnd> {
    let mut out: Vec<Hwnd> = Vec::new();
    if parent == 0 {
        return out;
    }
    unsafe { EnumChildWindows(parent, collect_callback, &mut out as *mut Vec<Hwnd> as LParam) };
    out
}

unsafe extern "system" fn collect_monitor(
    monitor: HMonitor,
    _dc: isize,
    _rect: *mut Rect,
    lparam: LParam,
) -> Bool {
    let out = unsafe { &mut *(lparam as *mut Vec<HMonitor>) };
    out.push(monitor);
    1
}

pub fn display_monitors() -> Vec<HMonitor> {
    let mut out: Vec<HMonitor> = Vec::new();
    let lparam = &mut out as *mut Vec<HMonitor> as LParam;
    unsafe { EnumDisplayMonitors(0, std::ptr::null(), collect_monitor, lparam) };
    out
}

pub fn monitor_info_ex(monitor: HMonitor) -> Option<MonitorInfoEx> {
    let mut info = MonitorInfoEx::default();
    info.info.cb_size = std::mem::size_of::<MonitorInfoEx>() as u32;
    let ok =
        unsafe { GetMonitorInfoW(monitor, &mut info as *mut MonitorInfoEx as *mut MonitorInfo) };
    (ok != 0).then_some(info)
}

// 96 is 100% scaling.
pub fn monitor_dpi(monitor: HMonitor) -> Option<u32> {
    let (mut x, mut y) = (0, 0);
    let ok = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) } == 0;
    ok.then_some(x)
}
