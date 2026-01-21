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
    _form_password: &'a str,
    _auth_method_password: bool,
    validation_error: Option<&'a String>,
) -> Element<'a, Message> {
    let is_new = editing_session
        .map(|s| !saved_sessions.iter().any(|saved| saved.id == s.id))
        .unwrap_or(true);

    let title = if is_new {
        "New Session"
    } else {
        "Edit Session"
    };
    let subtitle = if is_new {
        "Configure a new SSH connection to your remote server."
    } else {
        "Update your SSH connection configuration."
    };

    // Header with title and close button
    let header = row![
        column![
            text(title).size(20),
            text(subtitle).size(13).style(ui_style::muted_text),
        ]
        .spacing(4),
        container("").width(Length::Fill),
        button(text("✕").size(16))
            .padding(8)
            .style(ui_style::tab_close_button)
            .on_press(Message::CancelSessionEdit),
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    // Tab-like headers (currently just showing GENERAL)
    let tabs = row![
        container(text("GENERAL").size(11))
            .padding([8, 16])
            .style(ui_style::active_tab_header),
        container(text("AUTHENTICATION").size(11))
            .padding([8, 16])
            .style(ui_style::inactive_tab_header),
        container(text("ADVANCED").size(11))
            .padding([8, 16])
            .style(ui_style::inactive_tab_header),
    ]
    .spacing(0);

    // Error banner
    let error_banner = if let Some(error) = validation_error {
        container(
            text(format!("⚠️ {}", error))
                .size(12)
                .color(iced::Color::from_rgb(0.9, 0.3, 0.3)),
        )
        .padding(12)
        .width(Length::Fill)
        .style(ui_style::error_banner)
    } else {
        container("").height(0.0)
    };

    // Form fields
    let form_content = column![
        // Display Name
        column![
            text("DISPLAY NAME").size(11).style(ui_style::label_text),
            text_input("Production Server 01", form_name)
                .on_input(Message::SessionNameChanged)
                .padding(10)
                .size(12),
        ]
        .spacing(6),
        container("").height(16.0),
        // Host and Port
        row![
            column![
                text("HOST ADDRESS").size(11).style(ui_style::label_text),
                text_input("192.168.1.1 or example.com", form_host)
                    .on_input(Message::SessionHostChanged)
                    .padding(10)
                    .size(12),
            ]
            .spacing(6)
            .width(Length::FillPortion(3)),
            container("").width(12.0),
            column![
                text("PORT").size(11).style(ui_style::label_text),
                text_input("22", form_port)
                    .on_input(Message::SessionPortChanged)
                    .padding(10)
                    .size(12)
                    .width(Length::Fixed(80.0)),
            ]
            .spacing(6)
            .width(Length::FillPortion(1)),
        ],
        container("").height(16.0),
        // Username
        column![
            text("USERNAME").size(11).style(ui_style::label_text),
            text_input("root", form_username)
                .on_input(Message::SessionUsernameChanged)
                .padding(10)
                .size(12),
        ]
        .spacing(6),
    ]
    .spacing(0);

    // Footer with buttons
    let footer = row![
        container("").width(Length::Fill),
        button(text("Cancel").size(12))
            .padding([10, 20])
            .style(ui_style::secondary_button_style)
            .on_press(Message::CancelSessionEdit),
        button(text("Create Session").size(12))
            .padding([10, 20])
            .style(ui_style::primary_button_style)
            .on_press(Message::SaveSession),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    // Assemble dialog content
    let dialog_content = column![
        header,
        container("").height(16.0),
        tabs,
        container("")
            .height(1.0)
            .width(Length::Fill)
            .style(ui_style::divider),
        container("").height(20.0),
        error_banner,
        form_content,
        container("").height(24.0),
        footer,
    ]
    .spacing(0)
    .padding(24)
    .width(Length::Fixed(560.0));

    container(dialog_content)
        .style(ui_style::dialog_container)
        .into()
}
