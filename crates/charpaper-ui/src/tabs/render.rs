use bevy::prelude::*;
use charpaper_scene::AntiAliasing;
use charpaper_scene::DepthOfFieldQuality;
use charpaper_scene::Focused;
use charpaper_scene::FocusedMut;
use charpaper_scene::FogQuality;
use charpaper_scene::FogResolution;
use charpaper_scene::FpsLimit;
use charpaper_scene::Graphics;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;
use charpaper_scene::ShadowQuality;
use charpaper_scene::focused_changed;
use i18n_embed_fl::fl;

use crate::UiState;
use crate::elements::Cycler;
use crate::elements::Section;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiContainer;
use crate::locale::Locale;
use crate::panel::new_panel;
use crate::widgets::*;

// How often and how finely the scene is drawn, and when to stop drawing it. How finely is the
// focused stage's Graphics; the rest is RenderSettings, shared by every stage.
pub(crate) struct RenderTabPlugin;

impl Plugin for RenderTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_step).add_systems(
            Update,
            (
                show_values.run_if(
                    resource_changed::<RenderSettings>
                        .or_eager(focused_changed::<Graphics>)
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(new_panel),
                ),
                show_monitor.run_if(
                    resource_changed::<UiState>
                        .or_eager(resource_changed::<Locale>)
                        .or_eager(new_panel),
                ),
            ),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![
        shown_section(
            |l| fl!(l, "section-frame-rate"),
            Section::FrameRate,
            rows(children![
                segments(|l| fl!(l, "label-fps-limit"), Cycler::FpsLimit),
                segments(|l| fl!(l, "label-monitor"), Cycler::Monitor),
            ]),
        ),
        shown_section(
            |l| fl!(l, "section-quality"),
            Section::Quality,
            rows(children![
                segments(|l| fl!(l, "label-render-scale"), Cycler::RenderScale),
                segments(|l| fl!(l, "label-anti-aliasing"), Cycler::AntiAliasing),
                segments(|l| fl!(l, "label-depth-of-field"), Cycler::DepthOfField),
                segments(|l| fl!(l, "label-shadow-quality"), Cycler::ShadowQuality),
            ]),
        ),
        shown_section(
            |l| fl!(l, "section-fog"),
            Section::Fog,
            rows(children![
                segments(|l| fl!(l, "label-fog"), Cycler::Fog),
                picker(|l| fl!(l, "label-fog-quality"), Cycler::FogQuality),
                segments(|l| fl!(l, "label-fog-dithering"), Cycler::FogDithering),
                segments(|l| fl!(l, "label-fog-resolution"), Cycler::FogResolution),
            ]),
        ),
        shown_section(
            |l| fl!(l, "section-pause"),
            Section::Pause,
            rows(children![
                segments(|l| fl!(l, "label-pause-fullscreen"), Cycler::PauseFullscreen,),
                segments(|l| fl!(l, "label-pause-covered"), Cycler::PauseCovered),
                segments(|l| fl!(l, "label-pause-battery"), Cycler::PauseBattery),
            ]),
        ),
    ]
}

