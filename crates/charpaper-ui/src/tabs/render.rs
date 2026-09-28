use bevy::prelude::*;
use charpaper_scene::AntiAliasing;
use charpaper_scene::FpsLimit;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;

use crate::UiConfig;
use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::UiButton;
use crate::locale::UiLocale;
use crate::widgets::*;

// How often and how finely the scene is drawn, and when to stop drawing it. Edits
// RenderSettings, which the app saves and applies at once.
pub(crate) struct RenderTabPlugin;

impl Plugin for RenderTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click)
            .add_systems(Update, show_values.run_if(resource_changed::<RenderSettings>));
    }
}

pub(crate) fn page(locale: &UiLocale) -> impl Bundle {
    children![
        open_section(
            locale.get_or("section.frame_rate", "FRAME RATE"),
            Section::FrameRate,
            cycler(locale.get_or("label.fps_limit", "FPS limit"), Cycler::FpsLimit),
        ),
        open_section(
            locale.get_or("section.quality", "QUALITY"),
            Section::Quality,
            rows(children![
                cycler(locale.get_or("label.render_scale", "Render scale"), Cycler::RenderScale),
                cycler(locale.get_or("label.anti_aliasing", "Anti-aliasing"), Cycler::AntiAliasing),
            ]),
        ),
        open_section(
            locale.get_or("section.pause", "PAUSE RENDERING WHEN"),
            Section::Pause,
            rows(children![
                cycler(
                    locale.get_or("label.pause_fullscreen", "App fullscreen"),
                    Cycler::PauseFullscreen,
                ),
                cycler(locale.get_or("label.pause_covered", "Desktop covered"), Cycler::PauseCovered),
                cycler(locale.get_or("label.pause_battery", "On battery"), Cycler::PauseBattery),
            ]),
        ),
    ]
}

fn show_values(
    settings: Res<RenderSettings>,
    config: Res<UiConfig>,
    values: Query<(&CyclerValue, &mut Text)>,
) {
    let locale = &config.locale;
    for (value, mut text) in values {
        text.0 = match value.0 {
            Cycler::FpsLimit => match settings.fps_limit.per_second() {
                Some(fps) => format!("{fps:.0}"),
                None => locale.get_or("value.max", "Max").into(),
            },
            Cycler::RenderScale => format!("{}%", settings.scale_percent()),
            Cycler::AntiAliasing => match settings.anti_aliasing {
                AntiAliasing::Off => on_off(locale, false).into(),
                AntiAliasing::Msaa4 => "4x".into(),
            },
            Cycler::PauseFullscreen => on_off(locale, settings.pause_when_fullscreen).into(),
            Cycler::PauseCovered => on_off(locale, settings.pause_when_covered).into(),
            Cycler::PauseBattery => on_off(locale, settings.pause_on_battery).into(),
            _ => continue,
        };
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    mut settings: ResMut<RenderSettings>,
) {
    let Some((cycler, forward)) = buttons.get(event.entity).ok().and_then(UiButton::step) else {
        return;
    };
    match cycler {
        Cycler::FpsLimit => {
            if let Some(next) = step(&FpsLimit::ALL, &settings.fps_limit, forward) {
                settings.fps_limit = next;
            }
        }
        Cycler::RenderScale => {
            if let Some(next) = step(&RENDER_SCALES, &settings.scale_percent(), forward) {
                settings.render_scale = next;
            }
        }
        Cycler::AntiAliasing => {
            if let Some(next) = step(&AntiAliasing::ALL, &settings.anti_aliasing, forward) {
                settings.anti_aliasing = next;
            }
        }
        Cycler::PauseFullscreen => settings.pause_when_fullscreen ^= true,
        Cycler::PauseCovered => settings.pause_when_covered ^= true,
        Cycler::PauseBattery => settings.pause_on_battery ^= true,
        _ => {}
    }
}
