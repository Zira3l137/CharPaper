use bevy::prelude::*;

pub mod defaults {
    #![allow(dead_code)]
    use bevy::prelude::*;

    pub(crate) const BTN_PADDING: Val = Val::Px(10.0);
    pub(crate) const BTN_RADIUS: Val = Val::Px(16.0);
    pub(crate) const BTN_BG_COLOR: Color = Color::srgba(0.1, 0.2, 0.5, 1.0);
    pub(crate) const BTN_TEXT_SIZE: FontSize = FontSize::Px(16.0);
    pub(crate) const BTN_WIDTH: Val = Val::Px(200.0);
    pub(crate) const BTN_HEIGHT: Val = Val::Px(50.0);
}

const HOVER_DELTA: f32 = 0.08;
const PRESS_DELTA: f32 = -0.08;
const IDLE_DELTA: f32 = 0.0;

type FeedbackQuery<'w, 's> = Query<
    'w,
    's,
    (&'static BaseBackground, &'static mut BackgroundColor, &'static mut BackgroundGradient),
    With<UiButton>,
>;

#[allow(dead_code)]
pub trait UiNode {
    fn inner_mut(&mut self) -> &mut Node;

    fn width(&mut self, width: Val) -> &mut Self {
        self.inner_mut().width = width;
        self
    }

    fn height(&mut self, height: Val) -> &mut Self {
        self.inner_mut().height = height;
        self
    }

    fn padding(&mut self, padding: Val) -> &mut Self {
        self.inner_mut().padding = UiRect::all(padding);
        self
    }

    fn margin(&mut self, margin: Val) -> &mut Self {
        self.inner_mut().margin = UiRect::all(margin);
        self
    }

    fn border(&mut self, border: Val) -> &mut Self {
        self.inner_mut().border = UiRect::all(border);
        self
    }

    fn border_radius(&mut self, border_radius: Val) -> &mut Self {
        self.inner_mut().border_radius = BorderRadius::all(border_radius);
        self
    }

    fn justify_self(&mut self, justify_self: JustifySelf) -> &mut Self {
        self.inner_mut().justify_self = justify_self;
        self
    }

    fn justify_content(&mut self, justify_content: JustifyContent) -> &mut Self {
        self.inner_mut().justify_content = justify_content;
        self
    }

    fn align_self(&mut self, align_self: AlignSelf) -> &mut Self {
        self.inner_mut().align_self = align_self;
        self
    }

    fn align_items(&mut self, align_items: AlignItems) -> &mut Self {
        self.inner_mut().align_items = align_items;
        self
    }

    fn edit_node(&mut self, f: impl FnOnce(&mut Node)) -> &mut Self {
        f(&mut self.inner_mut());
        self
    }
}

#[allow(dead_code)]
pub trait WithBackground: UiNode {
    fn bg_color_mut(&mut self) -> &mut Color;
    fn bg_shadow_mut(&mut self) -> &mut ShadowStyle;
    fn bg_gradient_mut(&mut self) -> &mut Option<Gradient>;

    fn bg_color(&mut self, color: Color) -> &mut Self {
        *self.bg_color_mut() = color;
        self
    }

    fn bg_gradient(&mut self, gradient: Gradient) -> &mut Self {
        self.bg_gradient_mut().replace(gradient);
        self
    }

    fn bg_shadow_radius(&mut self, radius: Val) -> &mut Self {
        self.bg_shadow_mut().spread_radius = radius;
        self
    }

    fn bg_shadow_offset(&mut self, x: Val, y: Val) -> &mut Self {
        self.bg_shadow_mut().x_offset = x;
        self.bg_shadow_mut().y_offset = y;
        self
    }

    fn bg_shadow_color(&mut self, color: Color) -> &mut Self {
        self.bg_shadow_mut().color = color;
        self
    }

    fn bg_shadow_blur_radius(&mut self, radius: Val) -> &mut Self {
        self.bg_shadow_mut().blur_radius = radius;
        self
    }
}

#[allow(dead_code)]
pub trait WithText: UiNode {
    fn text_color_mut(&mut self) -> &mut TextColor;
    fn text_shadow_color_mut(&mut self) -> &mut Color;
    fn text_shadow_offset_mut(&mut self) -> &mut Vec2;
    fn text_mut(&mut self) -> &mut Text;
    fn font_mut(&mut self) -> &mut TextFont;

    fn text(&mut self, text: impl Into<String>) -> &mut Self {
        self.text_mut().0 = text.into();
        self
    }

    fn text_color(&mut self, color: Color) -> &mut Self {
        self.text_color_mut().0 = color;
        self
    }

    fn text_shadow_color(&mut self, color: Color) -> &mut Self {
        *self.text_shadow_color_mut() = color;
        self
    }

    fn text_shadow_offset(&mut self, offset: Vec2) -> &mut Self {
        *self.text_shadow_offset_mut() = offset;
        self
    }

    fn font_size(&mut self, size: FontSize) -> &mut Self {
        self.font_mut().font_size = size;
        self
    }

    fn font_family(&mut self, family: FontSource) -> &mut Self {
        self.font_mut().font = family;
        self
    }

    fn font_weight(&mut self, weight: FontWeight) -> &mut Self {
        self.font_mut().weight = weight;
        self
    }

    fn font_style(&mut self, style: FontStyle) -> &mut Self {
        self.font_mut().style = style;
        self
    }
}

impl UiNode for ButtonBuilder {
    fn inner_mut(&mut self) -> &mut Node {
        &mut self.inner
    }
}

impl WithBackground for ButtonBuilder {
    fn bg_color_mut(&mut self) -> &mut Color {
        &mut self.bg_color
    }

    fn bg_shadow_mut(&mut self) -> &mut ShadowStyle {
        &mut self.bg_shadow
    }

    fn bg_gradient_mut(&mut self) -> &mut Option<Gradient> {
        &mut self.bg_gradient
    }
}

impl WithText for ButtonBuilder {
    fn text_mut(&mut self) -> &mut Text {
        &mut self.text
    }

    fn text_color_mut(&mut self) -> &mut TextColor {
        &mut self.text_color
    }

    fn text_shadow_color_mut(&mut self) -> &mut Color {
        &mut self.text_shadow_color
    }

    fn text_shadow_offset_mut(&mut self) -> &mut Vec2 {
        &mut self.text_shadow_offset
    }

    fn font_mut(&mut self) -> &mut TextFont {
        &mut self.font
    }
}

pub trait ColorExt {
    fn adjust_brightness(self, delta: f32) -> Self;
}

impl ColorExt for Color {
    fn adjust_brightness(self, delta: f32) -> Self {
        if delta == 0.0 {
            return self;
        }
        let hsla: Hsla = self.into();
        Color::from(hsla.with_lightness((hsla.lightness + delta).clamp(0.0, 1.0)))
    }
}

impl ColorExt for Gradient {
    fn adjust_brightness(mut self, delta: f32) -> Self {
        match &mut self {
            Gradient::Linear(g) => {
                g.stops.iter_mut().for_each(|s| s.color = s.color.adjust_brightness(delta))
            }
            Gradient::Radial(g) => {
                g.stops.iter_mut().for_each(|s| s.color = s.color.adjust_brightness(delta))
            }
            Gradient::Conic(g) => {
                g.stops.iter_mut().for_each(|s| s.color = s.color.adjust_brightness(delta))
            }
        }
        self
    }
}

impl ColorExt for Background {
    fn adjust_brightness(self, delta: f32) -> Self {
        match self {
            Background::Color(c) => Background::Color(c.adjust_brightness(delta)),
            Background::Gradient(g) => Background::Gradient(g.adjust_brightness(delta)),
        }
    }
}

pub trait ButtonReactiveExt {
    fn with_button_feedback(&mut self) -> &mut Self;
}

impl ButtonReactiveExt for EntityCommands<'_> {
    fn with_button_feedback(&mut self) -> &mut Self {
        self.observe(feedback::<Over>(HOVER_DELTA))
            .observe(feedback::<Out>(IDLE_DELTA))
            .observe(feedback::<Press>(PRESS_DELTA))
            // Cursor is still over the button on release.
            .observe(feedback::<Release>(HOVER_DELTA))
    }
}

