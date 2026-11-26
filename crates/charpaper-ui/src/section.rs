use bevy::prelude::*;
use bevy::text::Justify;
use bevy::text::LineBreak;
use charpaper_settings::UiState;
use i18n_embed::fluent::FluentLanguageLoader;

use crate::icons::Icon;
use crate::icons::icon;
use crate::locale::Locale;
use crate::locale::Tr;
use crate::panel::new_panel;
use crate::row::DrawRows;
use crate::theme::*;
use crate::widgets::*;

// The titled groups of rows on each page, which fold away under their title. A section shows
// only while something in it does, so one whose rows all have nothing to do for the suite on
// screen takes no room.
pub(crate) struct SectionsPlugin;

impl Plugin for SectionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click)
            .add_systems(Update, show_folds.run_if(resource_changed::<UiState>.or_eager(new_panel)))
            .add_systems(Update, show_summaries.run_if(crate::tabs::sources_changed))
            .add_systems(
                PostUpdate,
                fit_sections.after(DrawRows).before(bevy::ui::UiSystems::Prepare),
            );
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Suite,
    Outfit,
    // The worn skin's objects, switched on and off one by one. Inside Outfit.
    Parts,
    // Animation and expression.
    Pose,
    // Environment and camera.
    Place,
    Gaze,
    Physics,
    Lighting,
    Color,
    Bloom,
    DepthOfField,
    Fog,
    AmbientOcclusion,
    Strokes,
    Film,
    Quality,
    FrameRate,
    Pause,
    Advanced,
    App,
    Display,
    Interface,
    Sound,
    Music,
}

impl Section {
    // Its name in state.toml.
    fn key(self) -> &'static str {
        match self {
            Section::Suite => "suite",
            Section::Outfit => "outfit",
            Section::Parts => "parts",
            Section::Pose => "pose",
            Section::Place => "place",
            Section::Gaze => "gaze",
            Section::Physics => "physics",
            Section::Lighting => "lighting",
            Section::Color => "color",
            Section::Bloom => "bloom",
            Section::DepthOfField => "depth_of_field",
            Section::Fog => "fog",
            Section::AmbientOcclusion => "ambient_occlusion",
            Section::Strokes => "strokes",
            Section::Film => "film",
            Section::Quality => "quality",
            Section::FrameRate => "frame_rate",
            Section::Pause => "pause",
            Section::Advanced => "advanced",
            Section::App => "app",
            Section::Display => "display",
            Section::Interface => "interface",
            Section::Sound => "sound",
            Section::Music => "music",
        }
    }

    // A section the viewer never folded or unfolded starts as it does by default.
    fn is_folded(self, ui: &UiState) -> bool {
        ui.folded.get(self.key()).copied().unwrap_or_else(|| self.folded_by_default())
    }

    // Fine-tuning starts folded, so a first look at the panel shows the main choices. Each
    // effect folds to a line saying whether it's on, so the Look tab reads as a list of them.
    fn folded_by_default(self) -> bool {
        matches!(
            self,
            Section::Parts
                | Section::Color
                | Section::Bloom
                | Section::DepthOfField
                | Section::Fog
                | Section::AmbientOcclusion
                | Section::Strokes
                | Section::Film
                | Section::Advanced
        )
    }
}

// What keeps a section shown while it is shown itself: a row, or a list a tab fills in.
#[derive(Component, Default)]
pub(crate) struct Content;

// The title, which folds the section on a click.
#[derive(Component)]
struct Fold(Section);

// The part that folds away under the title.
#[derive(Component)]
struct Body(Section);

// What a folded section says of how it's set, from the world as rows read it. None while
// there is nothing to say yet.
pub(crate) type SummaryFn = fn(&World, &FluentLanguageLoader) -> Option<String>;

// The line at the end of a section's title that says how it's set, while it's folded.
#[derive(Component)]
struct Summary(Section, Option<SummaryFn>);

// The chevron at the end of the title, pointing down while the section is open.
#[derive(Component)]
struct Chevron(Section);

const CHEVRON_SIZE: FontSize = FontSize::Px(14.0);

// The content goes on a card under the title, usually `children![]` of rows.
pub(crate) fn section(title: Tr, section: Section, content: impl Bundle) -> impl Bundle {
    let body = (
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(14.0), Val::Px(4.0)),
            border_radius: BorderRadius::all(CARD_RADIUS),
            ..default()
        },
        Fill(CARD_BG),
        Body(section),
        Pickable::IGNORE,
        content,
    );
    (
        Node { display: Display::None, flex_direction: FlexDirection::Column, ..default() },
        section,
        Pickable::IGNORE,
        children![fold_header(title, section, PANEL_BG, None), body],
    )
}

