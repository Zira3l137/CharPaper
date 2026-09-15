//! Windows wallpaper-layer backend.
//!
//! ## The desktop window hierarchy, and why this works
//!
//! Explorer builds the desktop out of ordinary windows:
//!
//! ```text
//! Progman                          "Program Manager", the desktop root
//!   └─ WorkerW                     (layout varies by Windows version)
//!        └─ SHELLDLL_DefView       the shell view host
//!             └─ SysListView32     the actual icon grid
//! ```
//!
//! Since Windows 8, sending `Progman` the undocumented message `0x052C` makes
//! Explorer create a SECOND `WorkerW` that sits behind the icon-hosting one.
//! That second window is the wallpaper layer. Reparent our window into it and we
//! render above the static wallpaper and below the icons.
//!
//! Where the second `WorkerW` ends up differs between Windows builds, so we try
//! three strategies in order. All three end with the same safety net:
//! `SetWindowPos(..., HWND_BOTTOM, ...)`, which puts us at the bottom of the
//! sibling z-order. Even if we guess the wrong host window, being at the bottom
//! means we go *under* the icons rather than over them.
//!
//! ## Threading note
//!
//! Bevy may run these systems on a worker thread. The Win32 functions used here
//! (`SetParent`, `SetWindowPos`, `SetWindowLongPtr`, `GetAsyncKeyState`) are all
//! safe to call cross-thread — the window manager marshals them to the owning
//! thread. The ones that are NOT (`DestroyWindow`, anything involving a message
//! pump) we never call; winit owns those. If you ever add such a call, put it in
//! a system with a `NonSend` parameter, which Bevy guarantees to run on the main
//! thread.

use super::win32_sys::*;
use super::{Coverage, DesktopBackend, DesktopLayerConfig, ZOrder};
use bevy::prelude::*;
use core::ptr::null_mut;

pub struct WindowsWallpaperBackend {
    /// Our own top-level window, as an `isize` so the struct stays `Send + Sync`.
    /// Zero means "not found yet".
    own: isize,
    /// The wallpaper-layer host we attached to. Zero means "not attached".
    host: isize,
    /// Set once we have logged the dry-run report, to avoid repeating it.
    dry_run_reported: bool,
    /// Set once we have dumped the window hierarchy, to avoid repeating it.
    inspected: bool,
    /// How many times `attach` has been called. Used to log the attach sequence
    /// step by step for the first couple of attempts and then go quiet — so that
    /// if a Win32 call ever blocks, the last line in the log names it.
    attempts: u32,
}

impl WindowsWallpaperBackend {
    pub fn new() -> Self {
        Self {
            own: 0,
            host: 0,
            dry_run_reported: false,
            inspected: false,
            attempts: 0,
        }
    }
}

