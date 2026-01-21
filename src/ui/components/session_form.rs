use crate::session::SessionConfig;
use crate::ui::Message;
use crate::ui::style as ui_style;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    editing_session: Option<&'a SessionConfig>,
    saved_sessions: &'a [SessionConfig],
    form_name: &'a str,
    form_host: &'a str,
    form_port: &'a str,
    form_username: &'a str,
    form_password: &'a str,
    auth_method_password: bool,
    validation_error: Option<&'a String>,
) -> Element<'a, Message> {
    let is_new = editing_session
        .as_ref()
        .map(|s| !saved_sessions.iter().any(|saved| saved.id == s.id))
        .unwrap_or(false);

    let title = if is_new {
        "New Session"
    } else {
        "Edit Session"
    };

    let form_header = row![
        text(title).size(15),
        container("").width(Length::Fill),
        button(text("Save").size(12))
            .padding([5, 10])
            .style(ui_style::new_tab_button)
            .on_press(Message::SaveSession),
        button(text("Cancel").size(12))
            .padding([5, 10])
            .style(ui_style::tab_close_button)
            .on_press(Message::CancelSessionEdit),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .padding(iced::Padding::default().bottom(10));

    let error_banner = if let Some(ref error) = validation_error {
        container(
            text(format!("⚠️ {}", error))
                .size(12)
                .color(iced::Color::from_rgb(0.8, 0.2, 0.2)),
        )
        .padding(10)
        .width(Length::Fill)
        .style(ui_style::panel)
    } else {
        container("")
    };

    column![
        form_header,
        error_banner,
        container("").height(8.0),
        text("Name").size(11).style(ui_style::muted_text),
        text_input("Production Server", form_name)
            .on_input(Message::SessionNameChanged)
            .padding(8)
            .size(12),
        container("").height(8.0),
        text("Host").size(11).style(ui_style::muted_text),
        text_input("example.com", form_host)
            .on_input(Message::SessionHostChanged)
            .padding(8)
            .size(12),
        container("").height(8.0),
        row![
            column![
                text("Port").size(11).style(ui_style::muted_text),
                text_input("22", form_port)
                    .on_input(Message::SessionPortChanged)
                    .padding(8)
                    .size(12)
                    .width(Length::Fixed(80.0)),
            ]
            .spacing(3),
            container("").width(Length::Fixed(12.0)),
            column![
                text("Username").size(11).style(ui_style::muted_text),
                text_input("user", form_username)
                    .on_input(Message::SessionUsernameChanged)
                    .padding(8)
                    .size(12)
                    .width(Length::Fill),
            ]
            .spacing(3)
            .width(Length::Fill),
        ],
        container("").height(8.0),
        text("Authentication").size(11).style(ui_style::muted_text),
        row![
            button(text("🔑 Private Key").size(11))
                .padding([6, 12])
                .style(move |theme, status| {
                    if !auth_method_password {
                        ui_style::new_tab_button(theme, status)
                    } else {
                        (ui_style::menu_button(false))(theme, status)
                    }
                })
                .on_press(if auth_method_password {
                    Message::ToggleAuthMethod
                } else {
                    Message::ToggleAuthMethod // dummy, won't toggle if already selected
                }),
            button(text("🔒 Password").size(11))
                .padding([6, 12])
                .style(move |theme, status| {
                    if auth_method_password {
                        ui_style::new_tab_button(theme, status)
                    } else {
                        (ui_style::menu_button(false))(theme, status)
                    }
                })
                .on_press(if !auth_method_password {
                    Message::ToggleAuthMethod
                } else {
                    Message::ToggleAuthMethod // dummy
                }),
        ]
        .spacing(6),
        container("").height(8.0),
        if !auth_method_password {
            column![
                text("Private Key Path")
                    .size(11)
                    .style(ui_style::muted_text),
                text_input("~/.ssh/id_rsa", "~/.ssh/id_rsa")
                    .padding(8)
                    .size(12),
            ]
            .spacing(3)
        } else {
            column![
                text("Password").size(11).style(ui_style::muted_text),
                text_input("", form_password)
                    .on_input(Message::SessionPasswordChanged)
                    .padding(8)
                    .size(12)
                    .secure(true),
            ]
            .spacing(3)
        },
    ]
    .spacing(3)
    .into()
}
