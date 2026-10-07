use bevy::prelude::*;
use charpaper_scene::AntiAliasing;
use charpaper_scene::DepthOfFieldQuality;
use charpaper_scene::FogQuality;
use charpaper_scene::FpsLimit;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;
use i18n_embed_fl::fl;

use crate::elements::Cycler;
use crate::elements::Section;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiContainer;
use crate::locale::Locale;
use crate::widgets::*;

// How often and how finely the scene is drawn, and when to stop drawing it. Edits
// RenderSettings, which the app saves and applies at once.
pub(crate) struct RenderTabPlugin;

impl Plugin for RenderTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_step).add_systems(
            Update,
            show_values
                .run_if(resource_changed::<RenderSettings>.or_eager(resource_changed::<Locale>)),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![
        shown_section(
            |l| fl!(l, "section-frame-rate"),
            Section::FrameRate,
            cycler(|l| fl!(l, "label-fps-limit"), Cycler::FpsLimit),
        ),
        shown_section(
            |l| fl!(l, "section-quality"),
            Section::Quality,
            rows(children![
                cycler(|l| fl!(l, "label-render-scale"), Cycler::RenderScale),
                cycler(|l| fl!(l, "label-anti-aliasing"), Cycler::AntiAliasing),
                cycler(|l| fl!(l, "label-depth-of-field"), Cycler::DepthOfField),
            ]),
        ),
        shown_section(
            |l| fl!(l, "section-fog"),
            Section::Fog,
            rows(children![
                cycler(|l| fl!(l, "label-fog"), Cycler::Fog),
                cycler(|l| fl!(l, "label-fog-quality"), Cycler::FogQuality),
                cycler(|l| fl!(l, "label-fog-dithering"), Cycler::FogDithering),
            ]),
        ),
        shown_section(
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
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    let off = on_off(&locale, false);
    for (container, mut shown) in &mut rows {
        let UiContainer::Row(cycler) = *container else {
            continue;
        };
        let next = match cycler {
            Cycler::FpsLimit => {
                choice(&FpsLimit::ALL, &settings.fps_limit, |limit| match limit.per_second() {
                    Some(fps) => format!("{fps:.0}"),
                    None => fl!(locale, "value-max"),
                })
            }
            Cycler::RenderScale => {
                choice(&RENDER_SCALES, &settings.scale_percent(), |scale| format!("{scale}%"))
            }
            Cycler::AntiAliasing => {
                choice(&AntiAliasing::ALL, &settings.anti_aliasing, |aa| match aa {
                    AntiAliasing::Off => off.clone(),
                    AntiAliasing::Msaa4 => "4x".into(),
                    AntiAliasing::Taa => "TAA".into(),
                })
            }
            Cycler::DepthOfField => {
                choice(&DepthOfFieldQuality::ALL, &settings.depth_of_field, |quality| match quality
                {
                    DepthOfFieldQuality::Off => off.clone(),
                    DepthOfFieldQuality::Blur => fl!(locale, "value-blur"),
                    DepthOfFieldQuality::Bokeh => fl!(locale, "value-bokeh"),
                })
            }
            Cycler::Fog => switch(&locale, settings.fog),
            Cycler::FogQuality => {
                choice(&FogQuality::ALL, &settings.fog_quality, |quality| match quality {
                    FogQuality::VeryLow => fl!(locale, "value-very-low"),
                    FogQuality::Low => fl!(locale, "value-low"),
                    FogQuality::Medium => fl!(locale, "value-medium"),
                    FogQuality::High => fl!(locale, "value-high"),
                    FogQuality::Ultra => fl!(locale, "value-ultra"),
                })
            }
            Cycler::FogDithering => switch(&locale, settings.fog_dithering),
            Cycler::PauseFullscreen => switch(&locale, settings.pause_when_fullscreen),
            Cycler::PauseCovered => switch(&locale, settings.pause_when_covered),
            Cycler::PauseBattery => switch(&locale, settings.pause_on_battery),
            _ => continue,
        };
        shown.set_if_neq(next);
    }
}

fn on_step(event: On<Step>, mut settings: ResMut<RenderSettings>) {
    match event.cycler {
        Cycler::FpsLimit => {
            if let Some(next) = step(&FpsLimit::ALL, &settings.fps_limit, &event) {
                settings.fps_limit = next;
            }
        }
        Cycler::RenderScale => {
            if let Some(next) = step(&RENDER_SCALES, &settings.scale_percent(), &event) {
                settings.render_scale = next;
            }
        }
        Cycler::AntiAliasing => {
            if let Some(next) = step(&AntiAliasing::ALL, &settings.anti_aliasing, &event) {
                settings.anti_aliasing = next;
            }
        }
        Cycler::DepthOfField => {
            if let Some(next) = step(&DepthOfFieldQuality::ALL, &settings.depth_of_field, &event) {
                settings.depth_of_field = next;
            }
        }
        Cycler::Fog => settings.fog = flip(settings.fog, &event),
        Cycler::FogQuality => {
            if let Some(next) = step(&FogQuality::ALL, &settings.fog_quality, &event) {
                settings.fog_quality = next;
            }
        }
        Cycler::FogDithering => settings.fog_dithering = flip(settings.fog_dithering, &event),
        Cycler::PauseFullscreen => {
            settings.pause_when_fullscreen = flip(settings.pause_when_fullscreen, &event)
        }
        Cycler::PauseCovered => {
            settings.pause_when_covered = flip(settings.pause_when_covered, &event)
        }
        Cycler::PauseBattery => settings.pause_on_battery = flip(settings.pause_on_battery, &event),
        _ => {}
    }
}
