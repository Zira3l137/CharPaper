use charpaper_wallpaper::ScreenArea;
use charpaper_wallpaper::WallpaperError;

use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
use windows_sys::Win32::Foundation::SetLastError;
use windows_sys::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows_sys::Win32::UI::WindowsAndMessaging::GWL_STYLE;
use windows_sys::Win32::UI::WindowsAndMessaging::HWND_BOTTOM;
use windows_sys::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
use windows_sys::Win32::UI::WindowsAndMessaging::SMTO_NORMAL;
use windows_sys::Win32::UI::WindowsAndMessaging::SPI_SETDESKWALLPAPER;
use windows_sys::Win32::UI::WindowsAndMessaging::SPIF_UPDATEINIFILE;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE;
use windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOMOVE;
use windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE;
use windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER;
use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW;
use windows_sys::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows_sys::Win32::UI::WindowsAndMessaging::SetParent;
use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos;
use windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;
use windows_sys::Win32::UI::WindowsAndMessaging::WS_CHILD;
use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_NOREDIRECTIONBITMAP;

use crate::options::AttachStrategy;
use crate::options::Options;
use crate::sys;
use crate::sys::Hwnd;

const WORKER_W_REQUEST_TIMEOUT_MS: u32 = 1000;

// Undocumented: asks Progman to create the WorkerW window behind the icons.
const WM_SPAWN_WORKER_W: u32 = 0x052C;

#[derive(Clone, Copy, Debug, Default)]
pub struct ShellWindows {
    pub progman: Hwnd,
    pub defview: Hwnd,
    pub worker_w: Hwnd,
    pub raised_desktop: bool,
}

