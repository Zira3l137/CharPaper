use bevy::prelude::*;
use charpaper_scene::AntiAliasing;
use charpaper_scene::FpsLimit;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;
use i18n_embed_fl::fl;

use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::UiButton;
use crate::locale::Locale;
use crate::widgets::*;

// How often and how finely the scene is drawn, and when to stop drawing it. Edits
// RenderSettings, which the app saves and applies at once.
pub(crate) struct RenderTabPlugin;

impl Plugin for RenderTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            show_values
                .run_if(resource_changed::<RenderSettings>.or_eager(resource_changed::<Locale>)),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![
        open_section(
            |l| fl!(l, "section-frame-rate"),
            Section::FrameRate,
            cycler(|l| fl!(l, "label-fps-limit"), Cycler::FpsLimit),
        ),
        open_section(
            |l| fl!(l, "section-quality"),
            Section::Quality,
            rows(children![
                cycler(|l| fl!(l, "label-render-scale"), Cycler::RenderScale),
                cycler(|l| fl!(l, "label-anti-aliasing"), Cycler::AntiAliasing),
            ]),
        ),
        open_section(
            |l| fl!(l, "section-pause"),
            Section::Pause,
            rows(children![
                cycler(|l| fl!(l, "label-pause-fullscreen"), Cycler::PauseFullscreen,),
                cycler(|l| fl!(l, "label-pause-covered"), Cycler::PauseCovered),
                cycler(|l| fl!(l, "label-pause-battery"), Cycler::PauseBattery),
            ]),
        ),
    ]
}

fn show_values(
    settings: Res<RenderSettings>,
    locale: Res<Locale>,
    values: Query<(&CyclerValue, &mut Text)>,
) {
    for (value, mut text) in values {
        text.0 = match value.0 {
            Cycler::FpsLimit => match settings.fps_limit.per_second() {
                Some(fps) => format!("{fps:.0}"),
                None => fl!(locale, "value-max").into(),
            },
            Cycler::RenderScale => format!("{}%", settings.scale_percent()),
            Cycler::AntiAliasing => match settings.anti_aliasing {
                AntiAliasing::Off => on_off(&locale, false).into(),
                AntiAliasing::Msaa4 => "4x".into(),
            },
            Cycler::PauseFullscreen => on_off(&locale, settings.pause_when_fullscreen).into(),
            Cycler::PauseCovered => on_off(&locale, settings.pause_when_covered).into(),
            Cycler::PauseBattery => on_off(&locale, settings.pause_on_battery).into(),
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
