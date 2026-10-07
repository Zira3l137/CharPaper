use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use crate::elements::Cycler;
use crate::elements::HelpText;
use crate::elements::UiContainer;
use crate::locale::Locale;

// A sentence at the bottom of the panel about the row under the pointer. With none under it,
// how the panel works, which is the only place the wheel is mentioned.
pub(crate) struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show_help);
    }
}

fn show_help(
    locale: Res<Locale>,
    rows: Query<(&UiContainer, Ref<Hovered>)>,
    mut lines: Query<&mut Text, With<HelpText>>,
) {
    if !locale.is_changed() && !rows.iter().any(|(_, hovered)| hovered.is_changed()) {
        return;
    }
    let hovered = rows.iter().find_map(|(container, hovered)| match container {
        UiContainer::Row(cycler) if hovered.get() => Some(*cycler),
        _ => None,
    });
    let text = match hovered {
        Some(cycler) => help(cycler, &locale),
        None => fl!(locale, "help-panel"),
    };
    for mut line in &mut lines {
        line.0.clone_from(&text);
    }
}

fn help(cycler: Cycler, l: &FluentLanguageLoader) -> String {
    match cycler {
        Cycler::Suite => fl!(l, "help-suite"),
        Cycler::Animation => fl!(l, "help-animation"),
        Cycler::Expression => fl!(l, "help-expression"),
        Cycler::FollowCursor => fl!(l, "help-follow-cursor"),
        Cycler::Camera => fl!(l, "help-camera"),
        Cycler::Environment => fl!(l, "help-environment"),
        Cycler::Brightness => fl!(l, "help-brightness"),
        Cycler::Shadows => fl!(l, "help-shadows"),
        Cycler::Tonemapping => fl!(l, "help-tonemapping"),
        Cycler::Exposure => fl!(l, "help-exposure"),
        Cycler::Bloom => fl!(l, "help-bloom"),
        Cycler::ChromaticAberration => fl!(l, "help-chromatic-aberration"),
        Cycler::Vignette => fl!(l, "help-vignette"),
        Cycler::VignetteSize => fl!(l, "help-vignette-size"),
        Cycler::Grain => fl!(l, "help-grain"),
        Cycler::GrainSize => fl!(l, "help-grain-size"),
        Cycler::Warmth => fl!(l, "help-warmth"),
        Cycler::Tint => fl!(l, "help-tint"),
        Cycler::Saturation => fl!(l, "help-saturation"),
        Cycler::Contrast => fl!(l, "help-contrast"),
        Cycler::Lut => fl!(l, "help-lut"),
        Cycler::LutStrength => fl!(l, "help-lut-strength"),
        Cycler::FpsLimit => fl!(l, "help-fps-limit"),
        Cycler::RenderScale => fl!(l, "help-render-scale"),
        Cycler::AntiAliasing => fl!(l, "help-anti-aliasing"),
        Cycler::DepthOfField => fl!(l, "help-depth-of-field"),
        Cycler::Fog => fl!(l, "help-fog"),
        Cycler::FogQuality => fl!(l, "help-fog-quality"),
        Cycler::FogDithering => fl!(l, "help-fog-dithering"),
        Cycler::PauseFullscreen => fl!(l, "help-pause-fullscreen"),
        Cycler::PauseCovered => fl!(l, "help-pause-covered"),
        Cycler::PauseBattery => fl!(l, "help-pause-battery"),
        Cycler::Language => fl!(l, "help-language"),
    }
}
