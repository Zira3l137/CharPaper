//! Finding the desktop's background layer and gluing our window into it.

use super::sys::Hwnd;
use super::sys::{self};
use crate::wallpaper::AttachStrategy;
use crate::wallpaper::LayeredMode;
use crate::wallpaper::WallpaperConfig;
use crate::wallpaper::WallpaperError;

const WORKER_W_REQUEST_TIMEOUT_MS: u32 = 1000;

/// The shell windows that are related to desktop rendering, plus which layout this machine uses.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShellWindows {
    pub progman: Hwnd,

    /// The icon host. `0` if we could not find it.
    pub defview: Hwnd,

    /// The wallpaper painter. `0` if absent.
    pub worker_w: Hwnd,

    /// True when `Progman` has `WS_EX_NOREDIRECTIONBITMAP`, i.e. the newer
    /// raised-desktop layout.
    pub raised_desktop: bool,

    /// In the classic layout, the top-level window that owns `defview`.
    pub defview_host: Hwnd,
}

impl ShellWindows {
    /// What `AttachStrategy::Auto` should resolve to for this machine.
    pub fn recommended_strategy(&self) -> AttachStrategy {
        if self.progman == 0 {
            AttachStrategy::None
        } else if self.raised_desktop {
            AttachStrategy::RaisedDesktopChild
        } else if self.worker_w != 0 {
            AttachStrategy::ClassicWorkerW
        } else {
            AttachStrategy::ProgmanDirect
        }
    }
}

/// Ask Explorer to create the background `WorkerW`.
///
/// `0x052C` is undocumented. Two parameter combinations are known to work, and
/// which one a build responds to varies, so we send both. If the window already
/// exists the message is a no-op, which makes sending it twice harmless.
///
/// The `wparam = 0xD, lparam = 0x1` form is the one current Windows 11 builds
/// respond to; the `0, 0` form is the classic Windows 7/10 incantation.
pub fn request_worker_w(progman: Hwnd, log: &mut Vec<String>) {
    const ATTEMPTS: [(usize, isize); 2] = [(0xD, 0x1), (0x0, 0x0)];

    for (wparam, lparam) in ATTEMPTS {
        let mut result: usize = 0;
        let sent = unsafe {
            sys::SendMessageTimeoutW(
                progman,
                sys::WM_SPAWN_WORKER_W,
                wparam,
                lparam,
                sys::SMTO_NORMAL,
                // One second. If Explorer is wedged we would rather log a
                // failure than freeze the wallpaper process.
                WORKER_W_REQUEST_TIMEOUT_MS,
                &mut result,
            )
        };
        log.push(format!(
            "sent 0x052C to Progman (wparam={wparam:#x}, lparam={lparam:#x}) -> \
             returned {sent}, result {result}"
        ));
    }
}

/// Walk the window tree and work out what we are dealing with. Read-only.
pub fn find_shell_windows(log: &mut Vec<String>) -> ShellWindows {
    let mut found = ShellWindows::default();

    found.progman = sys::find_window("Progman");
    if found.progman == 0 {
        log.push("Progman not found -- is Explorer running?".to_string());
        return found;
    }
    log.push(format!("Progman = {:#x}", found.progman));

    found.raised_desktop = sys::has_ex_style(found.progman, sys::WS_EX_NOREDIRECTIONBITMAP);
    log.push(format!(
        "desktop layout = {}",
        if found.raised_desktop {
            "raised (Progman has WS_EX_NOREDIRECTIONBITMAP)"
        } else {
            "classic"
        }
    ));

    // --- classic search: a top-level window hosting SHELLDLL_DefView --------
    //
    // We look at every top-level window and ask "do you have a DefView child?".
    // The last match wins.
    for top in sys::top_level_windows() {
        let defview = sys::find_window_ex(top, 0, "SHELLDLL_DefView");
        if defview != 0 {
            found.defview = defview;
            found.defview_host = top;
            // The wallpaper WorkerW is the next top-level WorkerW *after* the
            // icon host in z-order. Passing 0 as the parent makes
            // FindWindowExW search top-level windows, and `after` then means
            // "start looking after this one".
            let candidate = sys::find_window_ex(0, top, "WorkerW");
            if candidate != 0 {
                found.worker_w = candidate;
            }
            log.push(format!(
                "top-level {:#x} ({}) hosts SHELLDLL_DefView {:#x}; next WorkerW = {:#x}",
                top,
                sys::class_name(top),
                defview,
                candidate
            ));
        }
    }

    // --- raised-desktop search: both live inside Progman -------------------
    if found.raised_desktop {
        let child_defview = sys::find_window_ex(found.progman, 0, "SHELLDLL_DefView");
        let child_worker = sys::find_window_ex(found.progman, 0, "WorkerW");
        if child_defview != 0 {
            found.defview = child_defview;
            found.defview_host = found.progman;
        }
        // Deliberately overwrite: in this layout the top-level result, if any,
        // is not the window that paints our wallpaper.
        found.worker_w = child_worker;
        log.push(format!(
            "Progman children: SHELLDLL_DefView = {child_defview:#x}, WorkerW = {child_worker:#x}"
        ));
    }

    if found.worker_w == 0 {
        log.push("no WorkerW found".to_string());
    }

    found
}

