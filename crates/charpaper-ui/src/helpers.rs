use bevy::prelude::*;

use crate::Tab;

const HOVER_DELTA: f32 = 0.08;
const PRESS_DELTA: f32 = -0.08;
const IDLE_DELTA: f32 = 0.0;

type FeedbackQuery<'w, 's> = Query<
    'w,
    's,
    (&'static BaseBackground, &'static mut BackgroundColor, &'static mut BackgroundGradient),
    With<UiElement>,
>;

#[allow(dead_code)]
pub trait UiNode: Sized {
    fn inner_mut(&mut self) -> &mut Node;

    fn width(mut self, width: Val) -> Self {
        self.inner_mut().width = width;
        self
    }

    fn height(mut self, height: Val) -> Self {
        self.inner_mut().height = height;
        self
    }

    fn padding(mut self, padding: Val) -> Self {
        self.inner_mut().padding = UiRect::all(padding);
        self
    }

    fn margin(mut self, margin: Val) -> Self {
        self.inner_mut().margin = UiRect::all(margin);
        self
    }

    fn border(mut self, border: Val) -> Self {
        self.inner_mut().border = UiRect::all(border);
        self
    }

    fn border_radius(mut self, border_radius: Val) -> Self {
        self.inner_mut().border_radius = BorderRadius::all(border_radius);
        self
    }

    fn justify_self(mut self, justify_self: JustifySelf) -> Self {
        self.inner_mut().justify_self = justify_self;
        self
    }

    fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.inner_mut().justify_content = justify_content;
        self
    }

    fn align_self(mut self, align_self: AlignSelf) -> Self {
        self.inner_mut().align_self = align_self;
        self
    }

    fn align_items(mut self, align_items: AlignItems) -> Self {
        self.inner_mut().align_items = align_items;
        self
    }

    fn edit_node(mut self, f: impl FnOnce(&mut Node)) -> Self {
        f(&mut self.inner_mut());
        self
    }
}

#[allow(dead_code)]
pub trait WithBackground: UiNode {
    fn bg_color_mut(&mut self) -> &mut Color;
    fn bg_shadow_mut(&mut self) -> &mut ShadowStyle;
    fn bg_gradient_mut(&mut self) -> &mut Option<Gradient>;
    fn border_color_mut(&mut self) -> &mut Color;

    /// Only visible together with a non-zero [`UiNode::border`] width.
    fn border_color(mut self, color: Color) -> Self {
        *self.border_color_mut() = color;
        self
    }

    fn bg_color(mut self, color: Color) -> Self {
        *self.bg_color_mut() = color;
        self
    }

    fn bg_gradient(mut self, gradient: Gradient) -> Self {
        self.bg_gradient_mut().replace(gradient);
        self
    }

    fn bg_shadow_radius(mut self, radius: Val) -> Self {
        self.bg_shadow_mut().spread_radius = radius;
        self
    }

    fn bg_shadow_offset(mut self, x: Val, y: Val) -> Self {
        self.bg_shadow_mut().x_offset = x;
        self.bg_shadow_mut().y_offset = y;
        self
    }

    fn bg_shadow_color(mut self, color: Color) -> Self {
        self.bg_shadow_mut().color = color;
        self
    }

    fn bg_shadow_blur_radius(mut self, radius: Val) -> Self {
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

    fn text(mut self, text: impl Into<String>) -> Self {
        self.text_mut().0 = text.into();
        self
    }

    fn text_color(mut self, color: Color) -> Self {
        self.text_color_mut().0 = color;
        self
    }

    fn text_shadow_color(mut self, color: Color) -> Self {
        *self.text_shadow_color_mut() = color;
        self
    }

    fn text_shadow_offset(mut self, offset: Vec2) -> Self {
        *self.text_shadow_offset_mut() = offset;
        self
    }

    fn font_size(mut self, size: FontSize) -> Self {
        self.font_mut().font_size = size;
        self
    }

    fn font_family(mut self, family: FontSource) -> Self {
        self.font_mut().font = family;
        self
    }

    fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font_mut().weight = weight;
        self
    }

    fn font_style(mut self, style: FontStyle) -> Self {
        self.font_mut().style = style;
        self
    }
}

impl UiNode for ButtonBuilder {
    fn inner_mut(&mut self) -> &mut Node {
        &mut self.inner
    }
}

impl UiNode for ContainerBuilder {
    fn inner_mut(&mut self) -> &mut Node {
        &mut self.inner
    }
}

impl WithBackground for ButtonBuilder {
    fn bg_color_mut(&mut self) -> &mut Color {
        &mut self.bg_color
    }

    fn border_color_mut(&mut self) -> &mut Color {
        &mut self.border_color
    }

    fn bg_shadow_mut(&mut self) -> &mut ShadowStyle {
        &mut self.bg_shadow
    }

    fn bg_gradient_mut(&mut self) -> &mut Option<Gradient> {
        &mut self.bg_gradient
    }
}

impl WithBackground for ContainerBuilder {
    fn bg_color_mut(&mut self) -> &mut Color {
        &mut self.bg_color
    }

