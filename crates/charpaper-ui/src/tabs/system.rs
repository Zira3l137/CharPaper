use bevy::prelude::*;
use i18n_embed_fl::fl;

use crate::UiState;
use crate::elements::Cycler;
use crate::elements::CyclerValue;
use crate::elements::Section;
use crate::elements::UiButton;
use crate::locale::Languages;
use crate::locale::Locale;
use crate::widgets::*;

// Settings of the app itself rather than of what it shows. For now, the panel's language.
pub(crate) struct SystemTabPlugin;

impl Plugin for SystemTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            show_language.run_if(resource_changed::<UiState>.or_eager(resource_changed::<Locale>)),
        );
    }
}

pub(crate) fn page() -> impl Bundle {
    children![open_section(
        |l| fl!(l, "section-interface"),
        Section::Interface,
        cycler(|l| fl!(l, "label-language"), Cycler::Language),
    )]
}

fn show_language(
    ui: Res<UiState>,
    locale: Res<Locale>,
    languages: Res<Languages>,
    mut values: Query<(&CyclerValue, &mut Text)>,
) {
    let text = match &ui.language {
        None => fl!(locale, "language-system"),
        Some(tag) => languages.name(tag).unwrap_or(tag).to_string(),
    };
    set_value(&mut values, Cycler::Language, &text);
}

// None, following the system's languages, comes first.
fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<&UiButton>,
    languages: Res<Languages>,
    mut ui: ResMut<UiState>,
) {
    let Some((Cycler::Language, forward)) = buttons.get(event.entity).ok().and_then(UiButton::step)
    else {
        return;
    };
    let options: Vec<Option<String>> =
        std::iter::once(None).chain(languages.0.iter().map(|l| Some(l.tag.clone()))).collect();
    if let Some(next) = step(&options, &ui.language, forward) {
        ui.language = next;
    }
}
