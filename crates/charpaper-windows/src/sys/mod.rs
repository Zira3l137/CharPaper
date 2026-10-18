use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Foundation::LPARAM;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Foundation::SetLastError;
use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
use windows_sys::Win32::Graphics::Gdi::EnumDisplayMonitors;
use windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::Graphics::Gdi::HMONITOR;
use windows_sys::Win32::Graphics::Gdi::MONITORINFO;
use windows_sys::Win32::Graphics::Gdi::MONITORINFOEXW;
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::Threading::GetExitCodeProcess;
use windows_sys::Win32::System::Threading::INFINITE;
use windows_sys::Win32::System::Threading::OpenProcess;
use windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows_sys::Win32::System::Threading::PROCESS_SYNCHRONIZE;
use windows_sys::Win32::System::Threading::WaitForSingleObject;
use windows_sys::Win32::UI::HiDpi::GetDpiForMonitor;
use windows_sys::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI;
use windows_sys::Win32::UI::WindowsAndMessaging::EnumChildWindows;
use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowExW;
use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
use windows_sys::Win32::UI::WindowsAndMessaging::GA_PARENT;
use windows_sys::Win32::UI::WindowsAndMessaging::GA_ROOT;
use windows_sys::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows_sys::Win32::UI::WindowsAndMessaging::GetAncestor;
use windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW;
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW;
use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
use windows_sys::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX;
use windows_sys::Win32::UI::WindowsAndMessaging::WindowFromPoint;
use windows_sys::core::BOOL;

// Safe wrappers for the Win32 calls several modules make. A call only one module makes is
// imported from windows-sys there.

// A window's handle. windows-sys makes it a raw pointer, which Rust won't let cross threads, but
// it is only a number naming a window, which Windows accepts from any thread of the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hwnd(pub HWND);

// SAFETY: as above; nothing is ever read or written through the pointer.
unsafe impl Send for Hwnd {}
unsafe impl Sync for Hwnd {}

impl Hwnd {
    pub const NULL: Self = Self(std::ptr::null_mut());

    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl Default for Hwnd {
    fn default() -> Self {
        Self::NULL
    }
}

// So the log shows handles as hex numbers, the way Spy++ and other tools do.
impl fmt::LowerHex for Hwnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&(self.0 as usize), f)
    }
}

// Keep the Vec alive while its pointer is used: `wide(s).as_ptr()` inline dangles.
pub fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    OsString::from_wide(&buffer[..end]).to_string_lossy().into_owned()
}

pub fn get_window_long_ptr(hwnd: Hwnd, index: WINDOW_LONG_PTR_INDEX) -> isize {
    unsafe { GetWindowLongPtrW(hwnd.0, index) }
}

// 0 means either failure or an old value of 0. Clearing the error code first lets
// callers tell them apart.
pub fn set_window_long_ptr(hwnd: Hwnd, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    unsafe {
        SetLastError(0);
        SetWindowLongPtrW(hwnd.0, index, value)
    }
}

pub fn last_error() -> u32 {
    unsafe { GetLastError() }
}

pub fn class_name(hwnd: Hwnd) -> String {
    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd.0, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    from_wide(&buffer[..len as usize])
}

pub fn window_title(hwnd: Hwnd) -> String {
    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd.0, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    from_wide(&buffer[..len as usize])
}

pub fn window_rect(hwnd: Hwnd) -> Option<RECT> {
    let mut rect = RECT::default();
    let ok = unsafe { GetWindowRect(hwnd.0, &mut rect) };
    (ok != 0).then_some(rect)
}

pub fn is_window(hwnd: Hwnd) -> bool {
    unsafe { IsWindow(hwnd.0) != 0 }
}

pub fn window_from_point(point: POINT) -> Hwnd {
    Hwnd(unsafe { WindowFromPoint(point) })
}

pub fn root_ancestor(hwnd: Hwnd) -> Hwnd {
    Hwnd(unsafe { GetAncestor(hwnd.0, GA_ROOT) })
}

// The window a child sits in. GetParent would answer with the owner for some windows instead.
pub fn parent(hwnd: Hwnd) -> Hwnd {
    Hwnd(unsafe { GetAncestor(hwnd.0, GA_PARENT) })
}

