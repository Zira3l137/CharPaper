use bevy::prelude::*;
use bevy::ui::UiSystems;
use charpaper_settings::Palette;
use charpaper_settings::UiState;
use charpaper_settings::UiTheme;

use crate::row::DrawRows;
use crate::widgets::BaseBackground;

// Keeps every painted part of the panel and the performance monitor, and how big they are drawn,
// in step with the theme picked in the panel.
pub(crate) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Theme>()
            .add_systems(Update, follow_settings.run_if(resource_changed::<UiState>))
            .add_systems(PostUpdate, repaint.after(DrawRows).before(UiSystems::Prepare));
    }
}

// The part a color plays. Which color that is depends on the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Swatch {
    Clear,
    PanelBg,
    CardBg,
    Border,
    ButtonBg,
    WellBg,
    SelectedBg,
    Raised,
    Track,
    Knob,
    Accent,
    Bar,
    LiveBorder,
    OnAccent,
    Text,
    TextDim,
    TextLabel,
    DangerBorder,
    DangerText,
}

impl Swatch {
    const ALL: [Self; 19] = [
        Self::Clear,
        Self::PanelBg,
        Self::CardBg,
        Self::Border,
        Self::ButtonBg,
        Self::WellBg,
        Self::SelectedBg,
        Self::Raised,
        Self::Track,
        Self::Knob,
        Self::Accent,
        Self::Bar,
        Self::LiveBorder,
        Self::OnAccent,
        Self::Text,
        Self::TextDim,
        Self::TextLabel,
        Self::DangerBorder,
        Self::DangerText,
    ];
}

pub(crate) const CLEAR: Swatch = Swatch::Clear;
pub(crate) const PANEL_BG: Swatch = Swatch::PanelBg;
// Under a section's rows, a step up from the panel.
pub(crate) const CARD_BG: Swatch = Swatch::CardBg;
pub(crate) const BORDER: Swatch = Swatch::Border;
pub(crate) const BUTTON_BG: Swatch = Swatch::ButtonBg;
// The groove segments sit in, and an open list.
pub(crate) const WELL_BG: Swatch = Swatch::WellBg;
// A picked choice in a list or grid: tinted by the accent.
pub(crate) const SELECTED_BG: Swatch = Swatch::SelectedBg;
// The picked segment, raised out of its groove.
pub(crate) const RAISED_BG: Swatch = Swatch::Raised;
// Behind a number's fill and an off switch.
pub(crate) const TRACK: Swatch = Swatch::Track;
pub(crate) const KNOB: Swatch = Swatch::Knob;
pub(crate) const ACCENT: Swatch = Swatch::Accent;
// How far into its range a number is, along its track.
pub(crate) const BAR: Swatch = Swatch::Bar;
// Around a value the wheel would change right now.
pub(crate) const LIVE_BORDER: Swatch = Swatch::LiveBorder;
pub(crate) const ON_ACCENT: Swatch = Swatch::OnAccent;
pub(crate) const TEXT: Swatch = Swatch::Text;
pub(crate) const TEXT_DIM: Swatch = Swatch::TextDim;
pub(crate) const TEXT_LABEL: Swatch = Swatch::TextLabel;
pub(crate) const DANGER_BORDER: Swatch = Swatch::DangerBorder;
pub(crate) const DANGER_TEXT: Swatch = Swatch::DangerText;

pub(crate) const PANEL_WIDTH: Val = Val::Px(380.0);
pub(crate) const PANEL_MARGIN: Val = Val::Px(16.0);
pub(crate) const PANEL_RADIUS: Val = Val::Px(16.0);
pub(crate) const CARD_RADIUS: Val = Val::Px(12.0);
pub(crate) const RADIUS: Val = Val::Px(8.0);
// Inside something rounded by RADIUS, such as a segment in its groove.
pub(crate) const INNER_RADIUS: Val = Val::Px(6.0);
pub(crate) const LINE: Val = Val::Px(1.0);
pub(crate) const CONTROL_HEIGHT: Val = Val::Px(30.0);
pub(crate) const ROW_HEIGHT: Val = Val::Px(40.0);
pub(crate) const LABEL_WIDTH: Val = Val::Px(116.0);

