//! Raw Win32 bindings.
//!
//! This is the ONLY file in the project that talks to the operating system's C
//! API. Everything above it deals in plain Rust types. That boundary is what
//! makes the eventual Linux/macOS port cheap.
//!
//! ## Crash course in the Win32 types used here
//!
//! * `HWND` — a "handle to a window". It is an opaque pointer-sized value that
//!   the window manager hands you; you never dereference it. Every UI object in
//!   Windows is a window, including things that do not look like one (the
//!   desktop background, the icon grid, a button).
//! * `WPARAM` / `LPARAM` — two general-purpose message arguments. Historically
//!   "word" and "long" parameters; today both are pointer-sized. We pass them as
//!   `usize` / `isize`.
//! * `LRESULT` — a pointer-sized message return value. Passed as `isize`.
//! * `BOOL` — a C `int`, not a Rust `bool`. Zero is false, non-zero is true.
//!   Declaring it as `bool` would be undefined behaviour, so we use `i32`.
//! * Wide strings — the `...W` suffix on a function name means UTF-16. Windows
//!   is natively UTF-16; the `...A` variants are legacy 8-bit and should be
//!   avoided. `wide()` below does the conversion.
//!
//! ## Safety
//!
//! Every function here is `unsafe` to call because the compiler cannot verify
//! that we are passing valid handles or correctly sized buffers. The wrappers in
//! `windows.rs` are the safe layer.
//!
//! ## Platform assumptions
//!
//! 64-bit Windows only. `WS_POPUP` below is written as a positive `isize`, which
//! relies on `isize` being 64-bit wide. On a 32-bit target it would need to be a
//! `u32` cast. Nobody ships 32-bit desktop wallpapers in 2026, but if you ever
//! do, that constant is the one that breaks.

#![allow(non_snake_case)]
#![cfg(target_os = "windows")]

use core::ffi::c_void;
use core::ptr::{null, null_mut};

pub type Hwnd = *mut c_void;
pub const NULL_HWND: Hwnd = null_mut();

// ---------------------------------------------------------------------------
// Constants
//
// These are #defines in the Windows SDK headers. They are stable ABI: they have
// not changed since the 1990s and cannot change without breaking every program
// ever compiled for Windows.
// ---------------------------------------------------------------------------

/// The undocumented message that asks Explorer to create the wallpaper-level
/// `WorkerW` window. There is no SDK name for it; every live-wallpaper project
/// in existence hard-codes `0x052C`.
///
/// It is undocumented, not private: Explorer sends it to itself when the user
/// changes the wallpaper. Microsoft has never broken it, but it is also never
/// promised, which is the single biggest long-term risk in this project.
pub const WM_SPAWN_WORKER: u32 = 0x052C;

/// `SendMessageTimeout` flag: deliver normally, but give up after the timeout
/// rather than hanging forever if Explorer is wedged. Never use plain
/// `SendMessage` against another process — a hung Explorer would hang us too.
pub const SMTO_NORMAL: u32 = 0x0000;
/// Return immediately if the receiving thread appears to be hung, instead of
/// waiting out the full timeout. Always worth OR-ing in when messaging Explorer.
pub const SMTO_ABORTIFHUNG: u32 = 0x0002;

/// Indices for `Get/SetWindowLongPtrW`. Negative because the API originally
/// indexed into per-window extra bytes, and the built-in fields were given
/// negative indices to distinguish them.
pub const GWL_STYLE: i32 = -16;
pub const GWL_EXSTYLE: i32 = -20;

// Window styles (the GWL_STYLE bitfield).
pub const WS_CHILD: isize = 0x4000_0000;
pub const WS_POPUP: isize = 0x8000_0000;

// Extended window styles (the GWL_EXSTYLE bitfield).
/// Clicking this window does not activate it (does not steal focus).
pub const WS_EX_NOACTIVATE: isize = 0x0800_0000;
/// Keeps the window out of the taskbar and out of Alt-Tab.
pub const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
/// The opposite of the above; Bevy's window has it, so we clear it.
pub const WS_EX_APPWINDOW: isize = 0x0004_0000;