#[derive(Component, Debug)]
pub enum UiButton {
    Exit,
}

#[derive(Component, Debug, Clone)]
pub enum Background {
    Color(Color),
    Gradient(Gradient),
}

impl Default for Background {
    fn default() -> Self {
        Background::Color(Color::default())
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct BaseBackground(pub Background);

#[derive(Default)]
pub struct ButtonBuilder {
    inner: Node,
    text: Text,
    font: TextFont,
    text_color: TextColor,
    text_shadow_color: Color,
    text_shadow_offset: Vec2,
    bg_color: Color,
    bg_shadow: ShadowStyle,
    bg_gradient: Option<Gradient>,
}

#[allow(dead_code)]
impl ButtonBuilder {
    pub fn build(&mut self) -> impl Bundle {
        let base = if let Some(grad) = &self.bg_gradient {
            Background::Gradient(grad.clone())
        } else {
            Background::Color(self.bg_color)
        };
        let bg_color = if self.bg_gradient.is_some() {
            BackgroundColor(Color::NONE)
        } else {
            BackgroundColor(self.bg_color)
        };
        let bg_gradient = if let Some(grad) = &self.bg_gradient {
            BackgroundGradient(vec![grad.clone()])
        } else {
            BackgroundGradient::default()
        };

        (
            self.inner.clone(),
            BaseBackground(base),
            BoxShadow(vec![self.bg_shadow]),
            bg_color,
            bg_gradient,
            children![(
                self.text.clone(),
                self.font.clone(),
                self.text_color,
                Pickable::IGNORE,
                TextShadow { color: self.text_shadow_color, offset: self.text_shadow_offset }
            )],
        )
    }

    pub fn build_marked(&mut self, marker: impl Component) -> impl Bundle {
        (self.build(), marker)
    }
}

pub fn ui_root() -> impl Bundle {
    Node {
        width: percent(100),
        height: percent(100),
        align_self: AlignSelf::Center,
        justify_self: JustifySelf::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn feedback<E>(delta: f32) -> impl Fn(On<Pointer<E>>, FeedbackQuery)
where
    E: std::fmt::Debug + Clone + Reflect,
{
    move |event, mut query| {
        let Ok((base, mut color, mut gradient)) = query.get_mut(event.entity) else {
            return;
        };

        match base.0.clone().adjust_brightness(delta) {
            Background::Color(c) => color.0 = c,
            Background::Gradient(g) => gradient.0 = vec![g],
        }
    }
}