pub(crate) const TITLE_SIZE: FontSize = FontSize::Px(14.0);
pub(crate) const BODY_SIZE: FontSize = FontSize::Px(13.0);
pub(crate) const CONTROL_SIZE: FontSize = FontSize::Px(12.0);
pub(crate) const SMALL_SIZE: FontSize = FontSize::Px(11.0);
pub(crate) const ICON_SIZE: FontSize = FontSize::Px(16.0);

// Parts are spawned with the part their background, border or text plays rather than a color,
// and `repaint` gives them the theme's color for it. A part that changes look, such as a tab
// becoming the open one, changes these, never its colors.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[require(BackgroundColor)]
pub(crate) struct Fill(pub Swatch);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[require(BorderColor)]
pub(crate) struct Stroke(pub Swatch);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[require(TextColor)]
pub(crate) struct Ink(pub Swatch);

// The color of each swatch, in the order of Swatch::ALL.
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct Theme([Color; Swatch::ALL.len()]);

impl Default for Theme {
    fn default() -> Self {
        Self::new(&UiTheme::default())
    }
}

impl Theme {
    fn new(settings: &UiTheme) -> Self {
        let base = Base::of(settings.palette);
        let accent = settings.accent.color();
        let opacity = settings.opacity.clamp(0.0, 1.0);
        Self(std::array::from_fn(|i| match Swatch::ALL[i] {
            Swatch::Clear => Color::NONE,
            Swatch::PanelBg => with_alpha(base.panel, opacity),
            Swatch::CardBg => with_alpha(base.card, opacity),
            Swatch::Border => base.border,
            Swatch::ButtonBg => base.button,
            Swatch::WellBg => base.well,
            Swatch::SelectedBg => mix(base.well, accent, 0.22),
            Swatch::Raised => base.raised,
            Swatch::Track => mix(base.card, base.text, 0.14),
            Swatch::Knob => base.knob,
            Swatch::Accent => accent,
            Swatch::Bar => accent,
            Swatch::LiveBorder => mix(base.border, accent, 0.7),
            Swatch::OnAccent => on(accent),
            Swatch::Text => base.text,
            Swatch::TextDim => base.text_dim,
            Swatch::TextLabel => base.text_label,
            Swatch::DangerBorder => base.danger_border,
            Swatch::DangerText => base.danger_text,
        }))
    }

    pub(crate) fn color(&self, swatch: Swatch) -> Color {
        self.0[swatch as usize]
    }
}

// What a palette gives. The colors made from the accent are mixed into these.
struct Base {
    panel: Color,
    card: Color,
    border: Color,
    button: Color,
    well: Color,
    raised: Color,
    knob: Color,
    text: Color,
    text_dim: Color,
    text_label: Color,
    danger_border: Color,
    danger_text: Color,
}