/// Attach `hwnd` to the desktop background layer.
///
/// Returns the strategy actually used plus a log of what happened.
pub fn attach(
    hwnd: Hwnd,
    config: &WallpaperConfig,
    log: &mut Vec<String>,
) -> Result<AttachStrategy, WallpaperError> {
    if !sys::is_window(hwnd) {
        return Err(WallpaperError::DesktopNotFound(
            "our own window (handle is not a valid window)".to_string(),
        ));
    }

    let mut shell = find_shell_windows(log);

    if config.spawn_worker_w && shell.progman != 0 && shell.worker_w == 0 {
        log.push("no WorkerW yet; asking Explorer to create one".to_string());
        request_worker_w(shell.progman, log);
        // Re-scan: the window now exists (or still does not, and we fall back).
        shell = find_shell_windows(log);
    }

    let strategy = match config.strategy {
        AttachStrategy::Auto => shell.recommended_strategy(),
        explicit => explicit,
    };
    log.push(format!("using strategy {strategy:?}"));

    match strategy {
        AttachStrategy::None => Ok(strategy),
        AttachStrategy::RaisedDesktopChild => {
            attach_raised(hwnd, &shell, config, log)?;
            Ok(strategy)
        }
        AttachStrategy::ClassicWorkerW => {
            attach_classic(hwnd, &shell, log)?;
            Ok(strategy)
        }
        AttachStrategy::ProgmanDirect => {
            attach_progman(hwnd, &shell, log)?;
            Ok(strategy)
        }
        AttachStrategy::Auto => unreachable!("Auto was resolved above"),
    }
}

// ---------------------------------------------------------------------------
// The three attach paths
// ---------------------------------------------------------------------------

/// Attaches to the classic layout's `WorkerW` window (legacy case)
fn attach_classic(
    hwnd: Hwnd,
    shell: &ShellWindows,
    log: &mut Vec<String>,
) -> Result<(), WallpaperError> {
    if shell.worker_w == 0 {
        return Err(WallpaperError::DesktopNotFound(
            "a top-level WorkerW (classic layout)".to_string(),
        ));
    }

    set_parent(hwnd, shell.worker_w, log)?;
    fill_parent(hwnd, shell.worker_w, log)?;
    Ok(())
}

/// Attaches to the progman itself effectively rendering on top of the desktop icons.
/// Fallback, if this runs it means none of the other attach methods succeeded essentially meaning
/// the desktop is not supported or the window manager is not compatible.
fn attach_progman(
    hwnd: Hwnd,
    shell: &ShellWindows,
    log: &mut Vec<String>,
) -> Result<(), WallpaperError> {
    if shell.progman == 0 {
        return Err(WallpaperError::DesktopNotFound("Progman".to_string()));
    }
    log.push(
        "warning: ProgmanDirect draws on top of the desktop icons. \
         Useful to prove rendering works, not a final answer."
            .to_string(),
    );
    set_parent(hwnd, shell.progman, log)?;
    fill_parent(hwnd, shell.progman, log)?;
    Ok(())
}

/// The newer Windows 11 path.
///
/// Order matters here. The styles must be applied *before* `SetParent`;
/// applying `WS_EX_LAYERED` afterwards is known to silently fail for some
/// engines (Lively hit this with Godot).
fn attach_raised(
    hwnd: Hwnd,
    shell: &ShellWindows,
    config: &WallpaperConfig,
    log: &mut Vec<String>,
) -> Result<(), WallpaperError> {
    if shell.progman == 0 {
        return Err(WallpaperError::DesktopNotFound("Progman".to_string()));
    }

    // Mark ourselves as a child window.
    let style = sys::get_window_long_ptr(hwnd, sys::GWL_STYLE);
    sys::set_window_long_ptr(hwnd, sys::GWL_STYLE, style | sys::WS_CHILD);
    log.push(format!("added WS_CHILD (style {style:#x} -> {:#x})", style | sys::WS_CHILD));

    set_window_attributes(hwnd, config, log);
    set_parent(hwnd, shell.progman, log)?;

    ensure_window_behind_icon_layer(hwnd, shell, log);
    ensure_worker_w_at_bottom(shell, log);

    fill_parent(hwnd, shell.progman, log)?;
    Ok(())
}

/// Sets the window attributes to layered + fully opaque based on the config.
fn set_window_attributes(hwnd: Hwnd, config: &WallpaperConfig, log: &mut Vec<String>) {
    let want_layered = match config.layered {
        LayeredMode::Always => true,
        LayeredMode::Never => false,
        LayeredMode::Auto => true, // the raised path is the case that needs it
    };
    if want_layered {
        let ex = sys::get_window_long_ptr(hwnd, sys::GWL_EXSTYLE);
        if ex & sys::WS_EX_LAYERED == 0 {
            sys::set_window_long_ptr(hwnd, sys::GWL_EXSTYLE, ex | sys::WS_EX_LAYERED);
        }
        let ok = unsafe { sys::SetLayeredWindowAttributes(hwnd, 0, 255, sys::LWA_ALPHA) };
        log.push(format!("WS_EX_LAYERED + alpha 255 applied (ok={})", ok != 0));
    } else {
        log.push("skipping WS_EX_LAYERED (--no-layered)".to_string());
    }
}

