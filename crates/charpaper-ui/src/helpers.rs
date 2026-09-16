use bevy::prelude::*;

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

pub fn button(
    size: (f32, f32),
    text: &str,
    text_size: f32,
    color: (f32, f32, f32, f32),
    padding: f32,
    marker: impl Component,
) -> impl Bundle {
    (
        Node {
            width: px(size.0),
            height: px(size.1),
            padding: UiRect::all(px(padding)),
            justify_self: JustifySelf::Center,
            justify_content: JustifyContent::Center,
            align_self: AlignSelf::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::Srgba(Srgba {
            red: color.0,
            green: color.1,
            blue: color.2,
            alpha: color.3,
        })),
        children![(Text::new(text), TextFont { font_size: FontSize::Px(text_size), ..default() })],
        marker,
    )
}
