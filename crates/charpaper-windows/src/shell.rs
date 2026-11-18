use std::ptr;
use std::thread;

use tracing::warn;
use windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED;
use windows_sys::Win32::System::Com::COINIT_DISABLE_OLE1DDE;
use windows_sys::Win32::System::Com::CoInitializeEx;
use windows_sys::Win32::System::Com::CoUninitialize;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::sys::wide;

// Opens a web address in the default browser, as clicking a link would. ShellExecute may pass
// the address to shell extensions that need COM set up on the calling thread, which Bevy's
// worker threads never do, so it runs on a short-lived thread that does. Not waited for:
// starting a browser can take a moment.
pub fn open_in_browser(url: &str) {
    let (verb, file) = (wide("open"), wide(url));
    let url = url.to_owned();
    let spawned = thread::Builder::new().name("open in browser".into()).spawn(move || {
        let flags = (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32;
        let com = unsafe { CoInitializeEx(ptr::null(), flags) } >= 0;
        let result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // Not a real handle: anything above 32 means it worked, and anything else is an
        // error code from before Win32 had GetLastError.
        if result as isize <= 32 {
            warn!("cannot open {url}: error {}", result as isize);
        }
        if com {
            unsafe { CoUninitialize() };
        }
    });
    if let Err(err) = spawned {
        warn!("cannot open a web page: {err}");
    }
}
