//! Raw Win32 declarations. No logic lives here -- only the C ABI.
//!
//! A short orientation for anyone who hasn't touched the Windows API before:
//!
//! * `HWND` is a *window handle*: an opaque, pointer-sized number the OS gives
//!   you to refer to a window. Every window in the whole system has one,
//!   including other applications' windows and the desktop itself. `0` means
//!   "no window" and doubles as the error return for most functions.
//! * Functions ending in `W` take UTF-16 ("wide") strings. There are matching
//!   `A` functions taking 8-bit strings; always use `W`.
//! * `extern "system"` is the calling convention Win32 uses (`stdcall` on
//!   32-bit, same as `C` on 64-bit). Getting this wrong corrupts the stack, so
//!   never write `extern "C"` here even though it works on x86_64.
//! * Almost nothing returns a proper error. The convention is: a zero return
//!   *may* mean failure, and you then call `GetLastError` to find out. Several
//!   functions legitimately return zero on success, which is why
//!   `set_window_long_ptr` below clears the error code first.

#![allow(non_snake_case)]
// Some declarations are kept for the next milestone before anything calls them.

use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;

// --- basic types -----------------------------------------------------------

/// A window handle. Pointer-sized; `isize` matches what `raw-window-handle`
/// gives us, so we avoid casting back and forth.
pub type Hwnd = isize;
pub type Bool = i32;
pub type WParam = usize;
pub type LParam = isize;
pub type LResult = isize;
/// A hook handle, as returned by `SetWindowsHookExW`.
pub type HHook = isize;
/// A loaded module (the .exe or a .dll). Same thing as an `HMODULE`.
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

/// One message pulled off a thread's message queue.
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

/// What a low-level mouse hook receives through its `lparam`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MsLlHookStruct {
    /// Screen coordinates, always in physical pixels regardless of DPI
    /// awareness.
    pub pt: Point,
    /// Wheel delta or X button number in the high word, depending on the
    /// message. Unused for everything else.
    pub mouse_data: u32,
    pub flags: u32,
    pub time: u32,
    pub extra_info: usize,
}

pub type EnumWindowsProc = unsafe extern "system" fn(Hwnd, LParam) -> Bool;

/// `wparam` is the mouse message (`WM_MOUSEMOVE`, ...), `lparam` points at an
/// [`MsLlHookStruct`].
pub type HookProc = unsafe extern "system" fn(i32, WParam, LParam) -> LResult;

// --- constants -------------------------------------------------------------

/// Index for `GetWindowLongPtr` selecting the normal window style bits.
pub const GWL_STYLE: i32 = -16;
/// Index selecting the *extended* window style bits.
pub const GWL_EXSTYLE: i32 = -20;

/// This window is a child of another window (coordinates become relative to
/// the parent, and it can never be on top of the parent).
pub const WS_CHILD: isize = 0x4000_0000;

/// The window is composited by the desktop compositor rather than drawn
/// directly. Required for per-window alpha via `SetLayeredWindowAttributes`.
pub const WS_EX_LAYERED: isize = 0x0008_0000;

/// The window has no GDI redirection surface at all. Windows 11 sets this on
/// `Progman` when the desktop uses the newer "raised desktop" layout, which is
/// exactly how we detect that layout.
pub const WS_EX_NOREDIRECTIONBITMAP: isize = 0x0020_0000;

/// `SetLayeredWindowAttributes` flag: interpret the alpha argument.
pub const LWA_ALPHA: u32 = 0x0000_0002;

pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;

/// Special `hWndInsertAfter` value meaning "put this at the bottom of the
/// z-order".
pub const HWND_BOTTOM: Hwnd = 1;

/// `SendMessageTimeout` flag: behave like a normal `SendMessage`.
pub const SMTO_NORMAL: u32 = 0x0000;

/// The undocumented message that asks `Progman` to create the background
/// `WorkerW` window. Not in any header; discovered by reverse engineering and
/// used by every live-wallpaper program in existence.
pub const WM_SPAWN_WORKER_W: u32 = 0x052C;

