//! A single place for the color palette, so every screen draws from the
//! same set of colors instead of each `view` file redefining its own
//! `Color::from_rgb8(...)` constants (previously scattered, occasionally
//! inconsistent). `ACCENT` matches the app icon's own color.

use iced::Color;

pub fn accent() -> Color {
    Color::from_rgb8(0x6D, 0x5D, 0xFB)
}

pub fn accent_dim() -> Color {
    Color::from_rgb8(0x4C, 0x3D, 0xE3)
}

pub fn success() -> Color {
    Color::from_rgb8(0x4C, 0xAF, 0x50)
}

pub fn warning() -> Color {
    Color::from_rgb8(0xFF, 0xB7, 0x4D)
}

pub fn danger() -> Color {
    Color::from_rgb8(0xE5, 0x73, 0x73)
}

pub fn info() -> Color {
    Color::from_rgb8(0x64, 0xB5, 0xF6)
}

pub fn text_dim() -> Color {
    Color::from_rgb8(0x9E, 0x9E, 0x9E)
}