impl DesktopBackend for WindowsWallpaperBackend {
    fn name(&self) -> &'static str {
        "windows (Progman/WorkerW reparenting)"
    }

    fn attach(&mut self, cfg: &DesktopLayerConfig) -> Result<String, String> {
        self.attempts += 1;
        // Log each step for the first couple of attempts. Every Win32 call below
        // that can block is preceded by a line naming it, so a hang is always
        // localised to the last line printed.
        let step = self.attempts <= 2;
        macro_rules! step {
            ($($arg:tt)*) => { if step { info!($($arg)*); } };
        }

        step!("attach step 1: locating our own window");

        // 1. Find our own window. We ask the OS rather than asking Bevy.
        //
        //    Bevy does expose the raw handle via `RawHandleWrapper`, but that
        //    type's API has changed several times across releases (public
        //    fields, then a `get_handle()` accessor, then variations on the
        //    unsafety). Enumerating our own process's windows is a handful of
        //    lines, never breaks, and does not tie us to Bevy internals. Since we
        //    only ever create one visible window, it is unambiguous.
        let own = match find_own_top_level_window(&cfg.window_title) {
            Some(h) => h,
            None => return Err("could not find our own top-level window yet".into()),
        };
        self.own = own as isize;

        // 1b. Optional: dump the whole desktop window tree before we touch
        //     anything. This is the tool to reach for when the strategy ladder
        //     lands somewhere unexpected — it tells you where the icon layer
        //     actually is, which is what decides whether HWND_BOTTOM is enough.
        if cfg.inspect && !self.inspected {
            self.inspected = true;
            super::windows_inspect::dump_desktop_hierarchy();
        }

        // 2. Find the wallpaper-layer host window.
        step!("attach step 2: locating the wallpaper-layer host");
        let (host, strategy) = find_wallpaper_host(cfg.verbose)
            .ok_or_else(|| "no Progman window found; is Explorer running?".to_string())?;

        if cfg.verbose {
            info!(
                "desktop layer: own window {:?} ({}), host {:?} ({}) via {}",
                own,
                class_name_of(own),
                host,
                class_name_of(host),
                strategy
            );
        }

        let (x, y, w, h) = target_rect(cfg.coverage);

        // 3. Dry run: report and stop before mutating anything.
        if cfg.dry_run {
            if !self.dry_run_reported {
                self.dry_run_reported = true;
                info!("---- DRY RUN: no changes made ----");
                info!("  would SetParent into {:?} ({})", host, class_name_of(host));
                info!("  strategy: {strategy}");
                info!("  would position at x={x} y={y} w={w} h={h} (host-relative, physical px)");
                info!("  would clear WS_POPUP / WS_EX_APPWINDOW, set WS_CHILD / WS_EX_NOACTIVATE / WS_EX_TOOLWINDOW");
                info!("  would restack at HWND_BOTTOM");
                info!("----------------------------------");
            }
            return Ok(format!("dry run only, host would be {strategy}"));
        }

        step!("attach step 4: SetParent into {host:?}");
        // 4. Reparent. This alone is enough to change our z-order position; the
        //    style fixes below are about behaviour, not layering.
        let previous_parent = unsafe { SetParent(own, host) };
        if previous_parent.is_null() {
            // SetParent returns NULL on failure, but ALSO returns NULL on
            // success when the window previously had no parent — which is our
            // case. So NULL is ambiguous here and we must verify separately
            // rather than treat it as an error. This is a classic Win32 trap.
            if unsafe { GetParent(own) } != host {
                return Err("SetParent failed (window not reparented)".into());
            }
        }
        self.host = host as isize;

        step!("attach step 5: adjusting window styles");
        // 5. Fix up window styles.
        //
        //    A window that has been reparented should be a WS_CHILD, not a
        //    WS_POPUP. Windows tolerates the mismatch but behaviour gets strange
        //    around clipping and activation, so make it explicit.
        unsafe {
            let style = GetWindowLongPtrW(own, GWL_STYLE);
            SetWindowLongPtrW(own, GWL_STYLE, (style & !WS_POPUP) | WS_CHILD);

            let ex = GetWindowLongPtrW(own, GWL_EXSTYLE);
            // NOACTIVATE: never steal focus. TOOLWINDOW: stay out of Alt-Tab and
            // the taskbar. Clearing APPWINDOW is needed for TOOLWINDOW to stick.
            let ex_new = (ex & !WS_EX_APPWINDOW) | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;
            SetWindowLongPtrW(own, GWL_EXSTYLE, ex_new);
        }

        // 6. Position, size, and restack.
        //
        //    SWP_FRAMECHANGED makes the style changes above take effect.
        //    SWP_NOACTIVATE keeps focus where it was.
        //    HWND_BOTTOM is the guarantee that we land below SHELLDLL_DefView;
        //    HWND_TOP is the diagnostic mode that covers the icons instead.
        let (insert_after, z_name) = match cfg.z_order {
            ZOrder::BelowIcons => (HWND_BOTTOM, "HWND_BOTTOM"),
            ZOrder::AboveIcons => (HWND_TOP, "HWND_TOP (diagnostic)"),
        };

        // The one-pixel-short first call is deliberate when nudging: it
        // guarantees the follow-up call is a genuine size *change*, so winit
        // emits a resize and Bevy reconfigures the render surface against the
        // window in its new parent. SetWindowPos ignores a no-op resize.
        let first_h = if cfg.nudge_after_attach { h - 1 } else { h };

        step!("attach step 6: SetWindowPos to {w}x{first_h} at ({x}, {y}), {z_name}");
        let ok = unsafe {
            SetWindowPos(
                own,
                insert_after,
                x,
                y,
                w,
                first_h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            )
        };
        if ok == 0 {
            return Err("SetWindowPos failed".into());
        }

        if cfg.nudge_after_attach {
            // Second call restores the real height. SWP_NOZORDER preserves the
            // stacking we just established.
            step!("attach step 6b: nudging size to {w}x{h} to force surface reconfigure");
            unsafe {
                SetWindowPos(
                    own,
                    NULL_HWND,
                    x,
                    y,
                    w,
                    h,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            };
        }

        // SW_SHOWNA = show without activating. Belt and braces; the window
        // should already be visible.
        step!("attach step 7: ShowWindow(SW_SHOWNA)");
        unsafe { ShowWindow(own, SW_SHOWNA) };

        // Bevy does not need telling about the new size: SetWindowPos generates
        // a WM_SIZE message, winit turns that into a resize event, and Bevy
        // resizes its render surface to match. That is why the cube is not
        // stretched even though we changed the window behind Bevy's back.

        Ok(format!("{strategy}, {w}x{h} at ({x}, {y}), {z_name}"))
    }

    fn is_attached(&self) -> bool {
        if self.own == 0 || self.host == 0 {
            return false;
        }
        let own = self.own as Hwnd;
        let host = self.host as Hwnd;
        unsafe { GetParent(own) == host }
    }

    fn diagnostics(&self) -> String {
        if self.own == 0 {
            return "win32: no window yet".to_string();
        }
        let own = self.own as Hwnd;
        let r = window_rect(own);
        let style = unsafe { GetWindowLongPtrW(own, GWL_STYLE) };
        let parent = unsafe { GetParent(own) };
        format!(
            "win32: {own:?} {}x{} at ({},{}) parent={parent:?} visible={} child={} popup={}",
            r.width(),
            r.height(),
            r.left,
            r.top,
            unsafe { IsWindowVisible(own) } != 0,
            style & WS_CHILD != 0,
            style & WS_POPUP != 0,
        )
    }

    fn quit_requested(&self, _cfg: &DesktopLayerConfig) -> bool {
        // Ctrl + Alt + Shift + Q. Four keys because GetAsyncKeyState is global:
        // it sees keys you press in any application, so a simpler combination
        // would fire while you were typing in a text editor.
        //
        // This is polling, sampled once per frame, so a very fast press can be
        // missed. Hold it for a moment. Proper input handling (raw input with
        // RIDEV_INPUTSINK, plus gating on what is under the cursor) is the next
        // milestone; this is just a panic button so you are never forced to open
        // Task Manager.
        key_is_down(VK_CONTROL)
            && key_is_down(VK_MENU)
            && key_is_down(VK_SHIFT)
            && key_is_down(VK_Q)
    }
}

