use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};
use iced::widget::scrollable;

// === Modern Dark Theme Color Palette ===

#[allow(dead_code)]
// Background colors - Light theme
fn color_bg() -> Color {
    Color::from_rgb8(245, 245, 247) // macOS window background
}

// Panel colors with layering
fn color_panel() -> Color {
    Color::from_rgb8(255, 255, 255)
}

fn color_panel_elevated() -> Color {
    Color::from_rgb8(249, 250, 251)
}

fn color_panel_alt() -> Color {
    Color::from_rgb8(242, 242, 244)
}

// Border colors
fn color_border() -> Color {
    Color::from_rgb8(229, 229, 231)
}

// Text colors - darker for light background

fn color_text_muted() -> Color {
    Color::from_rgb8(110, 110, 115)
}

// Accent colors - vibrant but work on light bg
fn color_accent() -> Color {
    Color::from_rgb8(10, 132, 255)
}

fn color_accent_dark() -> Color {
    Color::from_rgb8(0, 96, 223)
}

fn color_accent_soft() -> Color {
    Color::from_rgba8(10, 132, 255, 0.12)
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

pub fn primary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => color_accent_dark(),
            _ => color_accent(),
        })),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}

pub fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb8(237, 238, 240),
            _ => Color::from_rgb8(247, 248, 249),
        })),
        text_color: Color::from_rgb8(60, 60, 67),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn destructive_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Color::from_rgb8(236, 86, 80),
        _ => Color::from_rgb8(246, 96, 90),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
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
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.04),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 6.0,
        },
        ..container::Style::default()
    }
}

pub fn drawer_panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel())),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: iced::border::Radius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: 12.0,
                bottom_left: 0.0,
            },
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
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
        color: Some(Color::from_rgb8(28, 28, 30)),
    }
}

// iTerm2-style compact tab
pub fn compact_tab(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let mut style = button::Style {
            background: if active {
                Some(Background::Color(color_panel_elevated()))
            } else {
                None
            },
            text_color: if active {
                Color::from_rgb8(28, 28, 30)
            } else {
                color_text_muted()
            },
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            ..button::Style::default()
        };

        if let button::Status::Hovered = status {
            if !active {
                style.background = Some(Background::Color(color_panel_elevated()));
                style.text_color = Color::from_rgb8(28, 28, 30);
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
        style.background = Some(Background::Color(Color::from_rgb8(232, 71, 68)));
        style.text_color = Color::WHITE;
    }

    style
}

pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
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
        style.background = Some(Background::Color(color_panel_elevated()));
        style.text_color = Color::from_rgb8(28, 28, 30);
    }

    style
}


// New tab button (+)
pub fn new_tab_button(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: Some(Background::Color(color_panel())),
        text_color: Color::from_rgb8(60, 60, 67),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..button::Style::default()
    };

    if let button::Status::Hovered = status {
        style.background = Some(Background::Color(color_panel_elevated()));
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
                Some(Background::Color(color_panel_elevated()))
            } else {
                Some(Background::Color(color_panel()))
            },
            text_color: if active {
                color_accent_dark()
            } else {
                color_text_muted()
            },
            border: Border {
                color: color_border(),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..button::Style::default()
        };

        if let button::Status::Hovered = status {
            style.background = Some(Background::Color(color_panel_elevated()));
            style.text_color = Color::from_rgb8(28, 28, 30);
        }

        style
    }
}

pub fn menu_button_disabled() -> impl Fn(&Theme, button::Status) -> button::Style {
    |_theme, _status| button::Style {
        background: Some(Background::Color(color_panel())),
        text_color: color_text_muted(),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..button::Style::default()
    }
}

// Tab bar container
pub fn tab_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel())),
        border: Border {
            color: color_border(),
            width: 1.0,
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
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

// Status bar
pub fn status_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel_alt())),
        border: Border {
            color: color_border(),
            width: 1.0,
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
        background: Some(Background::Color(color_panel_alt())),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

pub fn popover_menu(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color_panel())),
        border: Border {
            color: color_border(),
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 10.0,
        },
        ..container::Style::default()
    }
}

