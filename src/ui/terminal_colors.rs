use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use iced::Color;

pub fn convert_color(color: AnsiColor) -> Color {
    match color {
        AnsiColor::Named(named) => match named {
            NamedColor::Black => Color::BLACK,
            NamedColor::Red => Color::from_rgb8(205, 49, 49),
            NamedColor::Green => Color::from_rgb8(13, 188, 121),
            NamedColor::Yellow => Color::from_rgb8(180, 160, 0),
            NamedColor::Blue => Color::from_rgb8(36, 114, 200),
            NamedColor::Magenta => Color::from_rgb8(188, 63, 188),
            NamedColor::Cyan => Color::from_rgb8(0, 150, 200),
            NamedColor::White => Color::from_rgb8(240, 240, 240),
            NamedColor::Foreground => Color::BLACK,
            NamedColor::Background => Color::WHITE,
            _ => Color::BLACK,
        },
        AnsiColor::Spec(rgb) => Color::from_rgb8(rgb.r, rgb.g, rgb.b),
        AnsiColor::Indexed(idx) => convert_indexed_color(idx),
    }
}

pub fn convert_indexed_color(idx: u8) -> Color {
    const ANSI_16: [Color; 16] = [
        Color::from_rgb8(0, 0, 0),
        Color::from_rgb8(205, 49, 49),
        Color::from_rgb8(13, 188, 121),
        Color::from_rgb8(180, 160, 0),
        Color::from_rgb8(36, 114, 200),
        Color::from_rgb8(188, 63, 188),
        Color::from_rgb8(0, 150, 200),
        Color::from_rgb8(229, 229, 229),
        Color::from_rgb8(85, 85, 85),
        Color::from_rgb8(255, 95, 95),
        Color::from_rgb8(100, 215, 140),
        Color::from_rgb8(255, 215, 95),
        Color::from_rgb8(95, 175, 255),
        Color::from_rgb8(215, 95, 255),
        Color::from_rgb8(95, 215, 255),
        Color::from_rgb8(245, 245, 245),
    ];

    match idx {
        0..=15 => ANSI_16[idx as usize],
        16..=231 => {
            let idx = idx - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            let scale = [0, 95, 135, 175, 215, 255];
            Color::from_rgb8(scale[r as usize], scale[g as usize], scale[b as usize])
        }
        232..=255 => {
            let gray = 8 + (idx - 232) * 10;
            Color::from_rgb8(gray, gray, gray)
        }
    }
}
