use bevy::prelude::*;

use crate::elements::UiButton;
use crate::locale::Localized;
use crate::locale::Tr;

const HOVER: f32 = 0.08;
const PRESS: f32 = -0.08;

// A button's resting fill. Hover and press feedback brighten or darken from it.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct BaseBackground(pub Color);

pub(crate) struct ButtonBuilder {
    node: Node,
    text: String,
    translated: Option<Tr>,
    font: TextFont,
    text_color: TextColor,
    fill: Color,
    border_color: Color,
}

impl ButtonBuilder {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            node: Node::default(),
            text: text.into(),
            translated: None,
            font: TextFont::default(),
            text_color: TextColor::default(),
            fill: Color::default(),
            border_color: Color::NONE,
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

    pub(crate) fn text_color(mut self, color: Color) -> Self {
        self.text_color.0 = color;
        self
    }

    pub(crate) fn fill(mut self, color: Color) -> Self {
        self.fill = color;
        self
    }

    // Only visible with a non-zero border width on the node.
    pub(crate) fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub(crate) fn build(self, action: UiButton) -> impl Bundle {
        (
            self.node,
            BaseBackground(self.fill),
            BackgroundColor(self.fill),
            BorderColor::all(self.border_color),
            action,
            children![(
                Text::new(self.text),
                self.font,
                self.text_color,
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

type Feedback<'w, 's> =
    Query<'w, 's, (&'static BaseBackground, &'static mut BackgroundColor), With<UiButton>>;

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
    Color::from(hsla.with_lightness((hsla.lightness + delta).clamp(0.0, 1.0)))
}
