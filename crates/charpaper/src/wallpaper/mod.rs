mod attach;
mod cursor;
mod input;
mod monitors;
mod pacing;
mod platform;
mod windows;

use bevy::prelude::*;
use charpaper_scene::ScreenSettings;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;
use charpaper_wallpaper::WindowKey;

pub(crate) use pacing::Paused;
pub use platform::inspect_report;

// Puts a window behind the desktop icons on every chosen monitor and keeps them working there:
// attaching, keeping each on its monitor, replaying the mouse input the desktop keeps from
// them, telling the scene where the cursor is, and resting while nobody can see them.
// Everything OS-specific stays behind the WallpaperBackend that platform.rs picks.
pub struct WallpaperPlugin {
    pub config: WallpaperConfig,
    pub screen: ScreenSettings,
}

impl Plugin for WallpaperPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WallpaperSettings(self.config.clone()))
            .insert_resource(Backend(platform::create_backend()))
            .add_plugins((
                monitors::MonitorsPlugin { screen: self.screen.clone() },
                windows::WindowsPlugin,
                attach::AttachPlugin,
                cursor::CursorPlugin,
                input::InputPlugin,
                pacing::PacingPlugin,
            ));
    }
}

// The backend knows our windows by these: the window entity's bits, so turning one back into
// the other needs no table.
fn key(window: Entity) -> WindowKey {
    WindowKey(window.to_bits())
}

fn window_of(key: WindowKey) -> Option<Entity> {
    Entity::try_from_bits(key.0)
}

// WallpaperConfig lives in a Bevy-free crate, so it can't derive Resource itself.
#[derive(Resource, Deref)]
struct WallpaperSettings(WallpaperConfig);

#[derive(Resource)]
struct Backend(Box<dyn WallpaperBackend>);