// ---------------------------------------------------------------------------
// Finding our own window
// ---------------------------------------------------------------------------

/// State for the `EnumWindows` callback below.
///
/// Holding an owned `String` rather than a `&str` keeps the struct free of
/// lifetime parameters, which matters because we round-trip a pointer to it
/// through an `isize` and any lifetime would be a fiction at that point.
struct OwnWindowSearch {
    pid: u32,
    title: String,
    /// A window of ours whose title matches exactly. This is the one we want.
    exact: Hwnd,
    /// Largest visible top-level window of ours, as a fallback if the title
    /// does not match (for instance if you change the title at runtime).
    fallback: Hwnd,
    fallback_area: i64,
}

unsafe extern "system" fn own_window_proc(hwnd: Hwnd, lparam: isize) -> i32 {
    // Reconstitute the pointer we smuggled through lparam. This is the standard
    // pattern for C callbacks: there is no closure capture, so you pass a raw
    // pointer to your state as an integer.
    let search = unsafe { &mut *(lparam as *mut OwnWindowSearch) };

    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid != search.pid {
        return 1; // keep going
    }
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    // A window we have already reparented has a parent, so skip those: we want
    // the top-level one. On a re-attach after being orphaned this is still true,
    // because orphaning means Explorer destroyed our parent and Windows
    // promoted us back to top level.
    if !unsafe { GetParent(hwnd) }.is_null() {
        return 1;
    }

    // Cheap geometric prefilter, applied BEFORE reading the title.
    //
    // `GetWindowTextW` sends WM_GETTEXT, which is a synchronous cross-thread
    // call. It is safe here only because this system is pinned to the main
    // thread, but there is no reason to make it on windows that obviously are
    // not our renderer: winit's `Winit Thread Event Target` is 16x16 and the IME
    // helper windows are 0x0.
    let r = window_rect(hwnd);
    if r.width() < 100 || r.height() < 100 {
        return 1;
    }

    // Identify by title, not by "it is the only visible one".
    //
    // winit creates several top-level windows per process. On this machine the
    // process owns an `MSCTFIME UI` window, an `IME` window, the real render
    // window (class `Window Class`), and a 16x16 VISIBLE window of class
    // `Winit Thread Event Target`. Picking the first visible one only worked
    // because the render window happened to sit in front of the event target in
    // z-order — a coin flip we should not be relying on. Reparenting the event
    // target instead would have produced an invisible cube and no error.
    if window_title_of(hwnd) == search.title {
        search.exact = hwnd;
        return 0; // stop, this is definitely it
    }

    let area = i64::from(r.width()) * i64::from(r.height());
    if area > search.fallback_area {
        search.fallback = hwnd;
        search.fallback_area = area;
    }
    1
}

