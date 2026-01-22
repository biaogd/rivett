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

// === Dialog and Modal Styles ===

pub fn dialog_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel())),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..container::Style::default()
    }
}

pub fn active_tab_header(_theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: color_accent(),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn inactive_tab_header(_theme: &Theme) -> container::Style {
    container::Style {
        ..container::Style::default()
    }
}

pub fn error_banner(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(1.0, 0.95, 0.95))),
        border: Border {
            color: Color::from_rgb(0.9, 0.6, 0.6),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn label_text(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(color_text_muted()),
    }
}

pub fn primary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb(0.02, 0.5, 0.88),
            _ => color_accent(),
        })),
        text_color: Color::WHITE,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}

pub fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb(0.96, 0.96, 0.96),
            _ => Color::from_rgb(0.98, 0.98, 0.98),
        })),
        text_color: Color::from_rgb(0.3, 0.3, 0.3),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn divider(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_border())),
        ..container::Style::default()
    }
}

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

// Quick Connect Popover
pub fn quick_connect_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

pub fn quick_connect_item(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        text_color: Color::BLACK,
        ..button::Style::default()
    };

    if let button::Status::Hovered = status {
        style.background = Some(Background::Color(color_panel_elevated()));
    }

    style
}

pub fn modal_backdrop(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
        ..button::Style::default()
    }
}

#[allow(dead_code)]
pub fn search_bar_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(248, 250, 252))), // slate-50
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

#[allow(dead_code)]
pub fn transparent(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        ..button::Style::default()
    }
}

pub fn quick_connect_section_header(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Color::from_rgb8(148, 163, 184)), // slate-400
    }
}

pub fn quick_connect_footer_hint(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Color::from_rgb8(148, 163, 184)), // slate-400
    }
}

pub fn search_input(
    _theme: &Theme,
    _status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    use iced::widget::text_input;

    text_input::Style {
        background: Background::Color(Color::WHITE),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: Color::from_rgb8(100, 116, 139),
        placeholder: Color::from_rgb8(148, 163, 184),
        value: Color::BLACK,
        selection: Color::from_rgb8(14, 165, 233),
    }
}

pub fn ime_input(_theme: &Theme, _status: iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    use iced::widget::text_input;

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: Color::TRANSPARENT,
        placeholder: Color::TRANSPARENT,
        value: Color::TRANSPARENT,
        selection: Color::TRANSPARENT,
    }
}

// === Sidebar Styles ===

#[allow(dead_code)]
pub fn sidebar_search_input(
    _theme: &Theme,
    _status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    use iced::widget::text_input;

    text_input::Style {
        background: Background::Color(Color::from_rgb(0.97, 0.97, 0.97)),
        border: Border {
            color: Color::from_rgb(0.9, 0.9, 0.9),
            width: 1.0,
            radius: 6.0.into(),
        },
        icon: Color::from_rgb8(100, 116, 139),
        placeholder: Color::from_rgb8(148, 163, 184),
        value: Color::from_rgb8(50, 50, 50),
        selection: color_accent(),
    }
}

pub fn sidebar_button_active(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb(0.92, 0.92, 0.92),
            _ => Color::from_rgb(0.95, 0.95, 0.95),
        })),
        text_color: Color::from_rgb(0.2, 0.2, 0.2),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}

pub fn sidebar_button_inactive(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb(0.95, 0.95, 0.95),
            _ => Color::TRANSPARENT,
        })),
        text_color: Color::from_rgb(0.3, 0.3, 0.3),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}

pub fn sidebar_section_header(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Color::from_rgb8(100, 116, 139)), // slate-500
    }
}

pub fn sidebar_recent_item(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb(0.96, 0.96, 0.96),
            _ => Color::TRANSPARENT,
        })),
        text_color: Color::from_rgb(0.4, 0.4, 0.4),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}