/// Special `hWndInsertAfter` value meaning "put this at the BOTTOM of the
/// sibling z-order". This is load-bearing: it is what guarantees we end up
/// *under* the icon layer rather than on top of it.
pub const HWND_BOTTOM: Hwnd = 1 as Hwnd;
/// The opposite: top of the sibling z-order. Useful as a diagnostic — if the
/// scene appears here but not at HWND_BOTTOM, rendering works and the problem is
/// that something above us is opaque.
pub const HWND_TOP: Hwnd = 0 as Hwnd;

// SetWindowPos flags.
pub const SWP_NOACTIVATE: u32 = 0x0010;
/// Leave the z-order alone (ignore the `insert_after` argument).
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_SHOWWINDOW: u32 = 0x0040;
/// Tells the window manager to recalculate the frame; required for style
/// changes made via SetWindowLongPtr to actually take effect.
pub const SWP_FRAMECHANGED: u32 = 0x0020;

// ShowWindow commands.
/// Show the window without activating it.
pub const SW_SHOWNA: i32 = 8;

// GetSystemMetrics indices.
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const SM_XVIRTUALSCREEN: i32 = 76;
pub const SM_YVIRTUALSCREEN: i32 = 77;
pub const SM_CXVIRTUALSCREEN: i32 = 78;
pub const SM_CYVIRTUALSCREEN: i32 = 79;

// Virtual key codes, for the quit hotkey.
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
/// "MENU" means Alt. Another 1990s naming artefact.
pub const VK_MENU: i32 = 0x12;
pub const VK_Q: i32 = 0x51;

// ---------------------------------------------------------------------------
// Function declarations
//
// `unsafe extern "system"` is required by Rust edition 2024 (previously a bare
// `extern "system"` block). "system" is the calling convention: on x86-64
// Windows it is identical to "C", but on 32-bit x86 it means stdcall, so using
// "system" keeps the code portable across Windows architectures.
// ---------------------------------------------------------------------------

#[link(name = "user32")]
unsafe extern "system" {
    /// Find a top-level window by class name and/or title. Pass null for
    /// "don't care". Returns NULL_HWND if nothing matches.
    pub fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Hwnd;

    /// The flexible version of the above.
    /// * `parent` non-null: search that window's children.
    /// * `parent` null, `child_after` non-null: search TOP-LEVEL windows,
    ///   starting after `child_after` in z-order. This second mode is the trick
    ///   we use to find the WorkerW that sits behind the icon layer.
    pub fn FindWindowExW(
        parent: Hwnd,
        child_after: Hwnd,
        class_name: *const u16,
        window_name: *const u16,
    ) -> Hwnd;

    /// Send a message to a window and block until it is handled or the timeout
    /// expires. Cross-process safe, unlike `SendMessage`.
    pub fn SendMessageTimeoutW(
        hwnd: Hwnd,
        msg: u32,
        wparam: usize,
        lparam: isize,
        flags: u32,
        timeout_ms: u32,
        result: *mut usize,
    ) -> isize;

    /// Call `callback` once per top-level window. Return 0 from the callback to
    /// stop early, non-zero to continue. `lparam` is an arbitrary value passed
    /// through to the callback — the standard way to smuggle a pointer to your
    /// own state into a C callback.
    pub fn EnumWindows(
        callback: unsafe extern "system" fn(Hwnd, isize) -> i32,
        lparam: isize,
    ) -> i32;

    /// Reparent a window. Returns the previous parent, or NULL_HWND on failure.
    pub fn SetParent(child: Hwnd, new_parent: Hwnd) -> Hwnd;

    /// Returns the parent (or owner) of a window, NULL_HWND for an unowned
    /// top-level window. We use this to detect that Explorer has orphaned us.
    pub fn GetParent(hwnd: Hwnd) -> Hwnd;

    pub fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize;
    pub fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, new_value: isize) -> isize;

    /// Move, resize, and/or restack a window in one atomic operation.
    pub fn SetWindowPos(
        hwnd: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;

    pub fn ShowWindow(hwnd: Hwnd, cmd: i32) -> i32;
    pub fn IsWindowVisible(hwnd: Hwnd) -> i32;

    /// Writes the owning process id through the out-pointer and returns the
    /// owning thread id. We only want the process id.
    pub fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut u32) -> u32;

    /// Screen and virtual-desktop dimensions. Returns PHYSICAL pixels as long as
    /// the process is DPI-aware — see the note in windows.rs.
    pub fn GetSystemMetrics(index: i32) -> i32;

    /// Returns the current state of a key. The high bit (0x8000) means "down
    /// right now"; the low bit means "was pressed since the last call", which we
    /// deliberately ignore because it is stateful and shared.
    ///
    /// This reads the GLOBAL keyboard state — it reports keys pressed in other
    /// applications too. That is exactly why we require a four-key combination
    /// and will need proper input gating in the next milestone.
    pub fn GetAsyncKeyState(vkey: i32) -> i16;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetCurrentProcessId() -> u32;
}