/// Moves `hwnd` behind the icon layer, if possible.
fn ensure_window_behind_icon_layer(hwnd: Hwnd, shell: &ShellWindows, log: &mut Vec<String>) {
    if shell.defview != 0 {
        let ok = unsafe {
            sys::SetWindowPos(
                hwnd,
                shell.defview,
                0,
                0,
                0,
                0,
                sys::SWP_NOMOVE | sys::SWP_NOSIZE | sys::SWP_NOACTIVATE,
            )
        };
        log.push(format!("placed below SHELLDLL_DefView (ok={})", ok != 0));
    } else {
        log.push("no SHELLDLL_DefView found; z-order may put us over the icons".to_string());
    }
}

fn ensure_worker_w_at_bottom(shell: &ShellWindows, log: &mut Vec<String>) {
    if shell.worker_w == 0 {
        return;
    }
    // `EnumChildWindows` visits in z-order, front to back, so the last handle
    // it reports is the bottom-most descendant.
    let last = sys::child_windows(shell.progman).last().copied().unwrap_or(0);
    if last == shell.worker_w {
        return;
    }
    let ok = unsafe {
        sys::SetWindowPos(
            shell.worker_w,
            sys::HWND_BOTTOM,
            0,
            0,
            0,
            0,
            sys::SWP_NOMOVE | sys::SWP_NOSIZE | sys::SWP_NOACTIVATE,
        )
    };
    log.push(format!("pushed WorkerW to the bottom of the z-order (ok={})", ok != 0));
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn set_parent(hwnd: Hwnd, parent: Hwnd, log: &mut Vec<String>) -> Result<(), WallpaperError> {
    // SetParent returns the *previous* parent. Zero means either failure or
    // "it had no parent", so we clear the error code and check it explicitly.
    unsafe { sys::SetLastError(0) };
    let previous = unsafe { sys::SetParent(hwnd, parent) };
    let code = sys::last_error();
    if previous == 0 && code != 0 {
        return Err(WallpaperError::NativeCall { what: "SetParent", code });
    }
    log.push(format!("SetParent({hwnd:#x} -> {parent:#x}), previous parent {previous:#x}"));
    Ok(())
}

/// Resize our window to cover the whole parent.
///
/// After `SetParent`, our coordinates are relative to the parent's client area,
/// so `(0, 0)` is the parent's top-left corner rather than the screen's. The
/// desktop parent spans the entire virtual screen, which is why filling it
/// covers every monitor at once. Per-monitor placement is a later milestone;
/// it needs `MapWindowPoints` to translate screen coordinates into this
/// parent's space, because a secondary monitor above or left of the primary
/// gives you negative screen coordinates.
fn fill_parent(hwnd: Hwnd, parent: Hwnd, log: &mut Vec<String>) -> Result<(), WallpaperError> {
    let rect = sys::window_rect(parent).ok_or(WallpaperError::NativeCall {
        what: "GetWindowRect(parent)",
        code: sys::last_error(),
    })?;

    let (w, h) = (rect.width(), rect.height());
    let ok =
        unsafe { sys::SetWindowPos(hwnd, 0, 0, 0, w, h, sys::SWP_NOACTIVATE | sys::SWP_NOZORDER) };
    if ok == 0 {
        return Err(WallpaperError::NativeCall {
            what: "SetWindowPos(fill)",
            code: sys::last_error(),
        });
    }
    log.push(format!("sized to parent: {w}x{h} at child-relative (0, 0)"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// A readable dump of the desktop-related parts of the window tree.
///
/// This is the thing to run and paste somewhere when the attach misbehaves on a
/// machine you cannot reproduce on. It only reads.
pub fn dump_window_tree() -> Vec<String> {
    let mut out = vec!["--- desktop window tree ---".to_string()];

    for top in sys::top_level_windows() {
        let class = sys::class_name(top);
        if class != "Progman" && class != "WorkerW" {
            continue;
        }

        let title = sys::window_title(top);
        let rect = sys::window_rect(top).unwrap_or_default();
        out.push(format!(
            "{top:#010x} {class:<16} \"{title}\"  [{},{} {}x{}] exstyle={:#x}",
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            sys::get_window_long_ptr(top, sys::GWL_EXSTYLE),
        ));

        // EnumChildWindows walks all descendants, not just direct children, so
        // we cap the output to keep this readable.
        for (i, child) in sys::child_windows(top).into_iter().enumerate() {
            if i >= 12 {
                out.push("      ... (truncated)".to_string());
                break;
            }
            out.push(format!(
                "      {child:#010x} {:<20} \"{}\"",
                sys::class_name(child),
                sys::window_title(child)
            ));
        }
    }

    out.push("--- end ---".to_string());
    out
}
