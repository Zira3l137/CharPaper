//! Colours and sizes from the settings design (the "CharPaper Settings Menu"
//! canvas), so every widget draws from the same few values.

use bevy::prelude::*;

pub(crate) const PANEL_BG: Color = Color::srgba_u8(18, 20, 26, 240);
pub(crate) const BORDER: Color = Color::srgb_u8(0x2E, 0x32, 0x40);
pub(crate) const BUTTON_BG: Color = Color::srgb_u8(0x1D, 0x20, 0x29);
pub(crate) const WELL_BG: Color = Color::srgb_u8(0x15, 0x17, 0x1D);
pub(crate) const SELECTED_BG: Color = Color::srgb_u8(0x1B, 0x35, 0x50);
pub(crate) const ACCENT: Color = Color::srgb_u8(0x5A, 0xA6, 0xF2);
pub(crate) const ON_ACCENT: Color = Color::srgb_u8(0x08, 0x11, 0x1C);
pub(crate) const TEXT: Color = Color::srgb_u8(0xE6, 0xE8, 0xEE);
pub(crate) const TEXT_DIM: Color = Color::srgb_u8(0xA0, 0xA7, 0xB4);
pub(crate) const TEXT_LABEL: Color = Color::srgb_u8(0xC9, 0xCD, 0xD6);
pub(crate) const DANGER_BORDER: Color = Color::srgb_u8(0x4A, 0x2A, 0x2E);
pub(crate) const DANGER_TEXT: Color = Color::srgb_u8(0xF2, 0x9A, 0x9A);

pub(crate) const PANEL_WIDTH: Val = Val::Px(380.0);
pub(crate) const PANEL_MARGIN: Val = Val::Px(16.0);
pub(crate) const PANEL_RADIUS: Val = Val::Px(6.0);
pub(crate) const RADIUS: Val = Val::Px(4.0);
pub(crate) const LINE: Val = Val::Px(1.0);
pub(crate) const CONTROL_HEIGHT: Val = Val::Px(32.0);
pub(crate) const LABEL_WIDTH: Val = Val::Px(120.0);

pub(crate) const TITLE_SIZE: FontSize = FontSize::Px(14.0);
pub(crate) const BODY_SIZE: FontSize = FontSize::Px(13.0);
pub(crate) const CONTROL_SIZE: FontSize = FontSize::Px(12.0);
pub(crate) const SMALL_SIZE: FontSize = FontSize::Px(11.0);
pub(crate) const GLYPH_SIZE: FontSize = FontSize::Px(14.0);