impl ShellWindows {
    pub fn recommended_strategy(&self) -> AttachStrategy {
        if self.raised_desktop {
            AttachStrategy::RaisedDesktopChild
        } else if !self.worker_w.is_null() {
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
            SendMessageTimeoutW(
                progman.0,
                WM_SPAWN_WORKER_W,
                wparam,
                lparam,
                SMTO_NORMAL,
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
    let mut found = ShellWindows { progman: sys::find_window("Progman"), ..Default::default() };
    if found.progman.is_null() {
        error!("Progman not found -- is Explorer running?");
        return found;
    }
    debug!("Progman = {:#x}", found.progman);

    found.raised_desktop = sys::has_ex_style(found.progman, WS_EX_NOREDIRECTIONBITMAP);
    debug!(
        "desktop layout = {}",
        if found.raised_desktop {
            "raised (Progman has WS_EX_NOREDIRECTIONBITMAP)"
        } else {
            "classic"
        }
    );

    for top in sys::top_level_windows() {
        let defview = sys::find_window_ex(top, Hwnd::NULL, "SHELLDLL_DefView");
        if !defview.is_null() {
            found.defview = defview;
            // With no parent this searches top-level windows, starting after `top` in z-order.
            let candidate = sys::find_window_ex(Hwnd::NULL, top, "WorkerW");
            if !candidate.is_null() {
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
        let child_defview = sys::find_window_ex(found.progman, Hwnd::NULL, "SHELLDLL_DefView");
        let child_worker = sys::find_window_ex(found.progman, Hwnd::NULL, "WorkerW");
        if !child_defview.is_null() {
            found.defview = child_defview;
        }
        // In this layout a top-level WorkerW is not the one behind the icons.
        found.worker_w = child_worker;
        debug!(
            "Progman children: SHELLDLL_DefView = {child_defview:#x}, WorkerW = {child_worker:#x}"
        );
    }

    if found.worker_w.is_null() {
        warn!("no WorkerW found");
    }

    found
}

pub fn attach(hwnd: Hwnd, area: ScreenArea, options: &Options) -> Result<(), WallpaperError> {
    if !sys::is_window(hwnd) {
        return Err(WallpaperError::DesktopNotFound(
            "our own window (handle is not a valid window)".to_string(),
        ));
    }

    let mut shell = find_shell_windows();
    if shell.progman.is_null() {
        return Err(WallpaperError::DesktopNotFound("Progman".to_string()));
    }

    if !options.no_spawn_workerw && shell.worker_w.is_null() {
        debug!("no WorkerW yet; asking Explorer to create one");
        request_worker_w(shell.progman);
        shell = find_shell_windows();
    }

    let strategy = match options.strategy {
        AttachStrategy::Auto => shell.recommended_strategy(),
        explicit => explicit,
    };
    let parent = match strategy {
        AttachStrategy::RaisedDesktopChild => attach_raised(hwnd, &shell, options)?,
        AttachStrategy::ClassicWorkerW => attach_classic(hwnd, &shell)?,
        AttachStrategy::ProgmanDirect => attach_progman(hwnd, &shell)?,
        AttachStrategy::Auto => unreachable!("Auto was resolved above"),
    };
    place(hwnd, parent, area)?;
    info!("attached {hwnd:#x} to the desktop with the {strategy:?} strategy, over {area:?}");
    Ok(())
}

fn attach_classic(hwnd: Hwnd, shell: &ShellWindows) -> Result<Hwnd, WallpaperError> {
    if shell.worker_w.is_null() {
        return Err(WallpaperError::DesktopNotFound(
            "a top-level WorkerW (classic layout)".to_string(),
        ));
    }

    set_parent(hwnd, shell.worker_w)?;
    Ok(shell.worker_w)
}

fn attach_progman(hwnd: Hwnd, shell: &ShellWindows) -> Result<Hwnd, WallpaperError> {
    warn!("ProgmanDirect draws over the desktop icons; it proves rendering works, nothing more");
    set_parent(hwnd, shell.progman)?;
    Ok(shell.progman)
}

fn attach_raised(
    hwnd: Hwnd,
    shell: &ShellWindows,
    options: &Options,
) -> Result<Hwnd, WallpaperError> {
    // Styles must be set before SetParent. WS_EX_LAYERED added afterwards can silently fail.
    let style = sys::get_window_long_ptr(hwnd, GWL_STYLE);
    let child = style | WS_CHILD as isize;
    sys::set_window_long_ptr(hwnd, GWL_STYLE, child);
    debug!("added WS_CHILD (style {style:#x} -> {child:#x})");

    if options.not_layered {
        debug!("leaving out WS_EX_LAYERED (--not-layered)");
    } else {
        make_layered(hwnd);
    }
    set_parent(hwnd, shell.progman)?;

    ensure_window_behind_icon_layer(hwnd, shell);
    ensure_worker_w_at_bottom(shell);
    Ok(shell.progman)
}

fn make_layered(hwnd: Hwnd) {
    let ex = sys::get_window_long_ptr(hwnd, GWL_EXSTYLE);
    let layered = WS_EX_LAYERED as isize;
    if ex & layered == 0 {
        sys::set_window_long_ptr(hwnd, GWL_EXSTYLE, ex | layered);
    }
    let ok = unsafe { SetLayeredWindowAttributes(hwnd.0, 0, 255, LWA_ALPHA) };
    debug!("WS_EX_LAYERED + alpha 255 applied (ok={})", ok != 0);
}

fn ensure_window_behind_icon_layer(hwnd: Hwnd, shell: &ShellWindows) {
    if !shell.defview.is_null() {
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
        let ok = unsafe { SetWindowPos(hwnd.0, shell.defview.0, 0, 0, 0, 0, flags) };
        debug!("placed below SHELLDLL_DefView (ok={})", ok != 0);
    } else {
        warn!("no SHELLDLL_DefView found; z-order may put us over the icons");
    }
}

fn ensure_worker_w_at_bottom(shell: &ShellWindows) {
    if shell.worker_w.is_null() {
        return;
    }
    // EnumChildWindows goes front to back, so the last window is the bottom-most one.
    let last = sys::child_windows(shell.progman).last().copied();
    if last == Some(shell.worker_w) {
        return;
    }
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    let ok = unsafe { SetWindowPos(shell.worker_w.0, HWND_BOTTOM, 0, 0, 0, 0, flags) };
    debug!("pushed WorkerW to the bottom of the z-order (ok={})", ok != 0);
}

fn set_parent(hwnd: Hwnd, parent: Hwnd) -> Result<(), WallpaperError> {
    // SetParent returns the old parent, and 0 means both "failed" and "had none".
    // Only the error code tells them apart.
    unsafe { SetLastError(0) };
    let previous = Hwnd(unsafe { SetParent(hwnd.0, parent.0) });
    let code = sys::last_error();
    if previous.is_null() && code != 0 {
        return Err(WallpaperError::native("SetParent", code));
    }
    debug!("SetParent({hwnd:#x} -> {parent:#x}), previous parent {previous:#x}");
    Ok(())
}

// A child's position counts from its parent's corner. The parent covers every monitor, so
// that corner is the top-left of the whole layout, which is below zero whenever a monitor sits
// left of or above the primary one.
pub fn place(hwnd: Hwnd, parent: Hwnd, area: ScreenArea) -> Result<(), WallpaperError> {
    let origin = sys::window_rect(parent)
        .ok_or_else(|| WallpaperError::native("GetWindowRect(parent)", sys::last_error()))?;

    let (x, y) = (area.x - origin.left, area.y - origin.top);
    let (w, h) = (area.width as i32, area.height as i32);
    let flags = SWP_NOACTIVATE | SWP_NOZORDER;
    let ok = unsafe { SetWindowPos(hwnd.0, std::ptr::null_mut(), x, y, w, h, flags) };
    if ok == 0 {
        return Err(WallpaperError::native("SetWindowPos(place)", sys::last_error()));
    }
    debug!("placed at {w}x{h}, ({x}, {y}) inside the parent");
    Ok(())
}

// Windows 10 leaves the last frame of a window that left the WorkerW behind the icons on screen,
// until something repaints the desktop: often not before the next sign-in. Hiding the window
// first, then setting the wallpaper again to what it already is, makes Explorer repaint. With
// no file given, it reads the wallpaper back from the user's settings, slideshow and solid
// color included; Lively Wallpaper refreshes the same way. Newer Windows 11's raised desktop
// has no such ghost, and there this would tear the WorkerW down instead.
pub fn leave(windows: &[Hwnd]) {
    for &hwnd in windows {
        unsafe { ShowWindow(hwnd.0, SW_HIDE) };
    }
    if !windows.is_empty() {
        repaint();
    }
}

pub fn repaint() {
    if find_shell_windows().raised_desktop {
        return;
    }
    let null = std::ptr::null_mut();
    let ok = unsafe { SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, null, SPIF_UPDATEINIFILE) };
    debug!("repainted the desktop (ok={})", ok != 0);
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
            rect.right - rect.left,
            rect.bottom - rect.top,
            sys::get_window_long_ptr(top, GWL_EXSTYLE),
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