    fn border_color_mut(&mut self) -> &mut Color {
        &mut self.border_color
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

#[derive(Component, Debug, Clone, PartialEq)]
pub enum UiElement {
    Container(UiContainer),
    Button(UiButton),
}

#[derive(Component, Debug, Clone, PartialEq)]
pub enum UiContainer {
    MainMenu,
    TabBar,
    Page(Tab),
    Section(Section),
    OutfitGrid,
    /// The row a cycler sits in, so single rows can be hidden.
    Row(Cycler),
}

/// The groups of controls on the pages, so each can be shown once it has
/// something in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Outfit,
    Animation,
    Camera,
    Environment,
    Image,
}

/// The values stepped through with `<` and `>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cycler {
    Animation,
    Camera,
    Environment,
    Brightness,
    Shadows,
    Tonemapping,
    Exposure,
    Bloom,
}

/// The text showing a cycler's current value.
#[derive(Component)]
pub struct CyclerValue(pub Cycler);

#[derive(Component, Debug, Clone, PartialEq)]
pub enum UiButton {
    HideMenu,
    RevealMenu,
    Exit,
    Tab(Tab),
    Skin(String),
    Previous(Cycler),
    Next(Cycler),
    RestoreLook,
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
    border_color: Color,
}

impl Default for ButtonBuilder {
    fn default() -> Self {
        Self {
            bg_shadow: ShadowStyle {
                color: Color::NONE,
                x_offset: Val::Px(0.0),
                y_offset: Val::Px(0.0),
                blur_radius: Val::Px(0.0),
                spread_radius: Val::Px(0.0),
            },
            bg_color: Color::default(),
            bg_gradient: None,
            border_color: Color::NONE,
            font: TextFont::default(),
            text_color: TextColor::default(),
            text_shadow_color: Color::default(),
            text_shadow_offset: Vec2::default(),
            inner: Node::default(),
            text: Text::default(),
        }
    }
}

#[allow(dead_code)]
impl ButtonBuilder {
    pub fn build(self) -> impl Bundle {
        let (base, bg_color, bg_gradient) = resolve_background(self.bg_color, &self.bg_gradient);

        (
            self.inner,
            BaseBackground(base),
            BoxShadow(vec![self.bg_shadow]),
            BorderColor::all(self.border_color),
            bg_color,
            bg_gradient,
            children![(
                self.text,
                self.font,
                self.text_color,
                Pickable::IGNORE,
                TextShadow { color: self.text_shadow_color, offset: self.text_shadow_offset }
            )],
        )
    }

    pub fn build_with(self, extras: impl Bundle) -> impl Bundle {
        (self.build(), extras)
    }
}

pub struct ContainerBuilder<C: Bundle = ()> {
    inner: Node,
    bg_color: Color,
    bg_shadow: ShadowStyle,
    bg_gradient: Option<Gradient>,
    border_color: Color,
    children: C,
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self {
            bg_shadow: ShadowStyle {
                color: Color::NONE,
                x_offset: Val::Px(0.0),
                y_offset: Val::Px(0.0),
                blur_radius: Val::Px(0.0),
                spread_radius: Val::Px(0.0),
            },
            bg_color: Color::default(),
            bg_gradient: None,
            border_color: Color::NONE,
            children: (),
            inner: Node::default(),
        }
    }
}

#[allow(dead_code)]
impl<C: Bundle> ContainerBuilder<C> {
    pub fn children<C2: Bundle>(self, children: C2) -> ContainerBuilder<C2> {
        ContainerBuilder {
            inner: self.inner,
            bg_color: self.bg_color,
            bg_shadow: self.bg_shadow,
            bg_gradient: self.bg_gradient,
            border_color: self.border_color,
            children,
        }
    }

    pub fn build(self) -> impl Bundle {
        let (base, bg_color, bg_gradient) = resolve_background(self.bg_color, &self.bg_gradient);

        (
            self.inner,
            BaseBackground(base),
            BoxShadow(vec![self.bg_shadow]),
            BorderColor::all(self.border_color),
            bg_color,
            bg_gradient,
            self.children,
            Pickable::IGNORE,
        )
    }

    pub fn build_with(self, extras: impl Bundle) -> impl Bundle {
        (self.build(), extras)
    }
}

fn resolve_background(
    color: Color,
    gradient: &Option<Gradient>,
) -> (Background, BackgroundColor, BackgroundGradient) {
    match gradient {
        Some(g) => (
            Background::Gradient(g.clone()),
            BackgroundColor(Color::NONE),
            BackgroundGradient(vec![g.clone()]),
        ),
        None => (Background::Color(color), BackgroundColor(color), BackgroundGradient::default()),
    }
}

fn feedback<E>(delta: f32) -> impl Fn(On<Pointer<E>>, FeedbackQuery)
where
    E: std::fmt::Debug + Clone + Reflect,
{
    move |event, mut query| {
        let Ok((base, mut color, mut gradient)) = query.get_mut(event.original_event_target())
        else {
            return;
        };

        match base.0.clone().adjust_brightness(delta) {
            Background::Color(c) => color.0 = c,
            Background::Gradient(g) => gradient.0 = vec![g],
        }
    }
}
