use bevy::prelude::*;
use charpaper_scene::AntiAliasing;
use charpaper_scene::DepthOfFieldQuality;
use charpaper_scene::Focused;
use charpaper_scene::FocusedMut;
use charpaper_scene::FogQuality;
use charpaper_scene::FogResolution;
use charpaper_scene::Graphics;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;
use charpaper_scene::ShadowQuality;
use charpaper_scene::focused_changed;
use i18n_embed_fl::fl;

use crate::elements::Cycler;
use crate::elements::Section;
use crate::elements::Shown;
use crate::elements::Step;
use crate::elements::UiContainer;
use crate::locale::Locale;
use crate::panel::new_panel;
use crate::widgets::*;

// How finely the focused stage is drawn, in the Graphics tab. Edits its Graphics, which the
// app saves with the stage. Shadow quality is the exception: every stage shares it.
pub(crate) struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_step).add_systems(
            Update,
            show_values.run_if(
                focused_changed::<Graphics>
                    .or_eager(resource_changed::<RenderSettings>)
                    .or_eager(resource_changed::<Locale>)
                    .or_eager(new_panel),
            ),
        );
    }
}

pub(crate) fn quality() -> impl Bundle {
    shown_section(
        |l| fl!(l, "section-quality"),
        Section::Quality,
        rows(children![
            segments(|l| fl!(l, "label-render-scale"), Cycler::RenderScale),
            segments(|l| fl!(l, "label-anti-aliasing"), Cycler::AntiAliasing),
            segments(|l| fl!(l, "label-depth-of-field"), Cycler::DepthOfField),
            segments(|l| fl!(l, "label-shadow-quality"), Cycler::ShadowQuality),
        ]),
    )
}

pub(crate) fn fog() -> impl Bundle {
    shown_section(
        |l| fl!(l, "section-fog"),
        Section::Fog,
        rows(children![
            segments(|l| fl!(l, "label-fog"), Cycler::Fog),
            picker(|l| fl!(l, "label-fog-quality"), Cycler::FogQuality),
            segments(|l| fl!(l, "label-fog-dithering"), Cycler::FogDithering),
            segments(|l| fl!(l, "label-fog-resolution"), Cycler::FogResolution),
        ]),
    )
}

fn show_values(
    graphics: Focused<Graphics>,
    settings: Res<RenderSettings>,
    locale: Res<Locale>,
    mut rows: Query<(&UiContainer, &mut Shown)>,
) {
    let shadows = choice(&ShadowQuality::ALL, &settings.shadow_quality, |quality| match quality {
        ShadowQuality::Low => fl!(locale, "value-low"),
        ShadowQuality::Medium => fl!(locale, "value-medium"),
        ShadowQuality::High => fl!(locale, "value-high"),
    });
    show(&mut rows, Cycler::ShadowQuality, shadows);
    let Some(graphics) = graphics.get() else {
        return;
    };
    let off = on_off(&locale, false);
    for (container, mut shown) in &mut rows {
        let UiContainer::Row(cycler) = *container else {
            continue;
        };
        let next = match cycler {
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
            _ => continue,
        };
        shown.set_if_neq(next);
    }
}

fn on_step(
    event: On<Step>,
    mut graphics: FocusedMut<Graphics>,
    mut settings: ResMut<RenderSettings>,
) {
    if event.cycler == Cycler::ShadowQuality {
        if let Some(next) = step(&ShadowQuality::ALL, &settings.shadow_quality, &event) {
            settings.shadow_quality = next;
        }
        return;
    }
    let Some(mut graphics) = graphics.get_mut() else {
        return;
    };
    match event.cycler {
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
        _ => {}
    }
}
