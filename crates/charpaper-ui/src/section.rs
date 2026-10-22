use bevy::prelude::*;
use charpaper_settings::UiState;

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
    Animation,
    Expression,
    Gaze,
    Camera,
    Environment,
    // Image, color and film together.
    PostProcessing,
    Lighting,
    Image,
    Color,
    Film,
    FrameRate,
    Quality,
    Fog,
    Strokes,
    Pause,
    App,
    Display,
    Sound,
    Music,
    Interface,
}

impl Section {
    // Its name in state.toml.
    fn key(self) -> &'static str {
        match self {
            Section::Suite => "suite",
            Section::Outfit => "outfit",
            Section::Parts => "parts",
            Section::Animation => "animation",
            Section::Expression => "expression",
            Section::Gaze => "gaze",
            Section::Camera => "camera",
            Section::Environment => "environment",
            Section::PostProcessing => "post_processing",
            Section::Lighting => "lighting",
            Section::Image => "image",
            Section::Color => "color",
            Section::Film => "film",
            Section::FrameRate => "frame_rate",
            Section::Quality => "quality",
            Section::Fog => "fog",
            Section::Strokes => "strokes",
            Section::Pause => "pause",
            Section::App => "app",
            Section::Display => "display",
            Section::Sound => "sound",
            Section::Music => "music",
            Section::Interface => "interface",
        }
    }

    // A section the viewer never folded or unfolded starts as it does by default.
    fn is_folded(self, ui: &UiState) -> bool {
        ui.folded.get(self.key()).copied().unwrap_or_else(|| self.folded_by_default())
    }

    // Fine-tuning starts folded, so a first look at the panel shows the main choices.
    fn folded_by_default(self) -> bool {
        matches!(
            self,
            Section::Parts
                | Section::Image
                | Section::Color
                | Section::Film
                | Section::Fog
                | Section::Pause
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

// The arrow beside the title, pointing down while the section is open.
#[derive(Component)]
struct Chevron(Section);

// The content goes in a column under the title, usually `children![]` of rows.
pub(crate) fn section(title: Tr, section: Section, content: impl Bundle) -> impl Bundle {
    (
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },
        section,
        Pickable::IGNORE,
        children![
            fold_header(title, section),
            (
                Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
                Body(section),
                Pickable::IGNORE,
                content,
            ),
        ],
    )
}

// The whole title row is the button. It's filled with the panel's own color, which looks like
// no fill at rest but gives hover feedback something to brighten.
fn fold_header(title: Tr, section: Section) -> impl Bundle {
    (
        Node {
            height: Val::Px(24.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            padding: UiRect::horizontal(Val::Px(4.0)),
            border_radius: BorderRadius::all(RADIUS),
            ..default()
        },
        BaseBackground(PANEL_BG),
        BackgroundColor(PANEL_BG),
        Fold(section),
        children![
            // Fixed width: the two arrows differ, and the title shouldn't shift between them.
            (
                Node { width: Val::Px(10.0), justify_content: JustifyContent::Center, ..default() },
                Pickable::IGNORE,
                children![(label("", SMALL_SIZE, TEXT_DIM), Chevron(section))],
            ),
            translated(title, SMALL_SIZE, TEXT_DIM),
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
        let arrow = if chevron.0.is_folded(&ui) { "►" } else { "▼" };
        if text.0 != arrow {
            text.0 = arrow.to_string();
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
