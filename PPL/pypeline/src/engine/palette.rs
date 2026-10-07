//! PROVISIONAL master palette.
//!
//! Every channel is a multiple of 8, i.e. a real GBA 15-bit color (5 bits per
//! channel). This is a placeholder until the art pass locks the real palette
//! in assets/palettes/master.pal and docs/ART_STYLE.md.

use bevy::prelude::Color;

const fn gba(r: u8, g: u8, b: u8) -> Color {
    Color::srgb_u8(r, g, b)
}

pub const SKY: Color = gba(120, 192, 248);
pub const SKY_DEEP: Color = gba(64, 120, 200);
pub const LETTERBOX: Color = gba(16, 16, 32);

pub const GRASS_LIGHT: Color = gba(136, 208, 96);
pub const GRASS: Color = gba(104, 184, 72);
pub const GRASS_DARK: Color = gba(56, 128, 48);

pub const DIRT: Color = gba(168, 112, 64);
pub const DIRT_DARK: Color = gba(112, 72, 40);

pub const METAL_LIGHT: Color = gba(184, 192, 200);
pub const METAL: Color = gba(144, 152, 168);

pub const BRASS: Color = gba(224, 168, 56);
pub const COPPER: Color = gba(200, 112, 64);
pub const OUTLINE: Color = gba(40, 32, 48);

/// FireRed-style dialogue box colors, used by the egui theme.
pub const UI_CREAM: [u8; 3] = [248, 240, 216];
pub const UI_INK: [u8; 3] = [40, 32, 48];
pub const UI_BORDER: [u8; 3] = [72, 96, 152];
pub const UI_ERROR: [u8; 3] = [200, 48, 48];
pub const UI_INFO: [u8; 3] = [72, 96, 152];
