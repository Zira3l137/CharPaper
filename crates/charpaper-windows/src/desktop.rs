use charpaper_wallpaper::AttachStrategy;
use charpaper_wallpaper::LayeredMode;
use charpaper_wallpaper::ScreenArea;
use charpaper_wallpaper::WallpaperConfig;
use charpaper_wallpaper::WallpaperError;

use tracing::debug;
use tracing::error;
use tracing::warn;

use crate::sys;
use crate::sys::Hwnd;

const WORKER_W_REQUEST_TIMEOUT_MS: u32 = 1000;

#[derive(Clone, Copy, Debug, Default)]
pub struct ShellWindows {
    pub progman: Hwnd,
    pub defview: Hwnd,
    pub worker_w: Hwnd,
    pub raised_desktop: bool,
}

impl ShellWindows {
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

// 0x052C is undocumented. Windows builds differ in which parameters they answer, so both
// known forms are sent. It does nothing when the WorkerW already exists.
pub fn request_worker_w(progman: Hwnd) {
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
                // A timeout, so a hung Explorer can't hang us too.
                WORKER_W_REQUEST_TIMEOUT_MS,
                &mut result,
            )
        };
        debug!(
            "sent 0x052C to Progman (wparam={wparam:#x}, lparam={lparam:#x}) -> \
             returned {sent}, result {result}"
        );
    }
}

pub fn find_shell_windows() -> ShellWindows {
    let mut found = ShellWindows::default();

    found.progman = sys::find_window("Progman");
    if found.progman == 0 {
        error!("Progman not found -- is Explorer running?");
        return found;
    }
    debug!("Progman = {:#x}", found.progman);

    found.raised_desktop = sys::has_ex_style(found.progman, sys::WS_EX_NOREDIRECTIONBITMAP);
    debug!(
        "desktop layout = {}",
        if found.raised_desktop {
            "raised (Progman has WS_EX_NOREDIRECTIONBITMAP)"
        } else {
            "classic"
        }
    );

    for top in sys::top_level_windows() {
        let defview = sys::find_window_ex(top, 0, "SHELLDLL_DefView");
        if defview != 0 {
            found.defview = defview;
            // With no parent this searches top-level windows, starting after `top` in z-order.
            let candidate = sys::find_window_ex(0, top, "WorkerW");
            if candidate != 0 {
                found.worker_w = candidate;
            }
            debug!(
                "top-level {:#x} ({}) hosts SHELLDLL_DefView {:#x}; next WorkerW = {:#x}",
                top,
                sys::class_name(top),
                defview,
                candidate
            );
        }
    }

    if found.raised_desktop {
        let child_defview = sys::find_window_ex(found.progman, 0, "SHELLDLL_DefView");
        let child_worker = sys::find_window_ex(found.progman, 0, "WorkerW");
        if child_defview != 0 {
            found.defview = child_defview;
        }
        // In this layout a top-level WorkerW is not the one behind the icons.
        found.worker_w = child_worker;
        debug!(
            "Progman children: SHELLDLL_DefView = {child_defview:#x}, WorkerW = {child_worker:#x}"
        );
    }

    if found.worker_w == 0 {
        warn!("no WorkerW found");
    }

    found
}

pub fn attach(
    hwnd: Hwnd,
    area: ScreenArea,
    config: &WallpaperConfig,
) -> Result<AttachStrategy, WallpaperError> {
    if !sys::is_window(hwnd) {
        return Err(WallpaperError::DesktopNotFound(
            "our own window (handle is not a valid window)".to_string(),
        ));
    }

    let mut shell = find_shell_windows();

    if config.spawn_worker_w && shell.progman != 0 && shell.worker_w == 0 {
        debug!("no WorkerW yet; asking Explorer to create one");
        request_worker_w(shell.progman);
        shell = find_shell_windows();
    }

    let strategy = match config.strategy {
        AttachStrategy::Auto => shell.recommended_strategy(),
        explicit => explicit,
    };
    debug!("using strategy {strategy:?}");

    let parent = match strategy {
        AttachStrategy::None => return Ok(strategy),
        AttachStrategy::RaisedDesktopChild => attach_raised(hwnd, &shell, config)?,
        AttachStrategy::ClassicWorkerW => attach_classic(hwnd, &shell)?,
        AttachStrategy::ProgmanDirect => attach_progman(hwnd, &shell)?,
        AttachStrategy::Auto => unreachable!("Auto was resolved above"),
    };
    place(hwnd, parent, area)?;
    Ok(strategy)
}

