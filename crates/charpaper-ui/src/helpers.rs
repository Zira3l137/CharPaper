use bevy::prelude::*;

pub mod defaults {
    #![allow(dead_code)]
    use bevy::prelude::*;

    pub(crate) const BTN_PADDING: Val = Val::Px(10.0);
    pub(crate) const BTN_RADIUS: Val = Val::Px(10.0);
    pub(crate) const BTN_BG_COLOR: Color = Color::srgba(0.1, 0.7, 0.1, 1.0);
    pub(crate) const BTN_TEXT_SIZE: FontSize = FontSize::Px(16.0);
    pub(crate) const BTN_WIDTH: Val = Val::Px(200.0);
    pub(crate) const BTN_HEIGHT: Val = Val::Px(50.0);
}

const HOVER_DELTA: f32 = 0.08;
const PRESS_DELTA: f32 = -0.08;

pub trait ColorExt: Into<Hsla> {
    fn adjust_brightness(self, delta: f32) -> Color {
        let hsla: Hsla = self.into();
        Color::from(hsla.with_lightness((hsla.lightness + delta).clamp(0.0, 1.0)))
    }
}

pub trait ButtonReactiveExt {
    fn with_button_feedback(&mut self) -> &mut Self;
}

impl ButtonReactiveExt for EntityCommands<'_> {
    fn with_button_feedback(&mut self) -> &mut Self {
        self.observe(on_hover_start).observe(on_hover_end).observe(on_press).observe(on_release)
    }
}

impl ColorExt for Color {}

#[derive(Component, Debug)]
pub enum UiButton {
    Exit,
}

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct BaseColor(pub Color);

#[derive(Default)]
pub struct ButtonBuilder {
    inner: Node,
    text: Text,
    font: TextFont,
    text_color: TextColor,
    bg_color: Color,
}

#[allow(dead_code)]
impl ButtonBuilder {
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text.0 = text.into();
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color.0 = color;
        self
    }

    pub fn font_size(mut self, size: FontSize) -> Self {
        self.font.font_size = size;
        self
    }

    pub fn font_family(mut self, family: FontSource) -> Self {
        self.font.font = family;
        self
    }

    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font.weight = weight;
        self
    }

    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.font.style = style;
        self
    }

    pub fn width(mut self, width: Val) -> Self {
        self.inner.width = width;
        self
    }

    pub fn height(mut self, height: Val) -> Self {
        self.inner.height = height;
        self
    }

    pub fn padding(mut self, padding: Val) -> Self {
        self.inner.padding = UiRect::all(padding);
        self
    }

    pub fn margin(mut self, margin: Val) -> Self {
        self.inner.margin = UiRect::all(margin);
        self
    }

    pub fn border(mut self, border: Val) -> Self {
        self.inner.border = UiRect::all(border);
        self
    }

    pub fn border_radius(mut self, border_radius: Val) -> Self {
        self.inner.border_radius = BorderRadius::all(border_radius);
        self
    }

    pub fn justify_self(mut self, justify_self: JustifySelf) -> Self {
        self.inner.justify_self = justify_self;
        self
    }

    pub fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.inner.justify_content = justify_content;
        self
    }

    pub fn align_self(mut self, align_self: AlignSelf) -> Self {
        self.inner.align_self = align_self;
        self
    }

    pub fn align_items(mut self, align_items: AlignItems) -> Self {
        self.inner.align_items = align_items;
        self
    }

    pub fn edit_node(mut self, f: impl FnOnce(&mut Node)) -> Self {
        f(&mut self.inner);
        self
    }

    pub fn build_marked(self, marker: impl Component) -> impl Bundle {
        (
            self.inner,
            BackgroundColor(self.bg_color),
            BaseColor(self.bg_color),
            children![(self.text, self.font, self.text_color)],
            marker,
        )
    }

    pub fn build(self) -> impl Bundle {
        (
            self.inner,
            BackgroundColor(self.bg_color),
            BaseColor(self.bg_color),
            children![(self.text, self.font, self.text_color)],
        )
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

fn on_hover_start(
    event: On<Pointer<Over>>,
    mut query: Query<(&BaseColor, &mut BackgroundColor), With<UiButton>>,
) {
    info!("feedback: hover start: {:?}", event.entity);
    if let Ok((base, mut bg)) = query.get_mut(event.entity) {
        bg.0 = base.0.adjust_brightness(HOVER_DELTA);
    }
}

fn on_hover_end(
    event: On<Pointer<Out>>,
    mut query: Query<(&BaseColor, &mut BackgroundColor), With<UiButton>>,
) {
    info!("feedback: hover end: {:?}", event.entity);
    if let Ok((base, mut bg)) = query.get_mut(event.entity) {
        bg.0 = base.0;
    }
}

fn on_press(
    event: On<Pointer<Press>>,
    mut query: Query<(&BaseColor, &mut BackgroundColor), With<UiButton>>,
) {
    info!("feedback: press: {:?}", event.entity);
    if let Ok((base, mut bg)) = query.get_mut(event.entity) {
        bg.0 = base.0.adjust_brightness(PRESS_DELTA);
    }
}

fn on_release(event: On<Pointer<Release>>, mut query: Query<(&BaseColor, &mut BackgroundColor)>) {
    info!("feedback: release: {:?}", event.entity);
    if let Ok((base, mut bg)) = query.get_mut(event.entity) {
        // Back to hover brightness since cursor is still over it on release
        bg.0 = base.0.adjust_brightness(HOVER_DELTA);
    }
}
