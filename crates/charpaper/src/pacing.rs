use std::time::Duration;

use bevy::prelude::*;
use bevy::winit::UpdateMode;
use bevy::winit::WinitSettings;
use charpaper_audio::Hush;
use charpaper_settings::RenderSettings;
use charpaper_ui::PanelHovered;

use crate::wallpaper::Paused;
use crate::wallpaper::ReadActivity;

const PAUSED_WAKE: Duration = Duration::from_secs(1);
// While the pointer is on the panel, whatever the frame limit and even while paused, so a
// click or a wheel turn shows at once.
const PANEL_FPS: f64 = 60.0;

// How often the app draws, and whether it plays sound, given the frame limit, the panel and
// whether the desktop says to rest.
pub struct PacingPlugin;

impl Plugin for PacingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_update_mode.run_if(
                    resource_changed::<RenderSettings>
                        .or_eager(resource_changed::<Paused>)
                        .or_eager(resource_changed::<PanelHovered>),
                ),
                hush_while_paused.run_if(resource_changed::<Paused>),
            )
                .after(ReadActivity),
        );
    }
}

// Whatever stops the drawing stops the sound too.
fn hush_while_paused(paused: Res<Paused>, mut hush: ResMut<Hush>) {
    hush.set_if_neq(Hush(paused.0.is_some()));
}

// In reactive mode winit sleeps until the wait runs out or an event arrives, so between
// frames nothing runs and the GPU draws nothing.
fn apply_update_mode(
    settings: Res<RenderSettings>,
    paused: Res<Paused>,
    panel: Res<PanelHovered>,
    mut winit: ResMut<WinitSettings>,
) {
    let every = |fps: f64| UpdateMode::reactive_low_power(Duration::from_secs_f64(1.0 / fps));
    let mode = match (paused.0, settings.fps_limit.per_second()) {
        (_, Some(fps)) if panel.0 => every(fps.max(PANEL_FPS)),
        (_, None) if panel.0 => UpdateMode::Continuous,
        (Some(_), _) => UpdateMode::reactive_low_power(PAUSED_WAKE),
        (None, Some(fps)) => every(fps),
        (None, None) => UpdateMode::Continuous,
    };
    winit.focused_mode = mode;
    winit.unfocused_mode = mode;
}
