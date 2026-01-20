use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

// === Modern Dark Theme Color Palette ===

#[allow(dead_code)]
// Background colors - Light theme
fn color_bg() -> Color {
    Color::from_rgb8(248, 250, 252) // Very light slate background
}

// Panel colors with layering
fn color_panel() -> Color {
    Color::from_rgb8(255, 255, 255) // Pure white panels
}

fn color_panel_elevated() -> Color {
    Color::from_rgba8(248, 250, 252, 0.9) // Very light slate with transparency
}

fn color_panel_alt() -> Color {
    Color::from_rgb8(250, 252, 254) // Alternative light panel
}

// Border colors
fn color_border() -> Color {
    Color::from_rgb8(226, 232, 240) // slate-200
}

// Text colors - darker for light background

fn color_text_muted() -> Color {
    Color::from_rgb8(100, 116, 139) // slate-500
}

// Accent colors - vibrant but work on light bg
fn color_accent() -> Color {
    Color::from_rgb8(14, 165, 233) // sky-500
}

fn color_accent_dark() -> Color {
    Color::from_rgb8(3, 105, 161) // sky-700
}

fn color_accent_soft() -> Color {
    Color::from_rgba8(224, 242, 254, 0.8) // sky-100 with opacity
}

// Status colors

// === Container Styles ===

pub fn app_background(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_bg())),
        ..container::Style::default()
    }
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel())),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.05), // Much lighter shadow
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..container::Style::default()
    }
}

pub fn muted_text(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(color_text_muted()),
    }
}

pub fn header_text(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Color::BLACK),
    }
}

// iTerm2-style compact tab
pub fn compact_tab(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let mut style = button::Style {
            background: if active {
                Some(Background::Color(color_accent_soft()))
            } else {
                None
            },
            text_color: if active {
                color_accent_dark()
            } else {
                color_text_muted()
            },
            border: Border {
                color: if active {
                    color_accent()
                } else {
                    Color::TRANSPARENT
                },
                width: 0.0,
                radius: 8.0.into(),
            },
            shadow: if active {
                Shadow {
                    color: Color::from_rgba8(14, 165, 233, 0.15),
                    offset: Vector::new(0.0, 0.0),
                    blur_radius: 8.0,
                }
            } else {
                Shadow::default()
            },
            ..button::Style::default()
        };

        if let button::Status::Hovered = status {
            if !active {
                style.background = Some(Background::Color(color_panel_elevated()));
                style.text_color = color_accent_dark();
            }
        }

        style
    }
}

// Tab close button (small × button)
pub fn tab_close_button(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        text_color: color_text_muted(),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    };

    if let button::Status::Hovered = status {
        style.background = Some(Background::Color(Color::from_rgb8(239, 68, 68)));
        style.text_color = Color::WHITE;
    }

    style
}

// New tab button (+)
pub fn new_tab_button(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: Some(Background::Color(color_panel_elevated())),
        text_color: color_accent(),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..button::Style::default()
    };

    if let button::Status::Hovered = status {
        style.background = Some(Background::Color(color_accent_soft()));
        style.border.color = color_accent();
    }

    style
}

pub fn save_button(theme: &Theme, status: button::Status) -> button::Style {
    new_tab_button(theme, status)
}

// Menu button (≡)
pub fn menu_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let mut style = button::Style {
            background: if active {
                Some(Background::Color(color_accent_soft()))
            } else {
                Some(Background::Color(color_panel_elevated()))
            },
            text_color: if active {
                color_accent()
            } else {
                color_text_muted()
            },
            border: Border {
                color: if active {
                    color_accent()
                } else {
                    color_border()
                },
                width: 1.0,
                radius: 6.0.into(),
            },
            ..button::Style::default()
        };

        if let button::Status::Hovered = status {
            style.border.color = color_accent();
            style.text_color = color_accent();
        }

        style
    }
}

// Tab bar container
pub fn tab_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel())),
        border: Border {
            color: color_border(),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

// Terminal content area
pub fn terminal_content(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(255, 255, 255))), // White terminal bg
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

// Status bar
pub fn status_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel_alt())),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

// Menu item button
pub fn menu_item(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        text_color: color_text_muted(),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        ..button::Style::default()
    };

    if let button::Status::Hovered = status {
        style.background = Some(Background::Color(color_accent_soft()));
        style.text_color = color_accent_dark();
    }

    style
}

// Menu divider line
pub fn menu_divider(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_border())),
        ..container::Style::default()
    }
}

// Dropdown menu container (left sidebar)
pub fn dropdown_menu(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel())),
        border: Border {
            color: color_border(),
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            offset: Vector::new(4.0, 0.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}