fn find_own_top_level_window(title: &str) -> Option<Hwnd> {
    let mut search = OwnWindowSearch {
        pid: unsafe { GetCurrentProcessId() },
        title: title.to_string(),
        exact: null_mut(),
        fallback: null_mut(),
        fallback_area: 0,
    };
    unsafe {
        EnumWindows(
            own_window_proc,
            &mut search as *mut OwnWindowSearch as isize,
        )
    };

    if !search.exact.is_null() {
        return Some(search.exact);
    }
    if !search.fallback.is_null() {
        warn!(
            "desktop layer: no window titled {title:?}; falling back to our largest visible window {:?}",
            search.fallback
        );
        return Some(search.fallback);
    }
    None
}

// ---------------------------------------------------------------------------
// Finding the wallpaper-layer host
// ---------------------------------------------------------------------------

struct HostSearch {
    /// The top-level window that hosts `SHELLDLL_DefView`, i.e. the icon layer.
    /// Knowing this is useful even when there is no wallpaper `WorkerW`: it is
    /// the window we must end up beneath.
    defview_host: Hwnd,
    /// The top-level `WorkerW` immediately behind it, if Explorer made one.
    behind: Hwnd,
}

/// Is this top-level window one that could plausibly own the desktop?
///
/// This filter is essential, not cosmetic. `SHELLDLL_DefView` is the standard
/// shell view host, so **every open File Explorer window has one too** — on
/// Windows 11 hosted inside a window of class `ExplorerBrowserOwner`. Those
/// windows usually sit in front of `Progman` in z-order, so a search for "the
/// first top-level window with a SHELLDLL_DefView child" happily latches onto
/// whichever folder the user happens to have open. Only `Progman` and `WorkerW`
/// are desktop windows.
fn is_desktop_class(hwnd: Hwnd) -> bool {
    let class = class_name_of(hwnd);
    class == "Progman" || class == "WorkerW"
}

