mod attach;
mod cursor;
mod input;
mod pacing;
mod platform;

use bevy::prelude::*;
use charpaper_wallpaper::WallpaperBackend;
use charpaper_wallpaper::WallpaperConfig;

pub use platform::inspect_report;

// Puts the window behind the desktop icons and keeps it working there: attaching, replaying
// the mouse input the desktop keeps from it, telling the scene where the cursor is, and
// resting while nobody can see it. Everything
// OS-specific stays behind the WallpaperBackend that platform.rs picks.
pub struct WallpaperPlugin {
    pub config: WallpaperConfig,
}

impl Plugin for WallpaperPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WallpaperSettings(self.config.clone()))
            .insert_resource(Backend(platform::create_backend()))
            .add_plugins((
                attach::AttachPlugin,
                cursor::CursorPlugin,
                input::InputPlugin,
                pacing::PacingPlugin,
            ));
    }
}

// WallpaperConfig lives in a Bevy-free crate, so it can't derive Resource itself.
#[derive(Resource, Deref)]
struct WallpaperSettings(WallpaperConfig);

#[derive(Resource)]
struct Backend(Box<dyn WallpaperBackend>);