impl Base {
    fn of(palette: Palette) -> Self {
        match palette {
            Palette::Slate => Self {
                panel: Color::srgb_u8(0x13, 0x16, 0x1D),
                card: Color::srgb_u8(0x1A, 0x1D, 0x25),
                border: Color::srgb_u8(0x29, 0x2D, 0x38),
                button: Color::srgb_u8(0x20, 0x24, 0x2D),
                well: Color::srgb_u8(0x21, 0x25, 0x2E),
                raised: Color::srgb_u8(0x31, 0x36, 0x42),
                knob: Color::srgb_u8(0xF4, 0xF5, 0xF8),
                text: Color::srgb_u8(0xE8, 0xEA, 0xF0),
                text_dim: Color::srgb_u8(0x9A, 0xA1, 0xB2),
                text_label: Color::srgb_u8(0xC5, 0xCA, 0xD6),
                danger_border: Color::srgb_u8(0x4A, 0x2A, 0x2E),
                danger_text: Color::srgb_u8(0xF2, 0x9A, 0x9A),
            },
            Palette::Midnight => Self {
                panel: Color::srgb_u8(0x0A, 0x0A, 0x0C),
                card: Color::srgb_u8(0x12, 0x12, 0x15),
                border: Color::srgb_u8(0x22, 0x23, 0x28),
                button: Color::srgb_u8(0x18, 0x19, 0x1D),
                well: Color::srgb_u8(0x19, 0x1A, 0x1E),
                raised: Color::srgb_u8(0x29, 0x2A, 0x30),
                knob: Color::srgb_u8(0xF2, 0xF2, 0xF5),
                text: Color::srgb_u8(0xED, 0xED, 0xF0),
                text_dim: Color::srgb_u8(0x9B, 0x9C, 0xA4),
                text_label: Color::srgb_u8(0xC7, 0xC8, 0xCE),
                danger_border: Color::srgb_u8(0x4A, 0x24, 0x27),
                danger_text: Color::srgb_u8(0xF2, 0x9A, 0x9A),
            },
            Palette::Light => Self {
                panel: Color::srgb_u8(0xEE, 0xF0, 0xF3),
                card: Color::srgb_u8(0xFF, 0xFF, 0xFF),
                border: Color::srgb_u8(0xD6, 0xDA, 0xE1),
                button: Color::srgb_u8(0xF3, 0xF4, 0xF7),
                well: Color::srgb_u8(0xEB, 0xED, 0xF1),
                raised: Color::srgb_u8(0xFF, 0xFF, 0xFF),
                knob: Color::srgb_u8(0xFF, 0xFF, 0xFF),
                text: Color::srgb_u8(0x16, 0x19, 0x1F),
                text_dim: Color::srgb_u8(0x5A, 0x61, 0x6E),
                text_label: Color::srgb_u8(0x34, 0x3A, 0x45),
                danger_border: Color::srgb_u8(0xE5, 0xBF, 0xBF),
                danger_text: Color::srgb_u8(0xB4, 0x23, 0x18),
            },
        }
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    let mut srgba = color.to_srgba();
    srgba.alpha = alpha;
    srgba.into()
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    let (a, b) = (from.to_srgba(), to.to_srgba());
    let at = |a: f32, b: f32| a + (b - a) * amount;
    Color::srgba(at(a.red, b.red), at(a.green, b.green), at(a.blue, b.blue), at(a.alpha, b.alpha))
}

// Text on the accent: near-black or white, whichever stands out more against it.
fn on(accent: Color) -> Color {
    let dark = Color::srgb_u8(0x08, 0x11, 0x1C);
    let contrast = |a: Color, b: Color| {
        let (a, b) = (luminance(a) + 0.05, luminance(b) + 0.05);
        a.max(b) / a.min(b)
    };
    if contrast(accent, dark) >= contrast(accent, Color::WHITE) { dark } else { Color::WHITE }
}

fn luminance(color: Color) -> f32 {
    let linear = color.to_linear();
    0.2126 * linear.red + 0.7152 * linear.green + 0.0722 * linear.blue
}

fn follow_settings(ui: Res<UiState>, mut theme: ResMut<Theme>, mut scale: ResMut<UiScale>) {
    theme.set_if_neq(Theme::new(&ui.theme));
    let size = ui.theme.size.clamp(0.5, 2.0);
    if scale.0 != size {
        scale.0 = size;
    }
}

// A changed theme repaints everything; otherwise only parts spawned or restyled since last frame.
// Runs after the rows are drawn, so their new buttons are painted before they are first seen.
fn repaint(
    theme: Res<Theme>,
    mut fills: Query<(Ref<Fill>, &mut BackgroundColor, Option<&mut BaseBackground>)>,
    mut strokes: Query<(Ref<Stroke>, &mut BorderColor)>,
    mut inks: Query<(Ref<Ink>, &mut TextColor)>,
) {
    let all = theme.is_changed();
    for (fill, mut background, base) in &mut fills {
        if all || fill.is_changed() {
            background.0 = theme.color(fill.0);
            if let Some(mut base) = base {
                base.0 = background.0;
            }
        }
    }
    for (stroke, mut border) in &mut strokes {
        if all || stroke.is_changed() {
            *border = BorderColor::all(theme.color(stroke.0));
        }
    }
    for (ink, mut text) in &mut inks {
        if all || ink.is_changed() {
            text.0 = theme.color(ink.0);
        }
    }
}