// ---------------------------------------------------------------------------
// Small safe helpers
// ---------------------------------------------------------------------------

/// Convert a Rust `&str` into a NUL-terminated UTF-16 buffer for the `...W` APIs.
///
/// CAVEAT: the returned `Vec` owns the buffer. You must keep it alive for the
/// duration of the call:
///
/// ```ignore
/// let class = wide("Progman");            // bind it
/// unsafe { FindWindowW(class.as_ptr(), null()) }
/// ```
///
/// Writing `FindWindowW(wide("Progman").as_ptr(), ...)` inline also happens to be
/// sound (Rust keeps temporaries alive to the end of the enclosing statement),
/// but binding it first makes the lifetime obvious to a reader.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}

/// Read a window's class name. Used only for diagnostics.
pub fn class_name_of(hwnd: Hwnd) -> String {
    // GetClassNameW is declared inline here because it is the only place we need
    // it and it keeps the diagnostic code self-contained.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetClassNameW(hwnd: Hwnd, buf: *mut u16, max_chars: i32) -> i32;
    }
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

/// True while the key is physically held down.
pub fn key_is_down(vkey: i32) -> bool {
    (unsafe { GetAsyncKeyState(vkey) } as u16 & 0x8000) != 0
}

/// Convenience so callers do not need to import `core::ptr::null`.
pub fn null_wstr() -> *const u16 {
    null()
}

// ---------------------------------------------------------------------------
// Hierarchy walking (used by the inspector)
// ---------------------------------------------------------------------------

/// `GetWindow` commands. We only need two.
///
/// Walking with these rather than with `EnumChildWindows` is deliberate:
/// `EnumChildWindows` flattens the whole subtree (it recurses into
/// grandchildren for you), which loses the depth information we care about.
/// `GW_CHILD` + `GW_HWNDNEXT` walks one level at a time, **in z-order**, which
/// is precisely the thing we are trying to understand.
pub const GW_HWNDNEXT: u32 = 2;
pub const GW_CHILD: u32 = 5;

/// Win32 `RECT`. Note that it stores edges, not a position and size, and that
/// `right`/`bottom` are exclusive.
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

#[link(name = "user32")]
unsafe extern "system" {
    /// The root of the window tree. Its children are the top-level windows.
    pub fn GetDesktopWindow() -> Hwnd;

    /// Navigate the window tree. See `GW_*` above.
    pub fn GetWindow(hwnd: Hwnd, cmd: u32) -> Hwnd;

    /// Window bounds in screen coordinates, including any frame.
    pub fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
}

pub fn window_rect(hwnd: Hwnd) -> Rect {
    let mut r = Rect::default();
    unsafe { GetWindowRect(hwnd, &mut r) };
    r
}

#[link(name = "user32")]
unsafe extern "system" {
    /// Copies a window's title text into the buffer. Returns the number of
    /// characters copied, excluding the terminating NUL.
    ///
    /// DANGER: when the target window belongs to our own process, this sends a
    /// synchronous WM_GETTEXT to the window's **owning thread**. Same process is
    /// not the same as same thread. Calling this from a Bevy worker thread on a
    /// window owned by the main thread deadlocks. Only call it from a system
    /// pinned to the main thread.
    pub fn GetWindowTextW(hwnd: Hwnd, buf: *mut u16, max_chars: i32) -> i32;
}

/// Read a window's title. Empty string if it has none.
pub fn window_title_of(hwnd: Hwnd) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}