/// Could this window plausibly be the wallpaper layer?
///
/// Explorer keeps a pile of hidden 136x39 `WorkerW` stubs around for unrelated
/// purposes — this machine has eleven of them. Attaching to one would put the
/// cube in a hidden 136x39 window and report success, which is the worst kind
/// of failure. A real wallpaper host is visible and covers a monitor.
fn is_plausible_host(hwnd: Hwnd) -> bool {
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return false;
    }
    let r = window_rect(hwnd);
    r.width() >= 640 && r.height() >= 480
}

unsafe extern "system" fn wallpaper_host_proc(hwnd: Hwnd, lparam: isize) -> i32 {
    let search = unsafe { &mut *(lparam as *mut HostSearch) };

    if !is_desktop_class(hwnd) {
        return 1;
    }

    // Does this top-level window host the icon view?
    let defview_class = wide("SHELLDLL_DefView");
    let defview = unsafe { FindWindowExW(hwnd, NULL_HWND, defview_class.as_ptr(), null_wstr()) };
    if defview.is_null() {
        return 1;
    }
    search.defview_host = hwnd;

    // It does. The wallpaper-layer WorkerW, if it exists, is the next top-level
    // WorkerW after this one in z-order. Note the argument shape:
    // parent = NULL means "search top-level windows", child_after = hwnd means
    // "start after this one".
    let worker_class = wide("WorkerW");
    search.behind = unsafe { FindWindowExW(NULL_HWND, hwnd, worker_class.as_ptr(), null_wstr()) };

    0 // we have what we came for either way, stop enumerating
}

