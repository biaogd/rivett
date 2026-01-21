use crate::session::SessionConfig;
use crate::ui::Message;
use crate::ui::components;
use crate::ui::style as ui_style;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    saved_sessions: &'a [SessionConfig],
    editing_session: Option<&'a SessionConfig>,
    form_name: &'a str,
    form_host: &'a str,
    form_port: &'a str,
    form_username: &'a str,
    form_password: &'a str,
    auth_method_password: bool,
    validation_error: Option<&'a String>,
) -> Element<'a, Message> {
    let title_bar = row![
        text("Session Manager").size(20),
        container("").width(Length::Fill),
        button(text("+ New").size(14))
            .padding([8, 16])
            .style(ui_style::new_tab_button)
            .on_press(Message::CreateNewSession),
        button(text("✕").size(16))
            .padding([8, 12])
            .style(ui_style::tab_close_button)
            .on_press(Message::CloseSessionManager),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([12, 16]);

    // Left panel: Session list
    let session_list: Element<Message> = if saved_sessions.is_empty() {
        column![
            container("").height(Length::Fixed(60.0)),
            text("No saved sessions")
                .size(14)
                .style(ui_style::muted_text),
            container("").height(Length::Fixed(8.0)),
            text("Click '+ New' to create")
                .size(12)
                .style(ui_style::muted_text),
        ]
        .align_x(Alignment::Center)
        .into()
    } else {
        iced::widget::responsive(move |size| {
            let card_width = 320.0;
            let spacing = 16.0;
            let padding = 24.0;
            // Calculate columns based on available width
            let cols = ((size.width - padding) / (card_width + spacing))
                .floor()
                .max(1.0) as usize;

            let chunks = saved_sessions.chunks(cols);
            let mut content = column![].spacing(spacing).padding(12);

            for chunk in chunks {
                let mut row = row![].spacing(spacing);
                for session in chunk {
                    row = row.push(components::session_card::render(session));
                }
                content = content.push(row);
            }

            scrollable(content).height(Length::Fill).into()
        })
        .into()
    };

    // Right panel: Form or empty state
    let right_panel = if editing_session.is_some() {
        container(
            container(
                scrollable(components::session_form::render(
                    editing_session,
                    saved_sessions,
                    form_name,
                    form_host,
                    form_port,
                    form_username,
                    form_password,
                    auth_method_password,
                    validation_error,
                ))
                .height(Length::Fill),
            )
            .padding(16)
            .height(Length::Fill)
            .style(ui_style::panel),
        )
        .width(Length::Fixed(400.0)) // Fixed width for form instead of portion
        .height(Length::Fill)
        .padding(12)
    } else {
        container("")
            .width(Length::Fixed(0.0))
            .height(Length::Fixed(0.0))
    };

    let content = column![
        container(title_bar)
            .width(Length::Fill)
            .style(ui_style::tab_bar),
        row![
            container(session_list)
                .width(Length::Fill) // Take remaining space
                .height(Length::Fill),
            right_panel,
        ]
        .height(Length::Fill),
    ]
    .spacing(0);

    // Return content directly (shell adds background)
    content.into()
}
