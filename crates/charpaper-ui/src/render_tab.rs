//! The Render tab: how often and how finely the scene is drawn, and when to
//! stop drawing it. Edits `RenderSettings`, which the app saves with the rest
//! of its state and applies at once.

use bevy::prelude::*;
use charpaper_scene::AntiAliasing;
use charpaper_scene::FpsLimit;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;

use crate::UiConfig;
use crate::config::UiLocale;
use crate::helpers::Cycler;
use crate::helpers::CyclerValue;
use crate::helpers::Section;
use crate::helpers::UiButton;
use crate::helpers::UiContainer;
use crate::helpers::UiElement;
use crate::pages::step;
use crate::widgets::*;

pub(crate) fn render_page(locale: &UiLocale) -> impl Bundle {
    let text = |key: &str, english: &'static str| locale.get_or(key, english).to_string();
    children![
        open_section(
            locale.get_or("section.frame_rate", "FRAME RATE"),
            UiContainer::Section(Section::FrameRate),
            cycler(text("label.fps_limit", "FPS limit"), Cycler::FpsLimit),
        ),
        open_section(
            locale.get_or("section.quality", "QUALITY"),
            UiContainer::Section(Section::Quality),
            rows(children![
                cycler(text("label.render_scale", "Render scale"), Cycler::RenderScale),
                cycler(text("label.anti_aliasing", "Anti-aliasing"), Cycler::AntiAliasing),
            ]),
        ),
        open_section(
            locale.get_or("section.pause", "PAUSE RENDERING WHEN"),
            UiContainer::Section(Section::Pause),
            rows(children![
                cycler(text("label.pause_fullscreen", "App fullscreen"), Cycler::PauseFullscreen),
                cycler(text("label.pause_covered", "Desktop covered"), Cycler::PauseCovered),
                cycler(text("label.pause_battery", "On battery"), Cycler::PauseBattery),
            ]),
        ),
    ]
}

fn rows(content: impl Bundle) -> impl Bundle {
    (
        Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
        Pickable::IGNORE,
        content,
    )
}

pub(crate) fn show_render_values(
    settings: Res<RenderSettings>,
    config: Res<UiConfig>,
    values: Query<(&CyclerValue, &mut Text)>,
) {
    let locale = &config.locale;
    let on_off = |on: bool| {
        if on { locale.get_or("value.on", "On") } else { locale.get_or("value.off", "Off") }
    };
    for (value, mut text) in values {
        text.0 = match value.0 {
            Cycler::FpsLimit => match settings.fps_limit.per_second() {
                Some(fps) => format!("{fps:.0}"),
                None => locale.get_or("value.max", "Max").into(),
            },
            Cycler::RenderScale => format!("{}%", settings.scale_percent()),
            Cycler::AntiAliasing => match settings.anti_aliasing {
                AntiAliasing::Off => on_off(false).into(),
                AntiAliasing::Msaa4 => "4x".into(),
            },
            Cycler::PauseFullscreen => on_off(settings.pause_when_fullscreen).into(),
            Cycler::PauseCovered => on_off(settings.pause_when_covered).into(),
            Cycler::PauseBattery => on_off(settings.pause_on_battery).into(),
            _ => continue,
        };
    }
}

pub(crate) fn on_render_click(
    event: On<Pointer<Click>>,
    elements: Query<&UiElement>,
    mut settings: ResMut<RenderSettings>,
) {
    let Ok(UiElement::Button(button)) = elements.get(event.entity) else {
        return;
    };
    let (cycler, forward) = match button {
        UiButton::Previous(cycler) => (*cycler, false),
        UiButton::Next(cycler) => (*cycler, true),
        _ => return,
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
