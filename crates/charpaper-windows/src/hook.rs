use std::cell::RefCell;
use std::sync::mpsc;
use std::thread::JoinHandle;

use charpaper_wallpaper::Wake;
use charpaper_wallpaper::WallpaperError;
use tracing::debug;
use tracing::warn;
use windows_sys::Win32::Foundation::LPARAM;
use windows_sys::Win32::Foundation::LRESULT;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Foundation::WPARAM;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::CallNextHookEx;
use windows_sys::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows_sys::Win32::UI::WindowsAndMessaging::HC_ACTION;
use windows_sys::Win32::UI::WindowsAndMessaging::MSG;
use windows_sys::Win32::UI::WindowsAndMessaging::MSLLHOOKSTRUCT;
use windows_sys::Win32::UI::WindowsAndMessaging::PM_NOREMOVE;
use windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW;
use windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowsHookExW;
use windows_sys::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx;
use windows_sys::Win32::UI::WindowsAndMessaging::WH_MOUSE_LL;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_USER;

use crate::sys;

const QUEUE_CAPACITY: usize = 4096;

#[derive(Clone, Copy)]
pub struct RawMouseEvent {
    pub message: u32,
    pub screen: POINT,
    pub mouse_data: u32,
}

struct Sink {
    events: mpsc::SyncSender<RawMouseEvent>,
    wake: Wake,
}

thread_local! {
    // The callback gets no user data, but it always runs on the hook thread.
    static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
}

// Windows calls a mouse hook on the thread that installed it, and only while that thread
// waits for messages. It also quietly removes a hook that answers too slowly. So the hook
// gets a thread of its own that does nothing but wait, and the callback only queues events.
pub struct MouseHook {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
    events: mpsc::Receiver<RawMouseEvent>,
}

impl MouseHook {
    pub fn install(wake: Wake) -> Result<Self, WallpaperError> {
        let (sink, events) = mpsc::sync_channel(QUEUE_CAPACITY);
        let (ready_tx, ready_rx) = mpsc::channel();

        let thread = std::thread::Builder::new()
            .name("charpaper-mouse-hook".to_string())
            .spawn(move || run(Sink { events: sink, wake }, ready_tx))
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
        // Makes GetMessageW return 0 on the hook thread, which then unhooks and exits.
        unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(sink: Sink, ready: mpsc::Sender<Result<u32, u32>>) {
    // SAFETY: all zeroes is a valid MSG: a null window and numbers.
    let mut msg: MSG = unsafe { std::mem::zeroed() };

    // A thread only gets a message queue on its first message call. Create it now so a
    // quick WM_QUIT from Drop has somewhere to land.
    let no_window = std::ptr::null_mut();
    unsafe { PeekMessageW(&mut msg, no_window, WM_USER, WM_USER, PM_NOREMOVE) };

    SINK.set(Some(sink));

    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), module, 0) };
    if hook.is_null() {
        let _ = ready.send(Err(sys::last_error()));
        return;
    }

    let _ = ready.send(Ok(unsafe { GetCurrentThreadId() }));
    debug!("low-level mouse hook installed");

    // The hook callback is delivered from inside GetMessageW; nothing else is posted here.
    loop {
        match unsafe { GetMessageW(&mut msg, no_window, 0, 0) } {
            0 => break,
            -1 => {
                warn!("mouse hook message loop failed: {}", std::io::Error::last_os_error());
                break;
            }
            _ => {}
        }
    }

    unsafe { UnhookWindowsHookEx(hook) };
    SINK.set(None);
    debug!("low-level mouse hook removed");
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_MOUSE_LL with HC_ACTION, lparam points at a valid MSLLHOOKSTRUCT.
        let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        let event =
            RawMouseEvent { message: wparam as u32, screen: info.pt, mouse_data: info.mouseData };

        SINK.with_borrow(|sink| {
            if let Some(sink) = sink {
                // Never block here. A full queue drops the event instead.
                let _ = sink.events.try_send(event);
                // Not for moves: a fast mouse would keep the app from ever resting.
                if event.message != WM_MOUSEMOVE {
                    (sink.wake)();
                }
            }
        });
    }

    // Always pass the event on. Returning non-zero would swallow it for the whole system.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}
