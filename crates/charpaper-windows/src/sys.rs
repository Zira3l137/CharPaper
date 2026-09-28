#![allow(non_snake_case)]

use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;

pub type Hwnd = isize;
pub type Bool = i32;
pub type WParam = usize;
pub type LParam = isize;
pub type LResult = isize;
pub type HHook = isize;
pub type HInstance = isize;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Msg {
    pub hwnd: Hwnd,
    pub message: u32,
    pub wparam: WParam,
    pub lparam: LParam,
    pub time: u32,
    pub pt: Point,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MsLlHookStruct {
    pub pt: Point,
    pub mouse_data: u32,
    pub flags: u32,
    pub time: u32,
    pub extra_info: usize,
}

pub type EnumWindowsProc = unsafe extern "system" fn(Hwnd, LParam) -> Bool;

pub type HookProc = unsafe extern "system" fn(i32, WParam, LParam) -> LResult;

pub const GWL_STYLE: i32 = -16;
pub const GWL_EXSTYLE: i32 = -20;

pub const WS_CHILD: isize = 0x4000_0000;

pub const WS_EX_LAYERED: isize = 0x0008_0000;

// Windows 11 sets this on Progman in the newer "raised desktop" layout; that's how we detect it.
pub const WS_EX_NOREDIRECTIONBITMAP: isize = 0x0020_0000;

pub const LWA_ALPHA: u32 = 0x0000_0002;

pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;

pub const HWND_BOTTOM: Hwnd = 1;

pub const SMTO_NORMAL: u32 = 0x0000;

// Undocumented: asks Progman to create the WorkerW window behind the icons.
pub const WM_SPAWN_WORKER_W: u32 = 0x052C;

pub const WH_MOUSE_LL: i32 = 14;
pub const HC_ACTION: i32 = 0;

pub const WM_QUIT: u32 = 0x0012;
pub const WM_USER: u32 = 0x0400;
pub const PM_NOREMOVE: u32 = 0x0000;

pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_XBUTTONDOWN: u32 = 0x020B;
pub const WM_XBUTTONUP: u32 = 0x020C;
pub const WM_MOUSEHWHEEL: u32 = 0x020E;

pub const XBUTTON1: u16 = 0x0001;
pub const XBUTTON2: u16 = 0x0002;

pub const WHEEL_DELTA: i16 = 120;

pub const GA_ROOT: u32 = 2;

pub type HMonitor = isize;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MonitorInfo {
    pub cb_size: u32,
    pub rc_monitor: Rect,
    pub rc_work: Rect,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemPowerStatus {
    pub ac_line_status: u8,
    pub battery_flag: u8,
    pub battery_life_percent: u8,
    pub system_status_flag: u8,
    pub battery_life_time: u32,
    pub battery_full_life_time: u32,
}

pub const MONITOR_DEFAULTTOPRIMARY: u32 = 1;
pub const MONITOR_DEFAULTTONEAREST: u32 = 2;

pub const WS_EX_TOOLWINDOW: isize = 0x0000_0080;

pub const DWMWA_CLOAKED: u32 = 14;

pub const QUNS_BUSY: i32 = 2;
pub const QUNS_RUNNING_D3D_FULL_SCREEN: i32 = 3;
pub const QUNS_PRESENTATION_MODE: i32 = 4;

// `extern "system"` is Win32's calling convention. Never use "C" here, even though it
// happens to work on 64-bit.
#[link(name = "user32")]
unsafe extern "system" {
    pub fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Hwnd;

    pub fn FindWindowExW(
        parent: Hwnd,
        child_after: Hwnd,
        class_name: *const u16,
        window_name: *const u16,
    ) -> Hwnd;

    pub fn EnumWindows(callback: EnumWindowsProc, lparam: LParam) -> Bool;

    pub fn EnumChildWindows(parent: Hwnd, callback: EnumWindowsProc, lparam: LParam) -> Bool;

    pub fn SendMessageTimeoutW(
        hwnd: Hwnd,
        msg: u32,
        wparam: WParam,
        lparam: LParam,
        flags: u32,
        timeout_ms: u32,
        result: *mut usize,
    ) -> LResult;

    pub fn SetParent(child: Hwnd, new_parent: Hwnd) -> Hwnd;

    pub fn SetWindowPos(
        hwnd: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> Bool;

    pub fn SetLayeredWindowAttributes(hwnd: Hwnd, color_key: u32, alpha: u8, flags: u32) -> Bool;

    pub fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;

    pub fn GetClassNameW(hwnd: Hwnd, buffer: *mut u16, max_chars: i32) -> i32;
    pub fn GetWindowTextW(hwnd: Hwnd, buffer: *mut u16, max_chars: i32) -> i32;
    pub fn IsWindow(hwnd: Hwnd) -> Bool;

    pub fn SetWindowsHookExW(id: i32, proc_: HookProc, module: HInstance, thread_id: u32) -> HHook;
    pub fn UnhookWindowsHookEx(hook: HHook) -> Bool;
    pub fn CallNextHookEx(hook: HHook, code: i32, wparam: WParam, lparam: LParam) -> LResult;

    pub fn GetMessageW(msg: *mut Msg, hwnd: Hwnd, filter_min: u32, filter_max: u32) -> Bool;
    pub fn PeekMessageW(
        msg: *mut Msg,
        hwnd: Hwnd,
        filter_min: u32,
        filter_max: u32,
        remove: u32,
    ) -> Bool;
    pub fn PostThreadMessageW(thread_id: u32, msg: u32, wparam: WParam, lparam: LParam) -> Bool;

    pub fn WindowFromPoint(point: Point) -> Hwnd;
    pub fn GetAncestor(hwnd: Hwnd, flags: u32) -> Hwnd;
    pub fn ScreenToClient(hwnd: Hwnd, point: *mut Point) -> Bool;

    #[cfg(target_pointer_width = "64")]
    pub fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize;
    #[cfg(target_pointer_width = "64")]
    pub fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;

    // 32-bit Windows has no *Ptr variants; the headers map them onto these.
    #[cfg(target_pointer_width = "32")]
    pub fn GetWindowLongW(hwnd: Hwnd, index: i32) -> i32;
    #[cfg(target_pointer_width = "32")]
    pub fn SetWindowLongW(hwnd: Hwnd, index: i32, value: i32) -> i32;

    pub fn GetForegroundWindow() -> Hwnd;
    pub fn IsWindowVisible(hwnd: Hwnd) -> Bool;
    pub fn IsIconic(hwnd: Hwnd) -> Bool;
    pub fn MonitorFromWindow(hwnd: Hwnd, flags: u32) -> HMonitor;
    pub fn MonitorFromPoint(point: Point, flags: u32) -> HMonitor;
    pub fn GetMonitorInfoW(monitor: HMonitor, info: *mut MonitorInfo) -> Bool;
}

#[link(name = "shell32")]
unsafe extern "system" {
    pub fn SHQueryUserNotificationState(state: *mut i32) -> i32;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    pub fn DwmGetWindowAttribute(hwnd: Hwnd, attribute: u32, value: *mut u32, size: u32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetLastError() -> u32;
    pub fn SetLastError(code: u32);
    pub fn GetCurrentThreadId() -> u32;
    pub fn GetModuleHandleW(name: *const u16) -> HInstance;
    pub fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> Bool;
}

// Keep the Vec alive while its pointer is used: `wide(s).as_ptr()` inline dangles.
pub fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn from_wide(buffer: &[u16]) -> String {
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
