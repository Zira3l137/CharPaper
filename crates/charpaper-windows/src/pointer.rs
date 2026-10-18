use charpaper_wallpaper::PointerButton;
use charpaper_wallpaper::PointerEvent;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::Wake;
use charpaper_wallpaper::WallpaperError;
use charpaper_wallpaper::WindowKey;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::WHEEL_DELTA;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_MBUTTONDOWN;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_MBUTTONUP;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEHWHEEL;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEWHEEL;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_RBUTTONDOWN;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_XBUTTONDOWN;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_XBUTTONUP;
use windows_sys::Win32::UI::WindowsAndMessaging::XBUTTON1;
use windows_sys::Win32::UI::WindowsAndMessaging::XBUTTON2;

use crate::backend::AttachedWindows;
use crate::hook::MouseHook;
use crate::hook::RawMouseEvent;
use crate::sys;
use crate::sys::Hwnd;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    // The desktop, over the part of it one of our windows covers.
    Desktop(WindowKey, Hwnd),
    // One of our windows really is under the cursor, so winit already delivers these.
    OwnWindow,
    Elsewhere,
}

pub struct DesktopPointer {
    hook: MouseHook,
    windows: AttachedWindows,
    // The window whose part of the desktop the pointer is on.
    inside: Option<(WindowKey, Hwnd)>,
    // One bit per PointerButton pressed over the desktop.
    held: u8,
    // Asking for a class name on every event adds up on a 1000 Hz mouse.
    last_root: Option<(Hwnd, bool)>,
}

type Out = Vec<(WindowKey, PointerEvent)>;

impl DesktopPointer {
    pub fn new(windows: AttachedWindows, wake: Wake) -> Result<Self, WallpaperError> {
        let hook = MouseHook::install(wake)?;
        Ok(Self { hook, windows, inside: None, held: 0, last_root: None })
    }

    fn translate(&mut self, raw: RawMouseEvent, out: &mut Out) {
        // Like native mouse capture: a drag belongs to the window that saw the press until every
        // button is up, even across other windows and monitors. Otherwise a release elsewhere
        // never reaches Bevy.
        let target = match self.inside {
            Some((key, hwnd)) if self.held != 0 => Target::Desktop(key, hwnd),
            _ => self.target_at(raw.screen),
        };

        let Target::Desktop(key, hwnd) = target else {
            if let Some((key, _)) = self.inside.take()
                && target == Target::Elsewhere
            {
                out.push((key, PointerEvent::Left));
            }
            return;
        };
        // Straight from one window's part of the desktop onto another's.
        if let Some((left, _)) = self.inside.filter(|(inside, _)| *inside != key) {
            out.push((left, PointerEvent::Left));
            self.inside = None;
        }

        let moved = moved_to(hwnd, raw.screen);
        // A click can be the first event we see, e.g. when a window closes under a still cursor.
        if self.inside.is_none() {
            self.inside = Some((key, hwnd));
            out.push((key, PointerEvent::Entered));
            out.push((key, moved));
            if raw.message == WM_MOUSEMOVE {
                return;
            }
        }

        match raw.message {
            WM_MOUSEMOVE => out.push((key, moved)),
            WM_MOUSEWHEEL => {
                let wheel = PointerEvent::Wheel { x: 0.0, y: wheel_notches(raw.mouse_data) };
                out.push((key, wheel));
            }
            // Negated to match winit's sign convention.
            WM_MOUSEHWHEEL => {
                let wheel = PointerEvent::Wheel { x: -wheel_notches(raw.mouse_data), y: 0.0 };
                out.push((key, wheel));
            }
            message => {
                if let Some((button, pressed)) = button_change(message, raw.mouse_data) {
                    self.press_or_release(key, button, pressed, out);
                }
            }
        }
    }

    fn press_or_release(
        &mut self,
        key: WindowKey,
        button: PointerButton,
        pressed: bool,
        out: &mut Out,
    ) {
        let bit = 1 << button as u8;
        if pressed {
            self.held |= bit;
        } else if self.held & bit != 0 {
            self.held &= !bit;
        } else {
            // Pressed somewhere else, released here: not ours.
            return;
        }
        out.push((key, PointerEvent::Button { button, pressed }));
    }

    fn target_at(&mut self, screen: POINT) -> Target {
        let hit = sys::window_from_point(screen);
        if hit.is_null() {
            return Target::Elsewhere;
        }
        let windows = self.windows.list();
        if windows.iter().any(|&(_, hwnd)| hwnd == hit) {
            return Target::OwnWindow;
        }

        // The icon view always lives under Progman or a top-level WorkerW, so the root's class
        // is enough to tell the desktop apart.
        let root = sys::root_ancestor(hit);
        let is_shell = match self.last_root {
            Some((cached, is_shell)) if cached == root => is_shell,
            _ => {
                let is_shell = matches!(sys::class_name(root).as_str(), "Progman" | "WorkerW");
                self.last_root = Some((root, is_shell));
                is_shell
            }
        };
        if !is_shell {
            return Target::Elsewhere;
        }

        // The icon view spans every monitor, but only the parts over our windows are ours.
        windows
            .iter()
            .find(|&&(_, hwnd)| covers(hwnd, screen))
            .map_or(Target::Elsewhere, |&(key, hwnd)| Target::Desktop(key, hwnd))
    }
}

fn covers(hwnd: Hwnd, screen: POINT) -> bool {
    sys::window_rect(hwnd).is_some_and(|r| {
        (r.left..r.right).contains(&screen.x) && (r.top..r.bottom).contains(&screen.y)
    })
}

fn moved_to(hwnd: Hwnd, screen: POINT) -> PointerEvent {
    let client = sys::screen_to_client(hwnd, screen).unwrap_or(screen);
    PointerEvent::Moved { x: client.x as f64, y: client.y as f64 }
}

impl PointerSource for DesktopPointer {
    fn drain(&mut self, out: &mut Out) {
        while let Some(raw) = self.hook.try_recv() {
            self.translate(raw, out);
        }
    }
}

fn button_change(message: u32, mouse_data: u32) -> Option<(PointerButton, bool)> {
    let button = match message {
        WM_LBUTTONDOWN | WM_LBUTTONUP => PointerButton::Left,
        WM_RBUTTONDOWN | WM_RBUTTONUP => PointerButton::Right,
        WM_MBUTTONDOWN | WM_MBUTTONUP => PointerButton::Middle,
        WM_XBUTTONDOWN | WM_XBUTTONUP => match high_word(mouse_data) {
            XBUTTON1 => PointerButton::Back,
            XBUTTON2 => PointerButton::Forward,
            _ => return None,
        },
        _ => return None,
    };

    let pressed =
        matches!(message, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN);
    Some((button, pressed))
}

fn high_word(value: u32) -> u16 {
    (value >> 16) as u16
}

fn wheel_notches(mouse_data: u32) -> f32 {
    high_word(mouse_data) as i16 as f32 / WHEEL_DELTA as f32
}
