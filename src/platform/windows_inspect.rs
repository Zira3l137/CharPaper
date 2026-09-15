//! Desktop window hierarchy inspector.
//!
//! A read-only diagnostic. It answers the one question that decides whether the
//! reparenting will work on a given machine:
//!
//! > **Where does `SHELLDLL_DefView` live, and what is its z-order relative to
//! > the window we are about to become a child of?**
//!
//! The icon layer (`SHELLDLL_DefView` and its `SysListView32` child) must end up
//! *above* us. Reparenting plus `HWND_BOTTOM` guarantees that only if we become
//! a **sibling or ancestor-sibling** of the icon layer. If the icon layer lives
//! in a completely separate top-level window that sits above ours, nothing we do
//! to our own z-order helps — we would be hidden entirely, cube and all.
//!
//! So: run this, find the `SHELLDLL_DefView` line, and look at what contains it.
//!
//! * Contained by the window we attach to, or by one of its ancestors → good.
//! * A separate top-level window listed *before* ours (further front) → we would
//!   be invisible, and the strategy ladder needs another rung.
//!
//! Top-level windows are listed front to back, which is the same order the
//! compositor draws them in reverse.

use super::win32_sys::*;
use bevy::prelude::*;

/// How deep to walk into each interesting top-level window. The desktop tree is
/// only three or four levels deep; the limit is just insurance against a
//  pathological subtree in some third-party shell replacement.
const MAX_DEPTH: usize = 5;

pub fn dump_desktop_hierarchy() {
    let own_pid = unsafe { GetCurrentProcessId() };

    info!("=============== desktop window hierarchy ===============");
    info!("top-level windows, front to back; only Progman, WorkerW and our own");

    let root = unsafe { GetDesktopWindow() };
    let mut hwnd = unsafe { GetWindow(root, GW_CHILD) };
    let mut z = 0usize;
    let mut omitted = 0usize;

    while !hwnd.is_null() {
        let class = class_name_of(hwnd);
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };

        let is_ours = pid == own_pid;
        if class == "Progman" || class == "WorkerW" || is_ours {
            let tag = if is_ours { "  <-- US" } else { "" };
            info!("[z={z}] {}{tag}", describe(hwnd, &class));
            dump_children(hwnd, 1);
        } else {
            omitted += 1;
        }

        z += 1;
        hwnd = unsafe { GetWindow(hwnd, GW_HWNDNEXT) };
    }

    info!("({omitted} unrelated top-level windows omitted)");
    report_icon_layer();
    info!("========================================================");
}

/// The verdict. Lists every top-level window hosting a `SHELLDLL_DefView` and
/// picks out the one that is actually the desktop.
///
/// There is usually more than one. `SHELLDLL_DefView` is the generic shell view
/// host, so every open File Explorer window has one, nested inside a window of
/// class `ExplorerBrowserOwner` on Windows 11. Those sit in front of `Progman`
/// in z-order, so "the first one found" is typically a folder the user left
/// open, not the desktop. Only `Progman` and `WorkerW` are desktop windows.
fn report_icon_layer() {
    let defview_class = wide("SHELLDLL_DefView");
    let root = unsafe { GetDesktopWindow() };
    let mut hwnd = unsafe { GetWindow(root, GW_CHILD) };
    let mut z = 0usize;
    let mut desktop_host: Option<(Hwnd, usize, String)> = None;
    let mut others = 0usize;

    while !hwnd.is_null() {
        let found = unsafe { FindWindowExW(hwnd, NULL_HWND, defview_class.as_ptr(), null_wstr()) };
        if !found.is_null() {
            let class = class_name_of(hwnd);
            let is_desktop = class == "Progman" || class == "WorkerW";
            info!(
                "  SHELLDLL_DefView found under {class} {hwnd:?} at z={z}{}",
                if is_desktop {
                    "   <-- desktop"
                } else {
                    "   (a shell browser window, not the desktop)"
                }
            );
            if is_desktop && desktop_host.is_none() {
                desktop_host = Some((hwnd, z, class));
            } else if !is_desktop {
                others += 1;
            }
        }
        z += 1;
        hwnd = unsafe { GetWindow(hwnd, GW_HWNDNEXT) };
    }

    if others > 0 {
        info!("  ({others} of those belong to open Explorer windows and are ignored)");
    }

    match desktop_host {
        Some((hwnd, z, class)) => {
            info!("VERDICT: the desktop icon layer is a direct child of {class} {hwnd:?} at z={z}");
            info!(
                "         attach to {class} and HWND_BOTTOM makes us a sibling below the icon view"
            );
        }
        None => {
            warn!("VERDICT: no SHELLDLL_DefView under any Progman or WorkerW window.");
            warn!("         Either desktop icons are disabled, or the shell is in an unusual state.");
            warn!("         If your desktop background is also solid black, suspect the Windows");
            warn!("         personalization-load bug rather than this code.");
        }
    }
}

fn dump_children(parent: Hwnd, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    let mut child = unsafe { GetWindow(parent, GW_CHILD) };
    while !child.is_null() {
        let class = class_name_of(child);
        let indent = "  ".repeat(depth);
        let note = match class.as_str() {
            "SHELLDLL_DefView" => "   <<< ICON LAYER HOST",
            "SysListView32" => "   <<< the icons themselves",
            _ => "",
        };
        info!("{indent}{}{note}", describe(child, &class));
        dump_children(child, depth + 1);
        child = unsafe { GetWindow(child, GW_HWNDNEXT) };
    }
}

fn describe(hwnd: Hwnd, class: &str) -> String {
    let r = window_rect(hwnd);
    let visible = unsafe { IsWindowVisible(hwnd) } != 0;
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
    let is_child = style & WS_CHILD != 0;
    format!(
        "{hwnd:?} class={class:<20} {}x{} at ({},{}) {}{}",
        r.width(),
        r.height(),
        r.left,
        r.top,
        if visible { "visible" } else { "hidden " },
        if is_child { " child" } else { "" }
    )
}
