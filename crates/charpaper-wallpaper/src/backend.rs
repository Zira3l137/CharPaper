use raw_window_handle::RawWindowHandle;

use crate::AttachStrategy;
use crate::config::WallpaperConfig;
use crate::error::WallpaperError;
use crate::input::PointerSource;
use crate::input::Wake;

#[derive(Debug, Default)]
pub struct DesktopProbe {
    pub recommended: Option<AttachStrategy>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DesktopActivity {
    // Nobody is at the screen: the session is locked, the screen saver runs, or another user
    // is signed in. Always a reason to pause, with no setting.
    pub away: bool,
    pub fullscreen_app: bool,
    pub covered: bool,
    pub on_battery: bool,
}

// A rectangle in physical pixels on the plane the OS lays every monitor out on. Coordinates
// can be negative: Windows puts the primary monitor's corner at (0, 0), so a monitor left of
// it starts below zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesktopMonitor {
    // Stays the same across reboots and driver updates, so a choice saved by it finds its
    // monitor again.
    pub id: String,
    // For people: the model, when the OS knows it.
    pub name: String,
    pub area: ScreenArea,
    // Physical pixels per logical one, as the OS's display scaling sets it.
    pub scale: f64,
    pub primary: bool,
}

#[derive(Debug, Default)]
pub struct AttachOutcome {
    pub strategy_used: Option<AttachStrategy>,
}

// Send + Sync so it fits in a Bevy Resource; callers keep to the main thread themselves.
pub trait WallpaperBackend: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn probe(&mut self, config: &WallpaperConfig) -> Result<DesktopProbe, WallpaperError>;

    // `area` is the part of the screen the window covers, usually one monitor.
    fn attach(
        &mut self,
        handle: RawWindowHandle,
        area: ScreenArea,
        config: &WallpaperConfig,
    ) -> Result<AttachOutcome, WallpaperError>;

    // Moves the attached window over `area`. Does nothing while no window is attached.
    fn place(&mut self, _area: ScreenArea) -> Result<(), WallpaperError> {
        Ok(())
    }

    // Every monitor, freshly read. Polled about once a second. Empty when the backend can't
    // tell, which leaves the app to Bevy's own list: it has no lasting ids and can miss a
    // monitor changing resolution.
    fn monitors(&mut self) -> Vec<DesktopMonitor> {
        Vec::new()
    }

    // Ok(None) means there is nothing to recover: the window gets its input on its own.
    fn forward_input(
        &mut self,
        _wake: Wake,
    ) -> Result<Option<Box<dyn PointerSource>>, WallpaperError> {
        Ok(None)
    }

    // Polled about once a second, so keep it cheap.
    fn activity(&mut self) -> DesktopActivity {
        DesktopActivity::default()
    }

    // Where the cursor is, wherever it is on screen, in physical pixels from the attached
    // window's top-left corner. Polled every frame. None when unknown, which leaves the app to
    // Bevy's own cursor position, known only while the cursor is over the window.
    fn cursor_position(&mut self) -> Option<[f32; 2]> {
        None
    }
}
