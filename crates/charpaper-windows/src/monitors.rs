use std::collections::HashMap;

use charpaper_wallpaper::DesktopMonitor;
use charpaper_wallpaper::ScreenArea;

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

fn describe(monitor: sys::HMonitor, names: &HashMap<String, Names>) -> Option<DesktopMonitor> {
    let info = sys::monitor_info_ex(monitor)?;
    let device = sys::from_wide(&info.device);
    let rect = info.info.rc_monitor;
    let names = names.get(&device);

    let id = names.map(|n| n.path.clone()).filter(|path| !path.is_empty());
    let name = names.map(|n| n.friendly.clone()).filter(|name| !name.is_empty());
    Some(DesktopMonitor {
        name: name.unwrap_or_else(|| device.trim_start_matches(GDI_PREFIX).to_string()),
        id: id.unwrap_or(device),
        area: ScreenArea {
            x: rect.left,
            y: rect.top,
            width: rect.width() as u32,
            height: rect.height() as u32,
        },
        scale: sys::monitor_dpi(monitor).map_or(1.0, |dpi| f64::from(dpi) / 96.0),
        primary: info.info.flags & sys::MONITORINFOF_PRIMARY != 0,
    })
}

// Windows names a monitor in two places. GetMonitorInfo gives its rectangle but calls it only
// \\.\DISPLAY1, whose number can change from one boot to the next. The display configuration
// knows the model's name and a device path that stays put, and links them to that DISPLAY1.
fn display_names() -> HashMap<String, Names> {
    let mut out = HashMap::new();
    for path in active_paths().unwrap_or_default() {
        let source = &path.source;
        let target = &path.target;
        let gdi = device_info::<sys::SourceDeviceName>(
            sys::DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            source.adapter_id,
            source.id,
        );
        let monitor = device_info::<sys::TargetDeviceName>(
            sys::DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            target.adapter_id,
            target.id,
        );
        let (Some(gdi), Some(monitor)) = (gdi, monitor) else {
            continue;
        };
        let names = Names {
            friendly: sys::from_wide(&monitor.friendly_name),
            path: sys::from_wide(&monitor.device_path),
        };
        // Mirrored monitors share one DISPLAY; the first of them names it.
        out.entry(sys::from_wide(&gdi.gdi_device_name)).or_insert(names);
    }
    out
}

fn active_paths() -> Option<Vec<sys::PathInfo>> {
    // A monitor plugged in between asking for the sizes and the query makes the buffers too
    // small. Asking again is the documented answer.
    for _ in 0..3 {
        let (mut path_count, mut mode_count) = (0u32, 0u32);
        let flags = sys::QDC_ONLY_ACTIVE_PATHS;
        if unsafe { sys::GetDisplayConfigBufferSizes(flags, &mut path_count, &mut mode_count) } != 0
        {
            return None;
        }
        let mut paths = vec![sys::PathInfo::default(); path_count as usize];
        let mut modes = vec![sys::ModeInfo::default(); mode_count as usize];
        let result = unsafe {
            sys::QueryDisplayConfig(
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
            sys::ERROR_INSUFFICIENT_BUFFER => continue,
            _ => return None,
        }
    }
    None
}

// Every DISPLAYCONFIG_*_NAME request starts with the same header saying what is asked and
// how big the struct is; Windows fills in the rest.
trait DeviceInfo: Copy {}
impl DeviceInfo for sys::SourceDeviceName {}
impl DeviceInfo for sys::TargetDeviceName {}

fn device_info<T: DeviceInfo>(kind: u32, adapter_id: sys::Luid, id: u32) -> Option<T> {
    // All zeroes is a valid value of both: they hold only numbers and arrays of them.
    let mut request: T = unsafe { std::mem::zeroed() };
    let header = &mut request as *mut T as *mut sys::DeviceInfoHeader;
    let size = std::mem::size_of::<T>() as u32;
    unsafe { header.write(sys::DeviceInfoHeader { kind, size, adapter_id, id }) };
    let ok = unsafe { sys::DisplayConfigGetDeviceInfo(header) } == 0;
    ok.then_some(request)
}