pub fn menu_item_button(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        text_color: Color::from_rgb8(60, 60, 67),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        ..button::Style::default()
    };

    if let button::Status::Hovered = status {
        style.background = Some(Background::Color(color_panel_elevated()));
        style.text_color = Color::from_rgb8(28, 28, 30);
    }

    style
}

pub fn sftp_row_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let mut style = button::Style {
            background: if selected {
                Some(Background::Color(color_accent_soft()))
            } else {
                None
            },
            text_color: Color::from_rgb8(60, 60, 67),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 6.0.into(),
            },
            ..button::Style::default()
        };

        if let button::Status::Hovered = status {
            if !selected {
                style.background = Some(Background::Color(color_panel_elevated()));
            }
        }

        style
    }
}

pub fn menu_item_destructive(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        text_color: Color::from_rgb8(210, 60, 60),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        ..button::Style::default()
    };

    if let button::Status::Hovered = status {
        style.background = Some(Background::Color(color_panel_elevated()));
        style.text_color = Color::from_rgb8(190, 50, 50);
    }

    style
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

pub fn scrollable_style(
    _theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{Rail, Scroller, Status, Style};

    let (alpha, border_alpha, disabled) = match status {
        Status::Dragged { .. } => (0.75, 0.5, false),
        Status::Hovered {
            is_vertical_scrollbar_disabled,
            is_horizontal_scrollbar_disabled,
            ..
        } => (0.55, 0.35, is_vertical_scrollbar_disabled && is_horizontal_scrollbar_disabled),
        Status::Active {
            is_vertical_scrollbar_disabled,
            is_horizontal_scrollbar_disabled,
        } => (0.0, 0.0, is_vertical_scrollbar_disabled && is_horizontal_scrollbar_disabled),
    };

    let scroller_color = if disabled || alpha == 0.0 {
        Color::TRANSPARENT
    } else {
        Color::from_rgba8(120, 120, 125, alpha)
    };
    let scroller_border = if disabled || border_alpha == 0.0 {
        Color::TRANSPARENT
    } else {
        Color::from_rgba8(120, 120, 125, border_alpha)
    };

    let rail = Rail {
        background: None,
        border: iced::border::rounded(0),
        scroller: Scroller {
            background: Background::Color(scroller_color),
            border: iced::border::rounded(6).color(scroller_border),
        },
    };

    Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: Background::Color(Color::from_rgba8(0, 0, 0, 0.05)),
            border: iced::border::rounded(999),
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 2.0,
            },
            icon: Color::from_rgba8(80, 80, 85, 0.7),
        },
    }
}

pub fn thin_scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(6)
            .scroller_width(6)
            .margin(2),
    )
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

pub fn dialog_input(
    _theme: &Theme,
    _status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    use iced::widget::text_input;

    text_input::Style {
        background: Background::Color(Color::WHITE),
        border: Border {
            color: Color::from_rgb8(218, 220, 224),
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: Color::from_rgb8(100, 116, 139),
        placeholder: Color::from_rgb8(148, 163, 184),
        value: Color::from_rgb8(28, 28, 30),
        selection: color_accent(),
    }
}

pub fn tooltip_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(25, 25, 28, 0.96))),
        border: Border {
            color: Color::from_rgba8(255, 255, 255, 0.08),
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

pub fn tooltip_text(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Color::from_rgb8(245, 245, 247)),
    }
}

pub fn ime_input(
    _theme: &Theme,
    _status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
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
            button::Status::Hovered => Color::from_rgb8(232, 233, 235),
            _ => Color::from_rgb8(236, 237, 240),
        })),
        text_color: Color::from_rgb8(28, 28, 30),
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
            button::Status::Hovered => Color::from_rgb8(240, 241, 244),
            _ => Color::TRANSPARENT,
        })),
        text_color: Color::from_rgb8(60, 60, 67),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}

pub fn sidebar_section_header(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(color_text_muted()),
    }
}

pub fn sidebar_recent_item(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb8(240, 241, 244),
            _ => Color::TRANSPARENT,
        })),
        text_color: color_text_muted(),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}
