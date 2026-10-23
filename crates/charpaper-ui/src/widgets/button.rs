use bevy::prelude::*;
use bevy::text::FontSource;

use crate::locale::Localized;
use crate::locale::Tr;
use crate::theme::*;

const HOVER: f32 = 0.08;
const PRESS: f32 = -0.08;

// A button's resting fill, which `repaint` keeps as its Fill's color. Hover and press feedback
// brighten or darken from it.
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct BaseBackground(pub Color);

pub(crate) struct ButtonBuilder {
    node: Node,
    text: String,
    translated: Option<Tr>,
    font: TextFont,
    text_color: Swatch,
    fill: Swatch,
    border_color: Swatch,
}

impl ButtonBuilder {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            node: Node::default(),
            text: text.into(),
            translated: None,
            font: TextFont { font: FontSource::SystemUi, ..default() },
            text_color: TEXT,
            fill: BUTTON_BG,
            border_color: CLEAR,
        }
    }

    pub(crate) fn translated(text: Tr) -> Self {
        Self { translated: Some(text), ..Self::new("") }
    }

    pub(crate) fn node(mut self, edit: impl FnOnce(&mut Node)) -> Self {
        edit(&mut self.node);
        self
    }

    pub(crate) fn font_size(mut self, size: FontSize) -> Self {
        self.font.font_size = size;
        self
    }

    pub(crate) fn text_color(mut self, color: Swatch) -> Self {
        self.text_color = color;
        self
    }

    pub(crate) fn fill(mut self, color: Swatch) -> Self {
        self.fill = color;
        self
    }

    // Only visible with a non-zero border width on the node.
    pub(crate) fn border_color(mut self, color: Swatch) -> Self {
        self.border_color = color;
        self
    }

    // The action is the component that says what a click does, for the observer that acts on it.
    pub(crate) fn build(self, action: impl Bundle) -> impl Bundle {
        (
            self.node,
            BaseBackground::default(),
            Fill(self.fill),
            Stroke(self.border_color),
            action,
            children![(
                Text::new(self.text),
                self.font,
                Ink(self.text_color),
                Pickable::IGNORE,
                Localized(self.translated),
            )],
        )
    }
}

pub(crate) trait HoverFeedback {
    fn with_hover_feedback(&mut self) -> &mut Self;
}

// On a button, or on any ancestor of buttons: pointer events bubble up to it, and the
// feedback is applied to the button the event started on.
impl HoverFeedback for EntityCommands<'_> {
    fn with_hover_feedback(&mut self) -> &mut Self {
        self.observe(brighten::<Over>(HOVER))
            .observe(brighten::<Out>(0.0))
            .observe(brighten::<Press>(PRESS))
            .observe(brighten::<Release>(HOVER))
    }
}

type Feedback<'w, 's> = Query<'w, 's, (&'static BaseBackground, &'static mut BackgroundColor)>;

fn brighten<E>(delta: f32) -> impl Fn(On<Pointer<E>>, Feedback)
where
    E: std::fmt::Debug + Clone + Reflect,
{
    move |event, mut buttons| {
        if let Ok((base, mut color)) = buttons.get_mut(event.original_event_target()) {
            color.0 = lighten(base.0, delta);
        }
    }
}

fn lighten(color: Color, delta: f32) -> Color {
    if delta == 0.0 {
        return color;
    }
    let hsla: Hsla = color.into();
    // Near-white fills, as in the light palette, darken instead: brightening them shows nothing.
    let delta = if hsla.lightness > 0.75 { -delta } else { delta };
    Color::from(hsla.with_lightness((hsla.lightness + delta).clamp(0.0, 1.0)))
}
