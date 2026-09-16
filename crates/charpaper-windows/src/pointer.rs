//! Decides which hooked mouse events belong to our window, and translates them.
//!
//! The hook sees every mouse event in the session. What we pass on is only
//! what the desktop would have taken from us: events over the shell's windows,
//! plus everything that belongs to a drag which started there.

use charpaper_wallpaper::PointerButton;
use charpaper_wallpaper::PointerEvent;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::WallpaperError;

use crate::hook::MouseHook;
use crate::hook::RawMouseEvent;
use crate::sys;
use crate::sys::Hwnd;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    /// The shell has it, so we never will. Forward.
    Desktop,
    /// Our window really is under the cursor (`ProgmanDirect`, for one), so
    /// winit already delivers these. Forwarding would double them up.
    OwnWindow,
    /// Some other application's window.
    Elsewhere,
}

pub struct DesktopPointer {
    hook: MouseHook,
    window: Hwnd,
    inside: bool,
    /// Bit per [`PointerButton`] we forwarded a press for.
    held: u8,
    /// `GetClassNameW` on every event of a 1000 Hz mouse adds up, and the root
    /// window only changes when the cursor crosses into another application.
    last_root: Option<(Hwnd, bool)>,
}

impl DesktopPointer {
    pub fn new(window: Hwnd) -> Result<Self, WallpaperError> {
        Ok(Self { hook: MouseHook::install()?, window, inside: false, held: 0, last_root: None })
    }

    fn translate(&mut self, raw: RawMouseEvent, out: &mut Vec<PointerEvent>) {
        // Mirrors the mouse capture every native window gets on button press:
        // until the last button is released, the drag belongs to whoever saw
        // the press. Without it a release over another window never reaches
        // Bevy and the button stays down forever.
        let target = if self.held != 0 { Target::Desktop } else { self.target_at(raw.screen) };

        if target != Target::Desktop {
            if self.inside {
                self.inside = false;
                if target == Target::Elsewhere {
                    out.push(PointerEvent::Left);
                }
            }
            return;
        }

        let moved = self.moved_to(raw.screen);
        if !self.inside {
            // Not every entry starts with a move: a window closing under a
            // still cursor turns the next click into the first event we see.
            self.inside = true;
            out.push(PointerEvent::Entered);
            out.push(moved);
            if raw.message == sys::WM_MOUSEMOVE {
                return;
            }
        }

        match raw.message {
            sys::WM_MOUSEMOVE => out.push(moved),
            sys::WM_MOUSEWHEEL => {
                out.push(PointerEvent::Wheel { x: 0.0, y: wheel_notches(raw.mouse_data) })
            }
            // Negated to match winit, which flips Windows' horizontal sign to
            // agree with every other platform.
            sys::WM_MOUSEHWHEEL => {
                out.push(PointerEvent::Wheel { x: -wheel_notches(raw.mouse_data), y: 0.0 })
            }
            message => {
                if let Some((button, pressed)) = button_change(message, raw.mouse_data) {
                    self.press_or_release(button, pressed, out);
                }
            }
        }
    }

    fn press_or_release(
        &mut self,
        button: PointerButton,
        pressed: bool,
        out: &mut Vec<PointerEvent>,
    ) {
        let bit = 1 << button as u8;
        if pressed {
            self.held |= bit;
        } else if self.held & bit != 0 {
            self.held &= !bit;
        } else {
            // Pressed somewhere else, released over the desktop. That press
            // was never ours, so neither is the release.
            return;
        }
        out.push(PointerEvent::Button { button, pressed });
    }

    /// Evaluated when Bevy drains the queue, up to a frame after the event
    /// happened. A window that appears or vanishes within that frame can be
    /// misjudged for one event.
    fn target_at(&mut self, screen: sys::Point) -> Target {
        let hit = sys::window_from_point(screen);
        if hit == 0 {
            return Target::Elsewhere;
        }
        if hit == self.window {
            return Target::OwnWindow;
        }

        // In both desktop layouts the icon view is a descendant of `Progman`
        // or of a top-level `WorkerW`, so the root's class is enough to tell
        // the desktop apart from everything stacked above it.
        let root = sys::root_ancestor(hit);
        let is_shell = match self.last_root {
            Some((cached, is_shell)) if cached == root => is_shell,
            _ => {
                let is_shell = matches!(sys::class_name(root).as_str(), "Progman" | "WorkerW");
                self.last_root = Some((root, is_shell));
                is_shell
            }
        };

        if is_shell { Target::Desktop } else { Target::Elsewhere }
    }

    fn moved_to(&self, screen: sys::Point) -> PointerEvent {
        let client = sys::screen_to_client(self.window, screen).unwrap_or(screen);
        PointerEvent::Moved { x: client.x as f64, y: client.y as f64 }
    }
}

impl PointerSource for DesktopPointer {
    fn drain(&mut self, out: &mut Vec<PointerEvent>) {
        while let Some(raw) = self.hook.try_recv() {
            self.translate(raw, out);
        }
    }
}

fn button_change(message: u32, mouse_data: u32) -> Option<(PointerButton, bool)> {
    let button = match message {
        sys::WM_LBUTTONDOWN | sys::WM_LBUTTONUP => PointerButton::Left,
        sys::WM_RBUTTONDOWN | sys::WM_RBUTTONUP => PointerButton::Right,
        sys::WM_MBUTTONDOWN | sys::WM_MBUTTONUP => PointerButton::Middle,
        sys::WM_XBUTTONDOWN | sys::WM_XBUTTONUP => match high_word(mouse_data) {
            sys::XBUTTON1 => PointerButton::Back,
            sys::XBUTTON2 => PointerButton::Forward,
            _ => return None,
        },
        _ => return None,
    };

    let pressed = matches!(
        message,
        sys::WM_LBUTTONDOWN | sys::WM_RBUTTONDOWN | sys::WM_MBUTTONDOWN | sys::WM_XBUTTONDOWN
    );
    Some((button, pressed))
}

fn high_word(value: u32) -> u16 {
    (value >> 16) as u16
}

/// The high word is a signed delta in multiples of `WHEEL_DELTA`.
fn wheel_notches(mouse_data: u32) -> f32 {
    high_word(mouse_data) as i16 as f32 / sys::WHEEL_DELTA as f32
}