/// Locate the window to reparent into, trying progressively less ideal
/// strategies. Returns the handle and a description of which strategy worked —
/// worth logging, because it tells you what the current Windows build is doing.
fn find_wallpaper_host(verbose: bool) -> Option<(Hwnd, &'static str)> {
    let progman_class = wide("Progman");
    let progman = unsafe { FindWindowW(progman_class.as_ptr(), null_wstr()) };
    if progman.is_null() {
        return None;
    }

    // Ask Explorer to create the wallpaper-layer WorkerW.
    //
    // There is more than one folklore incantation for this and which one works
    // varies by Windows build, so we send all of them. An unrecognised message
    // is simply ignored by the receiving window procedure, so this is safe —
    // just inelegant. Repetition is also not superstition: on Windows 11 24H2
    // and later the window frequently does not appear on the first request
    // shortly after login, which is what the retry loop in
    // `maintain_attachment` is for.
    //
    // (0, 0) is the classic form, documented nowhere and used by everyone.
    // (0xD, 0x1) is the form several shipping wallpaper apps use for Windows 8+.
    // Nobody outside Microsoft knows what the parameters mean.
    const VARIANTS: [(usize, isize); 3] = [(0, 0), (0xD, 0x1), (0xD, 0x0)];
    let mut result: usize = 0;
    for (wparam, lparam) in VARIANTS {
        unsafe {
            SendMessageTimeoutW(
                progman,
                WM_SPAWN_WORKER,
                wparam,
                lparam,
                SMTO_NORMAL | SMTO_ABORTIFHUNG,
                1000,
                &mut result,
            )
        };
    }

    // Find the icon layer's host and, if it exists, the WorkerW behind it.
    let mut search = HostSearch {
        defview_host: null_mut(),
        behind: null_mut(),
    };
    unsafe { EnumWindows(wallpaper_host_proc, &mut search as *mut HostSearch as isize) };

    // Strategy A — the classic Windows 8/10/11 layout: a top-level WorkerW
    // immediately behind the top-level WorkerW that hosts SHELLDLL_DefView.
    if !search.behind.is_null() && is_plausible_host(search.behind) {
        return Some((search.behind, "strategy A: top-level WorkerW behind the icon host"));
    }

    // No dedicated wallpaper window exists. If we at least know which top-level
    // window hosts the icon layer, we can attach to *that* and rely on
    // HWND_BOTTOM to sit beneath the icons. Less tidy than strategy A — we share
    // a window with the icon view rather than having our own layer — but it puts
    // us in the right place in the z-order, which is what actually matters.
    if !search.defview_host.is_null() {
        let host_class = class_name_of(search.defview_host);
        if verbose {
            info!(
                "desktop layer: no WorkerW behind the icon layer; icon layer is hosted by {:?} ({host_class})",
                search.defview_host
            );
        }
        if host_class == "WorkerW" {
            return Some((
                search.defview_host,
                "strategy A2: the icon-hosting WorkerW itself, beneath the icon view",
            ));
        }
        if host_class == "Progman" {
            return Some((
                search.defview_host,
                "strategy A3: Progman, which hosts the icon view directly",
            ));
        }
    }

    // Strategy B — seen on Windows 11 24H2 and some Insider builds: the WorkerW
    // windows are children of Progman rather than top-level siblings. Prefer a
    // child that does NOT contain SHELLDLL_DefView, because the one that does is
    // the icon layer itself.
    let worker_class = wide("WorkerW");
    let defview_class = wide("SHELLDLL_DefView");
    let mut child = unsafe { FindWindowExW(progman, NULL_HWND, worker_class.as_ptr(), null_wstr()) };
    let mut first_child: Hwnd = null_mut();
    while !child.is_null() {
        if first_child.is_null() {
            first_child = child;
        }
        let has_defview =
            !unsafe { FindWindowExW(child, NULL_HWND, defview_class.as_ptr(), null_wstr()) }
                .is_null();
        if !has_defview && is_plausible_host(child) {
            return Some((child, "strategy B: WorkerW child of Progman, no icon view"));
        }
        child = unsafe { FindWindowExW(progman, child, worker_class.as_ptr(), null_wstr()) };
    }
    if !first_child.is_null() {
        if verbose {
            warn!(
                "desktop layer: only found a WorkerW that hosts the icon view; \
                 relying on HWND_BOTTOM to stay beneath it"
            );
        }
        return Some((first_child, "strategy B': WorkerW child of Progman (icon host)"));
    }

    // Strategy C — parent directly to Progman. Works on several builds and is a
    // reasonable last resort, since HWND_BOTTOM still keeps us behind the icons.
    if verbose {
        warn!("desktop layer: no WorkerW found at all, falling back to Progman");
    }
    Some((progman, "strategy C: Progman directly (fallback)"))
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Work out where to put our window, in coordinates relative to the host.
///
/// ## The coordinate trap
///
/// The wallpaper `WorkerW` spans the entire virtual desktop — the bounding box
/// of all monitors. Its client origin `(0, 0)` corresponds to the virtual
/// desktop's top-left corner, which in *screen* coordinates is
/// `(SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN)`. Those values are NEGATIVE whenever
/// a secondary monitor sits above or to the left of the primary one, because the
/// primary monitor's top-left is the origin of screen coordinates by definition.
///
/// So: covering the whole virtual desktop means `(0, 0, cx, cy)`. Covering just
/// the primary monitor means offsetting by the negated virtual origin.
///
/// ## DPI
///
/// These are physical pixels. winit already sets the process to per-monitor
/// DPI-aware v2 during startup, so Windows does not lie to us about sizes and no
/// application manifest is needed. If you ever bypass winit's window creation,
/// you must set DPI awareness yourself or every measurement here becomes a
/// scaled approximation.
fn target_rect(coverage: Coverage) -> (i32, i32, i32, i32) {
    unsafe {
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        match coverage {
            Coverage::VirtualDesktop => (
                0,
                0,
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            ),
            Coverage::PrimaryMonitor => (
                -vx,
                -vy,
                GetSystemMetrics(SM_CXSCREEN),
                GetSystemMetrics(SM_CYSCREEN),
            ),
        }
    }
}
