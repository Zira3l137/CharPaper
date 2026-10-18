use std::collections::HashMap;

use charpaper_wallpaper::DesktopMonitor;
use charpaper_wallpaper::ScreenArea;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_HEADER;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_TYPE;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_MODE_INFO;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_PATH_INFO;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_SOURCE_DEVICE_NAME;
use windows_sys::Win32::Devices::Display::DISPLAYCONFIG_TARGET_DEVICE_NAME;
use windows_sys::Win32::Devices::Display::DisplayConfigGetDeviceInfo;
use windows_sys::Win32::Devices::Display::GetDisplayConfigBufferSizes;
use windows_sys::Win32::Devices::Display::QDC_ONLY_ACTIVE_PATHS;
use windows_sys::Win32::Devices::Display::QueryDisplayConfig;
use windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows_sys::Win32::Foundation::LUID;
use windows_sys::Win32::Graphics::Gdi::HMONITOR;
use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

use crate::sys;

const GDI_PREFIX: &str = r"\\.\";

struct Names {
    friendly: String,
    path: String,
}

pub fn monitors() -> Vec<DesktopMonitor> {
    let names = display_names();
    sys::display_monitors().into_iter().filter_map(|monitor| describe(monitor, &names)).collect()
}

fn describe(monitor: HMONITOR, names: &HashMap<String, Names>) -> Option<DesktopMonitor> {
    let info = sys::monitor_info_ex(monitor)?;
    let device = sys::from_wide(&info.szDevice);
    let rect = info.monitorInfo.rcMonitor;
    let names = names.get(&device);

    let id = names.map(|n| n.path.clone()).filter(|path| !path.is_empty());
    let name = names.map(|n| n.friendly.clone()).filter(|name| !name.is_empty());
    Some(DesktopMonitor {
        name: name.unwrap_or_else(|| device.trim_start_matches(GDI_PREFIX).to_string()),
        id: id.unwrap_or(device),
        area: ScreenArea {
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        },
        scale: sys::monitor_dpi(monitor).map_or(1.0, |dpi| f64::from(dpi) / 96.0),
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    })
}

// Windows names a monitor in two places. GetMonitorInfo gives its rectangle but calls it only
// \\.\DISPLAY1, whose number can change from one boot to the next. The display configuration
// knows the model's name and a device path that stays put, and links them to that DISPLAY1.
fn display_names() -> HashMap<String, Names> {
    let mut out = HashMap::new();
    for path in active_paths().unwrap_or_default() {
        let (source, target) = (&path.sourceInfo, &path.targetInfo);
        let gdi = device_info::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>(
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            source.adapterId,
            source.id,
        );
        let monitor = device_info::<DISPLAYCONFIG_TARGET_DEVICE_NAME>(
            DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            target.adapterId,
            target.id,
        );
        let (Some(gdi), Some(monitor)) = (gdi, monitor) else {
            continue;
        };
        let names = Names {
            friendly: sys::from_wide(&monitor.monitorFriendlyDeviceName),
            path: sys::from_wide(&monitor.monitorDevicePath),
        };
        // Mirrored monitors share one DISPLAY; the first of them names it.
        out.entry(sys::from_wide(&gdi.viewGdiDeviceName)).or_insert(names);
    }
    out
}

fn active_paths() -> Option<Vec<DISPLAYCONFIG_PATH_INFO>> {
    // A monitor plugged in between asking for the sizes and the query makes the buffers too
    // small. Asking again is the documented answer.
    for _ in 0..3 {
        let (mut path_count, mut mode_count) = (0u32, 0u32);
        let flags = QDC_ONLY_ACTIVE_PATHS;
        if unsafe { GetDisplayConfigBufferSizes(flags, &mut path_count, &mut mode_count) } != 0 {
            return None;
        }
        // SAFETY: all zeroes is a valid value of both, which hold only numbers and unions of them.
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            vec![unsafe { std::mem::zeroed() }; path_count as usize];
        let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
            vec![unsafe { std::mem::zeroed() }; mode_count as usize];
        let result = unsafe {
            QueryDisplayConfig(
                flags,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        match result {
            0 => {
                paths.truncate(path_count as usize);
                return Some(paths);
            }
            ERROR_INSUFFICIENT_BUFFER => continue,
            _ => return None,
        }
    }
    None
}

// Every DISPLAYCONFIG_*_NAME request starts with the same header saying what is asked and
// how big the struct is; Windows fills in the rest.
trait DeviceInfo: Copy {}
impl DeviceInfo for DISPLAYCONFIG_SOURCE_DEVICE_NAME {}
impl DeviceInfo for DISPLAYCONFIG_TARGET_DEVICE_NAME {}

fn device_info<T: DeviceInfo>(
    kind: DISPLAYCONFIG_DEVICE_INFO_TYPE,
    adapter_id: LUID,
    id: u32,
) -> Option<T> {
    // SAFETY: all zeroes is a valid value of both: they hold only numbers and arrays of them.
    let mut request: T = unsafe { std::mem::zeroed() };
    let header = &mut request as *mut T as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER;
    let size = size_of::<T>() as u32;
    let filled = DISPLAYCONFIG_DEVICE_INFO_HEADER { r#type: kind, size, adapterId: adapter_id, id };
    unsafe { header.write(filled) };
    let ok = unsafe { DisplayConfigGetDeviceInfo(header) } == 0;
    ok.then_some(request)
}