// A section whose title, while it's folded, says how it's set.
pub(crate) fn summarized(
    title: Tr,
    section: Section,
    summary: SummaryFn,
    content: impl Bundle,
) -> impl Bundle {
    let body = (
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(14.0), Val::Px(4.0)),
            border_radius: BorderRadius::all(CARD_RADIUS),
            ..default()
        },
        Fill(CARD_BG),
        Body(section),
        Pickable::IGNORE,
        content,
    );
    (
        Node { display: Display::None, flex_direction: FlexDirection::Column, ..default() },
        section,
        Pickable::IGNORE,
        children![fold_header(title, section, PANEL_BG, Some(summary)), body],
    )
}

// A section inside another, already on its card: the content goes straight under the title.
pub(crate) fn subsection(title: Tr, section: Section, content: impl Bundle) -> impl Bundle {
    let body = (
        Node { flex_direction: FlexDirection::Column, ..default() },
        Body(section),
        Pickable::IGNORE,
        content,
    );
    (
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        },
        section,
        Pickable::IGNORE,
        children![fold_header(title, section, CARD_BG, None), body],
    )
}

// The whole title row is the button. It's filled with what's behind it, which looks like no
// fill at rest but gives hover feedback something to brighten.
fn fold_header(
    title: Tr,
    section: Section,
    behind: Swatch,
    summary: Option<SummaryFn>,
) -> impl Bundle {
    (
        Node {
            height: Val::Px(28.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::horizontal(Val::Px(4.0)),
            border_radius: BorderRadius::all(RADIUS),
            ..default()
        },
        BaseBackground::default(),
        Fill(behind),
        Fold(section),
        children![
            (translated(title, SMALL_SIZE, TEXT_DIM), Node { flex_shrink: 0.0, ..default() }),
            (
                label("", SMALL_SIZE, TEXT_DIM),
                TextLayout::new(Justify::Right, LineBreak::NoWrap),
                Node {
                    flex_grow: 1.0,
                    min_width: Val::ZERO,
                    margin: UiRect::horizontal(Val::Px(8.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                Summary(section, summary),
            ),
            (icon(Icon::ChevronDown, CHEVRON_SIZE, TEXT_DIM), Chevron(section)),
        ],
    )
}

fn on_click(event: On<Pointer<Click>>, folds: Query<&Fold>, mut ui: ResMut<UiState>) {
    if let Ok(Fold(section)) = folds.get(event.entity) {
        let folded = !section.is_folded(&ui);
        ui.folded.insert(section.key().to_string(), folded);
    }
}

// Written only on a real change: UiState changes on every tab switch too, and an untouched
// Node or Text spares Bevy laying the panel out again.
fn show_folds(
    ui: Res<UiState>,
    mut bodies: Query<(&Body, &mut Node)>,
    mut chevrons: Query<(&Chevron, &mut Text)>,
) {
    for (body, mut node) in &mut bodies {
        let shown = display(!body.0.is_folded(&ui));
        if node.display != shown {
            node.display = shown;
        }
    }
    for (chevron, mut text) in &mut chevrons {
        let icon = if chevron.0.is_folded(&ui) { Icon::ChevronRight } else { Icon::ChevronDown };
        let glyph = icon.glyph().to_string();
        if text.0 != glyph {
            text.0 = glyph;
        }
    }
}

// Every summary works its line out again, but a line is only written when it changed.
fn show_summaries(world: &mut World, summaries: &mut QueryState<(Entity, &'static Summary)>) {
    let lines: Vec<(Entity, String)> = {
        let world: &World = world;
        let ui = world.resource::<UiState>();
        let locale: &FluentLanguageLoader = world.resource::<Locale>();
        let line = |(entity, summary): (Entity, &Summary)| {
            let text = match summary.1 {
                Some(summarize) if summary.0.is_folded(ui) => summarize(world, locale)?,
                _ => String::new(),
            };
            Some((entity, text))
        };
        summaries.iter(world).filter_map(line).collect()
    };
    for (entity, line) in lines {
        if let Some(mut text) = world.get_mut::<Text>(entity)
            && text.0 != line
        {
            text.0 = line;
        }
    }
}

// Looks at the content itself rather than whether it is on screen, so a folded section, whose
// content is out of sight, still shows its title.
fn fit_sections(
    changed: Query<(), (With<Content>, Changed<Node>)>,
    contents: Query<&Node, (With<Content>, Without<Section>)>,
    mut sections: Query<(Entity, &mut Node), (With<Section>, Without<Content>)>,
    children: Query<&Children>,
) {
    if changed.is_empty() {
        return;
    }
    for (section, mut node) in &mut sections {
        let filled = children
            .iter_descendants(section)
            .any(|part| contents.get(part).is_ok_and(|part| part.display != Display::None));
        let shown = display(filled);
        if node.display != shown {
            node.display = shown;
        }
    }
}
