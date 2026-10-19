use std::ffi::c_void;
use std::io;
use std::path::Path;

use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::Foundation::WIN32_ERROR;
use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
use windows_sys::Win32::System::Registry::REG_SZ;
use windows_sys::Win32::System::Registry::RRF_RT_REG_BINARY;
use windows_sys::Win32::System::Registry::RRF_RT_REG_SZ;
use windows_sys::Win32::System::Registry::RegDeleteKeyValueW;
use windows_sys::Win32::System::Registry::RegGetValueW;
use windows_sys::Win32::System::Registry::RegSetKeyValueW;

use crate::sys::from_wide;
use crate::sys::wide;

// At sign-in Windows starts every command in the user's Run key. Settings > Apps > Startup and
// Task Manager switch one off without removing it, by a value of the same name under
// StartupApproved, which this reads too and clears when switching the app on.
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const NAME: &str = "CharPaper";

// On only when the command is this executable: another copy, a nightly beside a release say,
// may have it instead, and a copy that was moved has left a stale path behind.
pub fn starts_at_login(exe: &Path) -> io::Result<bool> {
    let Some(command) = read_string(RUN, NAME)? else {
        return Ok(false);
    };
    let ours =
        command.trim().trim_matches('"').to_lowercase() == exe.to_string_lossy().to_lowercase();
    Ok(ours && !switched_off()?)
}

pub fn set_starts_at_login(exe: &Path, on: bool) -> io::Result<()> {
    delete(APPROVED, NAME)?;
    match on {
        true => write_string(RUN, NAME, &format!("\"{}\"", exe.display())),
        false => delete(RUN, NAME),
    }
}

// Its first byte is even while the command may run and odd once it was switched off. The rest
// is when that happened.
fn switched_off() -> io::Result<bool> {
    let mut data = [0u8; 64];
    let mut size = data.len() as u32;
    let (key, name) = (wide(APPROVED), wide(NAME));
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_BINARY,
            std::ptr::null_mut(),
            data.as_mut_ptr().cast(),
            &mut size,
        )
    };
    match status {
        ERROR_SUCCESS => Ok(size > 0 && data[0] & 1 == 1),
        ERROR_FILE_NOT_FOUND => Ok(false),
        code => Err(os_error(code)),
    }
}

// Asked twice: first for the size, then for the text.
fn read_string(key: &str, name: &str) -> io::Result<Option<String>> {
    let (key, name) = (wide(key), wide(name));
    let query = |data: *mut c_void, size: &mut u32| unsafe {
        let null = std::ptr::null_mut();
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            null,
            data,
            size,
        )
    };
    let mut size = 0;
    match query(std::ptr::null_mut(), &mut size) {
        ERROR_SUCCESS => {}
        ERROR_FILE_NOT_FOUND => return Ok(None),
        code => return Err(os_error(code)),
    }
    let mut text = vec![0u16; (size as usize).div_ceil(2)];
    match query(text.as_mut_ptr().cast(), &mut size) {
        ERROR_SUCCESS => Ok(Some(from_wide(&text))),
        ERROR_FILE_NOT_FOUND => Ok(None),
        code => Err(os_error(code)),
    }
}

// Makes the key too when it is missing.
fn write_string(key: &str, name: &str, value: &str) -> io::Result<()> {
    let (key, name, value) = (wide(key), wide(name), wide(value));
    let bytes = (value.len() * size_of::<u16>()) as u32;
    let status = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            REG_SZ,
            value.as_ptr().cast(),
            bytes,
        )
    };
    match status {
        ERROR_SUCCESS => Ok(()),
        code => Err(os_error(code)),
    }
}

// Nothing to delete is no failure.
fn delete(key: &str, name: &str) -> io::Result<()> {
    let (key, name) = (wide(key), wide(name));
    match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) } {
        ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
        code => Err(os_error(code)),
    }
}

fn os_error(code: WIN32_ERROR) -> io::Error {
    io::Error::from_raw_os_error(code as i32)
}