pub fn cursor_pos() -> Option<POINT> {
    let mut point = POINT::default();
    let ok = unsafe { GetCursorPos(&mut point) };
    (ok != 0).then_some(point)
}

pub fn screen_to_client(hwnd: Hwnd, screen: POINT) -> Option<POINT> {
    let mut point = screen;
    let ok = unsafe { ScreenToClient(hwnd.0, &mut point) };
    (ok != 0).then_some(point)
}

pub fn has_ex_style(hwnd: Hwnd, style: u32) -> bool {
    !hwnd.is_null() && (get_window_long_ptr(hwnd, GWL_EXSTYLE) & style as isize) != 0
}

pub fn find_window(class: &str) -> Hwnd {
    let class = wide(class);
    Hwnd(unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) })
}

pub fn find_window_ex(parent: Hwnd, after: Hwnd, class: &str) -> Hwnd {
    let class = wide(class);
    Hwnd(unsafe { FindWindowExW(parent.0, after.0, class.as_ptr(), std::ptr::null()) })
}

// `lparam` must be a `*mut Vec<Hwnd>` that outlives the enumeration.
unsafe extern "system" fn collect_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = unsafe { &mut *(lparam as *mut Vec<Hwnd>) };
    out.push(Hwnd(hwnd));
    1
}

pub fn top_level_windows() -> Vec<Hwnd> {
    let mut out: Vec<Hwnd> = Vec::new();
    unsafe { EnumWindows(Some(collect_callback), &mut out as *mut Vec<Hwnd> as LPARAM) };
    out
}

pub fn child_windows(parent: Hwnd) -> Vec<Hwnd> {
    let mut out: Vec<Hwnd> = Vec::new();
    if parent.is_null() {
        return out;
    }
    let lparam = &mut out as *mut Vec<Hwnd> as LPARAM;
    unsafe { EnumChildWindows(parent.0, Some(collect_callback), lparam) };
    out
}

// `lparam` must be a `*mut Vec<HMONITOR>` that outlives the enumeration.
unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _dc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let out = unsafe { &mut *(lparam as *mut Vec<HMONITOR>) };
    out.push(monitor);
    1
}

pub fn display_monitors() -> Vec<HMONITOR> {
    let mut out: Vec<HMONITOR> = Vec::new();
    let lparam = &mut out as *mut Vec<HMONITOR> as LPARAM;
    let everywhere = std::ptr::null();
    unsafe { EnumDisplayMonitors(std::ptr::null_mut(), everywhere, Some(collect_monitor), lparam) };
    out
}

pub fn monitor_info(monitor: HMONITOR) -> Option<MONITORINFO> {
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    let ok = !monitor.is_null() && unsafe { GetMonitorInfoW(monitor, &mut info) } != 0;
    ok.then_some(info)
}

// GetMonitorInfoW fills in the device name too when given the larger struct, with cbSize
// saying which of the two it got.
pub fn monitor_info_ex(monitor: HMONITOR) -> Option<MONITORINFOEXW> {
    // SAFETY: all zeroes is a valid MONITORINFOEXW, which holds only numbers.
    let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    let as_base = &mut info as *mut MONITORINFOEXW as *mut MONITORINFO;
    let ok = unsafe { GetMonitorInfoW(monitor, as_base) };
    (ok != 0).then_some(info)
}

// 96 is 100% scaling.
pub fn monitor_dpi(monitor: HMONITOR) -> Option<u32> {
    let (mut x, mut y) = (0, 0);
    let ok = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) } == 0;
    ok.then_some(x)
}

// Waits for another process to end and gives its exit code. None when it can't be watched,
// which includes having ended already.
pub fn wait_for_exit(pid: u32) -> Option<u32> {
    let access = PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION;
    let process = unsafe { OpenProcess(access, 0, pid) };
    if process.is_null() {
        return None;
    }
    let mut code = 0;
    let ended = unsafe { WaitForSingleObject(process, INFINITE) } == WAIT_OBJECT_0
        && unsafe { GetExitCodeProcess(process, &mut code) } != 0;
    unsafe { CloseHandle(process) };
    ended.then_some(code)
}