/// `SetWindowsHookExW` id for the global low-level mouse hook.
pub const WH_MOUSE_LL: i32 = 14;
/// The only hook `code` that carries an event; anything else must just be
/// passed on.
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

/// One wheel notch. High-resolution wheels report fractions of it.
pub const WHEEL_DELTA: i16 = 120;

/// `GetAncestor` flag: walk parents all the way up to the top-level window.
pub const GA_ROOT: u32 = 2;

// --- imports ---------------------------------------------------------------

#[link(name = "user32")]
unsafe extern "system" {
    /// Find a top-level window by class name and/or title. Either may be null.
    pub fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Hwnd;

    /// Find a *child* of `parent` by class name. `child_after` lets you resume
    /// a search: pass the previous result to get the next match. Passing `0`
    /// for `parent` searches top-level windows, in which case `child_after`
    /// means "the window after this one in z-order".
    pub fn FindWindowExW(
        parent: Hwnd,
        child_after: Hwnd,
        class_name: *const u16,
        window_name: *const u16,
    ) -> Hwnd;

    /// Call `callback` once for every top-level window. `lparam` is passed
    /// straight through, which is how we smuggle a `&mut Vec` into it.
    pub fn EnumWindows(callback: EnumWindowsProc, lparam: LParam) -> Bool;

    /// Same, but for the children of one window.
    pub fn EnumChildWindows(parent: Hwnd, callback: EnumWindowsProc, lparam: LParam) -> Bool;

    /// The root of the whole window tree.
    pub fn GetDesktopWindow() -> Hwnd;

    /// Send a message and give up after `timeout_ms` if the target hangs. We
    /// use the timeout variant because `Progman` belongs to Explorer, and a
    /// plain `SendMessage` to a wedged Explorer would hang us forever.
    pub fn SendMessageTimeoutW(
        hwnd: Hwnd,
        msg: u32,
        wparam: WParam,
        lparam: LParam,
        flags: u32,
        timeout_ms: u32,
        result: *mut usize,
    ) -> LResult;

    /// Re-parent a window. Returns the previous parent, or `0` on failure.
    pub fn SetParent(child: Hwnd, new_parent: Hwnd) -> Hwnd;

    /// Move, resize and/or restack a window.
    pub fn SetWindowPos(
        hwnd: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> Bool;

    /// Per-window alpha. `alpha` is 0-255; 255 is fully opaque.
    pub fn SetLayeredWindowAttributes(hwnd: Hwnd, color_key: u32, alpha: u8, flags: u32) -> Bool;

    /// Window rectangle in *screen* coordinates.
    pub fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;

    pub fn GetClassNameW(hwnd: Hwnd, buffer: *mut u16, max_chars: i32) -> i32;
    pub fn GetWindowTextW(hwnd: Hwnd, buffer: *mut u16, max_chars: i32) -> i32;
    pub fn IsWindow(hwnd: Hwnd) -> Bool;

    /// Install a hook. With `thread_id == 0` it is global: every thread on
    /// this desktop feeds it. Low-level hooks are called back on the
    /// installing thread, which must keep pumping messages for that to happen.
    pub fn SetWindowsHookExW(id: i32, proc_: HookProc, module: HInstance, thread_id: u32) -> HHook;
    pub fn UnhookWindowsHookEx(hook: HHook) -> Bool;
    /// Hand the event to the next hook in the chain. `hook` is ignored.
    pub fn CallNextHookEx(hook: HHook, code: i32, wparam: WParam, lparam: LParam) -> LResult;

    /// Block until a message arrives. Returns `0` for `WM_QUIT` and `-1` on
    /// error, which is why the result is not a real boolean.
    pub fn GetMessageW(msg: *mut Msg, hwnd: Hwnd, filter_min: u32, filter_max: u32) -> Bool;
    pub fn PeekMessageW(
        msg: *mut Msg,
        hwnd: Hwnd,
        filter_min: u32,
        filter_max: u32,
        remove: u32,
    ) -> Bool;
    pub fn PostThreadMessageW(thread_id: u32, msg: u32, wparam: WParam, lparam: LParam) -> Bool;

    /// The deepest visible, enabled window under a screen point.
    pub fn WindowFromPoint(point: Point) -> Hwnd;
    pub fn GetAncestor(hwnd: Hwnd, flags: u32) -> Hwnd;
    /// Screen coordinates to `hwnd`'s client coordinates, in place.
    pub fn ScreenToClient(hwnd: Hwnd, point: *mut Point) -> Bool;

    #[cfg(target_pointer_width = "64")]
    pub fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize;
    #[cfg(target_pointer_width = "64")]
    pub fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;

    // 32-bit Windows has no `Ptr` variants; the headers `#define` them onto
    // the plain ones.
    #[cfg(target_pointer_width = "32")]
    pub fn GetWindowLongW(hwnd: Hwnd, index: i32) -> i32;
    #[cfg(target_pointer_width = "32")]
    pub fn SetWindowLongW(hwnd: Hwnd, index: i32, value: i32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetLastError() -> u32;
    pub fn SetLastError(code: u32);
    pub fn GetCurrentThreadId() -> u32;
    /// `null` returns the handle of the running .exe.
    pub fn GetModuleHandleW(name: *const u16) -> HInstance;
}

// --- small safe wrappers ---------------------------------------------------

/// Convert a Rust string to a null-terminated UTF-16 buffer.
///
/// Keep the returned `Vec` alive for as long as you use the pointer. Writing
/// `wide("Progman").as_ptr()` inline is a use-after-free: the temporary is
/// dropped at the end of the statement. Always bind it to a variable first.
pub fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    OsString::from_wide(&buffer[..end]).to_string_lossy().into_owned()
}

/// Read a window's style bits.
#[cfg(target_pointer_width = "64")]
pub fn get_window_long_ptr(hwnd: Hwnd, index: i32) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, index) }
}