fn show_values(
    settings: Res<RenderSettings>,
    graphics: Focused<Graphics>,
    locale: Res<Locale>,
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    let Some(graphics) = graphics.get() else {
        return;
    };
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
                choice(&RENDER_SCALES, &graphics.scale_percent(), |scale| format!("{scale}%"))
            }
            Cycler::AntiAliasing => {
                choice(&AntiAliasing::ALL, &graphics.anti_aliasing, |aa| match aa {
                    AntiAliasing::Off => off.clone(),
                    AntiAliasing::Msaa4 => "4x".into(),
                    AntiAliasing::Taa => "TAA".into(),
                })
            }
            Cycler::DepthOfField => {
                choice(&DepthOfFieldQuality::ALL, &graphics.depth_of_field, |quality| match quality
                {
                    DepthOfFieldQuality::Off => off.clone(),
                    DepthOfFieldQuality::Blur => fl!(locale, "value-blur"),
                    DepthOfFieldQuality::Bokeh => fl!(locale, "value-bokeh"),
                })
            }
            Cycler::ShadowQuality => {
                choice(&ShadowQuality::ALL, &settings.shadow_quality, |quality| match quality {
                    ShadowQuality::Low => fl!(locale, "value-low"),
                    ShadowQuality::Medium => fl!(locale, "value-medium"),
                    ShadowQuality::High => fl!(locale, "value-high"),
                })
            }
            Cycler::Fog => switch(&locale, graphics.fog),
            Cycler::FogQuality => {
                choice(&FogQuality::ALL, &graphics.fog_quality, |quality| match quality {
                    FogQuality::VeryLow => fl!(locale, "value-very-low"),
                    FogQuality::Low => fl!(locale, "value-low"),
                    FogQuality::Medium => fl!(locale, "value-medium"),
                    FogQuality::High => fl!(locale, "value-high"),
                    FogQuality::Ultra => fl!(locale, "value-ultra"),
                })
            }
            Cycler::FogDithering => switch(&locale, graphics.fog_dithering),
            Cycler::FogResolution => {
                choice(&FogResolution::ALL, &graphics.fog_resolution, |resolution| match resolution
                {
                    FogResolution::Full => fl!(locale, "value-full"),
                    FogResolution::Half => fl!(locale, "value-half"),
                })
            }
            Cycler::PauseFullscreen => switch(&locale, settings.pause_when_fullscreen),
            Cycler::PauseCovered => switch(&locale, settings.pause_when_covered),
            Cycler::PauseBattery => switch(&locale, settings.pause_on_battery),
            _ => continue,
        };
        shown.set_if_neq(next);
    }
}

fn show_monitor(
    ui: Res<UiState>,
    locale: Res<Locale>,
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    show(&mut rows, Cycler::Monitor, switch(&locale, ui.performance_monitor));
}

fn on_step(
    event: On<Step>,
    mut settings: ResMut<RenderSettings>,
    mut graphics: FocusedMut<Graphics>,
    mut ui: ResMut<UiState>,
) {
    let Some(mut graphics) = graphics.get_mut() else {
        return;
    };
    match event.cycler {
        Cycler::Monitor => ui.performance_monitor = flip(ui.performance_monitor, &event),
        Cycler::FpsLimit => {
            if let Some(next) = step(&FpsLimit::ALL, &settings.fps_limit, &event) {
                settings.fps_limit = next;
            }
        }
        Cycler::RenderScale => {
            if let Some(next) = step(&RENDER_SCALES, &graphics.scale_percent(), &event) {
                graphics.render_scale = next;
            }
        }
        Cycler::AntiAliasing => {
            if let Some(next) = step(&AntiAliasing::ALL, &graphics.anti_aliasing, &event) {
                graphics.anti_aliasing = next;
            }
        }
        Cycler::DepthOfField => {
            if let Some(next) = step(&DepthOfFieldQuality::ALL, &graphics.depth_of_field, &event) {
                graphics.depth_of_field = next;
            }
        }
        Cycler::ShadowQuality => {
            if let Some(next) = step(&ShadowQuality::ALL, &settings.shadow_quality, &event) {
                settings.shadow_quality = next;
            }
        }
        Cycler::Fog => graphics.fog = flip(graphics.fog, &event),
        Cycler::FogQuality => {
            if let Some(next) = step(&FogQuality::ALL, &graphics.fog_quality, &event) {
                graphics.fog_quality = next;
            }
        }
        Cycler::FogDithering => graphics.fog_dithering = flip(graphics.fog_dithering, &event),
        Cycler::FogResolution => {
            if let Some(next) = step(&FogResolution::ALL, &graphics.fog_resolution, &event) {
                graphics.fog_resolution = next;
            }
        }
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
