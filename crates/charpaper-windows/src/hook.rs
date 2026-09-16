//! A global low-level mouse hook, running on a thread of its own.
//!
//! Windows calls a low-level hook *on the thread that installed it*, from
//! inside that thread's message loop, and waits for it before the event moves
//! on to any application. Two consequences shape this file:
//!
//! * The installing thread must pump messages for as long as the hook lives.
//!   Bevy's main thread does pump them, but only between frames; a long frame
//!   would stall the mouse for the whole session. So the hook gets a dedicated
//!   thread that does nothing but pump.
//! * The callback must be quick and must never block. If it overruns
//!   `LowLevelHooksTimeout` (a few hundred milliseconds), Windows unhooks it
//!   silently. It therefore only copies the event into a bounded queue and
//!   returns; all interpretation happens on the reading side.

use std::cell::RefCell;
use std::sync::mpsc;
use std::thread::JoinHandle;

use charpaper_wallpaper::WallpaperError;
use tracing::debug;
use tracing::warn;

use crate::sys;

/// Several seconds of a 1000 Hz mouse. If the reader stalls for longer than
/// that, new events are dropped rather than blocking the callback.
const QUEUE_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Debug)]
pub struct RawMouseEvent {
    /// `WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, ...
    pub message: u32,
    pub screen: sys::Point,
    pub mouse_data: u32,
}

thread_local! {
    // The callback receives no user pointer, but it always runs on the hook
    // thread, so a thread-local is how it finds the queue.
    static SINK: RefCell<Option<mpsc::SyncSender<RawMouseEvent>>> = const { RefCell::new(None) };
}

pub struct MouseHook {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
    events: mpsc::Receiver<RawMouseEvent>,
}

impl MouseHook {
    pub fn install() -> Result<Self, WallpaperError> {
        let (sink, events) = mpsc::sync_channel(QUEUE_CAPACITY);
        let (ready_tx, ready_rx) = mpsc::channel();

        let thread = std::thread::Builder::new()
            .name("charpaper-mouse-hook".to_string())
            .spawn(move || run(sink, ready_tx))
            .map_err(|source| WallpaperError::NativeCall {
                what: "spawning the hook thread",
                source,
            })?;

        match ready_rx.recv() {
            Ok(Ok(thread_id)) => Ok(Self { thread_id, thread: Some(thread), events }),
            Ok(Err(code)) => {
                let _ = thread.join();
                Err(WallpaperError::native("SetWindowsHookExW", code))
            }
            Err(_) => {
                let _ = thread.join();
                Err(WallpaperError::NativeCall {
                    what: "the hook thread",
                    source: std::io::Error::other("exited before installing the hook"),
                })
            }
        }
    }

    pub fn try_recv(&self) -> Option<RawMouseEvent> {
        self.events.try_recv().ok()
    }
}

impl Drop for MouseHook {
    fn drop(&mut self) {
        // Makes GetMessageW return 0 on the hook thread, which unhooks and exits.
        unsafe { sys::PostThreadMessageW(self.thread_id, sys::WM_QUIT, 0, 0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(sink: mpsc::SyncSender<RawMouseEvent>, ready: mpsc::Sender<Result<u32, u32>>) {
    let mut msg = sys::Msg::default();

    // A thread gets a message queue on its first message call, not when it
    // starts. Create it now so the WM_QUIT from `Drop` has somewhere to land
    // even if the owner is dropped immediately.
    unsafe { sys::PeekMessageW(&mut msg, 0, sys::WM_USER, sys::WM_USER, sys::PM_NOREMOVE) };

    SINK.set(Some(sink));

    let module = unsafe { sys::GetModuleHandleW(std::ptr::null()) };
    let hook = unsafe { sys::SetWindowsHookExW(sys::WH_MOUSE_LL, hook_proc, module, 0) };
    if hook == 0 {
        let _ = ready.send(Err(sys::last_error()));
        return;
    }

    let _ = ready.send(Ok(unsafe { sys::GetCurrentThreadId() }));
    debug!("low-level mouse hook installed");

    // Nothing is ever posted here except WM_QUIT; the loop exists because the
    // hook callback is dispatched from inside GetMessageW.
    loop {
        match unsafe { sys::GetMessageW(&mut msg, 0, 0, 0) } {
            0 => break,
            -1 => {
                warn!("mouse hook message loop failed: {}", std::io::Error::last_os_error());
                break;
            }
            _ => {}
        }
    }

    unsafe { sys::UnhookWindowsHookEx(hook) };
    SINK.set(None);
    debug!("low-level mouse hook removed");
}

unsafe extern "system" fn hook_proc(
    code: i32,
    wparam: sys::WParam,
    lparam: sys::LParam,
) -> sys::LResult {
    if code == sys::HC_ACTION {
        // SAFETY: for WH_MOUSE_LL with HC_ACTION, `lparam` points at an
        // MSLLHOOKSTRUCT that stays valid for the duration of this call.
        let info = unsafe { &*(lparam as *const sys::MsLlHookStruct) };
        let event =
            RawMouseEvent { message: wparam as u32, screen: info.pt, mouse_data: info.mouse_data };

        SINK.with_borrow(|sink| {
            if let Some(sink) = sink {
                let _ = sink.try_send(event);
            }
        });
    }

    // Always pass the event on unchanged. Returning non-zero instead would
    // swallow it, and the user's mouse would stop working everywhere.
    unsafe { sys::CallNextHookEx(0, code, wparam, lparam) }
}