/// Read a window's style bits. On 32-bit Windows there is no `Ptr` variant --
/// the headers `#define` it onto the plain one.
#[cfg(target_pointer_width = "32")]
pub fn get_window_long_ptr(hwnd: Hwnd, index: i32) -> isize {
    unsafe { GetWindowLongW(hwnd, index) as isize }
}

/// Write a window's style bits, returning the previous value.
///
/// Caveat: the API returns `0` both on failure *and* when the previous value
/// happened to be zero. The documented way to tell them apart is to clear the
/// thread's last-error code first, so that is what we do. Callers that care
/// should check `last_error()` after a `0` return.
#[cfg(target_pointer_width = "64")]
pub fn set_window_long_ptr(hwnd: Hwnd, index: i32, value: isize) -> isize {
    unsafe {
        SetLastError(0);
        SetWindowLongPtrW(hwnd, index, value)
    }
}

/// See the 64-bit version above.
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
    // 256 is the documented maximum length of a window class name.
    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    from_wide(&buffer[..len as usize])
}

pub fn window_title(hwnd: Hwnd) -> String {
    // Titles have no hard limit; anything longer than this is not useful to us.
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

/// `FindWindowW` with Rust strings.
pub fn find_window(class: &str) -> Hwnd {
    let class = wide(class);
    unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }
}

/// `FindWindowExW` with Rust strings. Pass `0` for `parent` or `after` to mean
/// "not specified".
pub fn find_window_ex(parent: Hwnd, after: Hwnd, class: &str) -> Hwnd {
    let class = wide(class);
    unsafe { FindWindowExW(parent, after, class.as_ptr(), std::ptr::null()) }
}

/// Collect handles into a `Vec` via the `lparam` back-channel.
///
/// # Safety
/// `lparam` must be a `*mut Vec<Hwnd>` that outlives the enumeration. The two
/// callers below satisfy this by construction.
unsafe extern "system" fn collect_callback(hwnd: Hwnd, lparam: LParam) -> Bool {
    let out = unsafe { &mut *(lparam as *mut Vec<Hwnd>) };
    out.push(hwnd);
    1 // non-zero == "keep going"
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