fn attach_classic(hwnd: Hwnd, shell: &ShellWindows) -> Result<Hwnd, WallpaperError> {
    if shell.worker_w == 0 {
        return Err(WallpaperError::DesktopNotFound(
            "a top-level WorkerW (classic layout)".to_string(),
        ));
    }

    set_parent(hwnd, shell.worker_w)?;
    Ok(shell.worker_w)
}

fn attach_progman(hwnd: Hwnd, shell: &ShellWindows) -> Result<Hwnd, WallpaperError> {
    if shell.progman == 0 {
        return Err(WallpaperError::DesktopNotFound("Progman".to_string()));
    }
    warn!("ProgmanDirect draws over the desktop icons; it proves rendering works, nothing more");
    set_parent(hwnd, shell.progman)?;
    Ok(shell.progman)
}

fn attach_raised(
    hwnd: Hwnd,
    shell: &ShellWindows,
    config: &WallpaperConfig,
) -> Result<Hwnd, WallpaperError> {
    if shell.progman == 0 {
        return Err(WallpaperError::DesktopNotFound("Progman".to_string()));
    }

    // Styles must be set before SetParent. WS_EX_LAYERED added afterwards can silently fail.
    let style = sys::get_window_long_ptr(hwnd, sys::GWL_STYLE);
    sys::set_window_long_ptr(hwnd, sys::GWL_STYLE, style | sys::WS_CHILD);
    debug!("added WS_CHILD (style {style:#x} -> {:#x})", style | sys::WS_CHILD);

    set_window_attributes(hwnd, config);
    set_parent(hwnd, shell.progman)?;

    ensure_window_behind_icon_layer(hwnd, shell);
    ensure_worker_w_at_bottom(shell);
    Ok(shell.progman)
}

fn set_window_attributes(hwnd: Hwnd, config: &WallpaperConfig) {
    if config.layered != LayeredMode::Never {
        let ex = sys::get_window_long_ptr(hwnd, sys::GWL_EXSTYLE);
        if ex & sys::WS_EX_LAYERED == 0 {
            sys::set_window_long_ptr(hwnd, sys::GWL_EXSTYLE, ex | sys::WS_EX_LAYERED);
        }
        let ok = unsafe { sys::SetLayeredWindowAttributes(hwnd, 0, 255, sys::LWA_ALPHA) };
        debug!("WS_EX_LAYERED + alpha 255 applied (ok={})", ok != 0);
    } else {
        debug!("skipping WS_EX_LAYERED (--layered never)");
    }
}

fn ensure_window_behind_icon_layer(hwnd: Hwnd, shell: &ShellWindows) {
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
        debug!("placed below SHELLDLL_DefView (ok={})", ok != 0);
    } else {
        warn!("no SHELLDLL_DefView found; z-order may put us over the icons");
    }
}

fn ensure_worker_w_at_bottom(shell: &ShellWindows) {
    if shell.worker_w == 0 {
        return;
    }
    // EnumChildWindows goes front to back, so the last window is the bottom-most one.
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
    debug!("pushed WorkerW to the bottom of the z-order (ok={})", ok != 0);
}

fn set_parent(hwnd: Hwnd, parent: Hwnd) -> Result<(), WallpaperError> {
    // SetParent returns the old parent, and 0 means both "failed" and "had none".
    // Only the error code tells them apart.
    unsafe { sys::SetLastError(0) };
    let previous = unsafe { sys::SetParent(hwnd, parent) };
    let code = sys::last_error();
    if previous == 0 && code != 0 {
        return Err(WallpaperError::native("SetParent", code));
    }
    debug!("SetParent({hwnd:#x} -> {parent:#x}), previous parent {previous:#x}");
    Ok(())
}

// A child's position counts from its parent's corner. The parent covers every monitor, so
// that corner is the top-left of the whole layout, which is below zero whenever a monitor sits
// left of or above the primary one.
fn place(hwnd: Hwnd, parent: Hwnd, area: ScreenArea) -> Result<(), WallpaperError> {
    let origin = sys::window_rect(parent)
        .ok_or_else(|| WallpaperError::native("GetWindowRect(parent)", sys::last_error()))?;

    let (x, y) = (area.x - origin.left, area.y - origin.top);
    let (w, h) = (area.width as i32, area.height as i32);
    let ok =
        unsafe { sys::SetWindowPos(hwnd, 0, x, y, w, h, sys::SWP_NOACTIVATE | sys::SWP_NOZORDER) };
    if ok == 0 {
        return Err(WallpaperError::native("SetWindowPos(place)", sys::last_error()));
    }
    debug!("placed at {w}x{h}, ({x}, {y}) inside the parent");
    Ok(())
}

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

        for (i, child) in sys::child_windows(top).into_iter().enumerate() {
            // This lists all descendants, not just children, so cap it.
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
