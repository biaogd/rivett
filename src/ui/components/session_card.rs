use crate::session::SessionConfig;
use crate::ui::Message;
use crate::ui::style as ui_style;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

pub fn render<'a>(session: &'a SessionConfig) -> Element<'a, Message> {
    let connection_info = format!("{}@{}:{}", session.username, session.host, session.port);

    let mut card_content = column![
        row![
            text(session.name.clone()).size(16),
            container("").width(Length::Fill),
        ],
        text(connection_info).size(13).style(ui_style::muted_text),
    ]
    .spacing(4);

    // Only show last connected if it exists
    if let Some(dt) = session.last_connected {
        card_content = card_content.push(container("").height(4.0)).push(
            text(format!("Last connected: {}", dt.format("%Y-%m-%d %H:%M")))
                .size(12)
                .style(ui_style::muted_text),
        );
    }

    card_content = card_content.push(container("").height(8.0)).push(
        row![
            button(text("Connect").size(13))
                .padding([6, 16])
                .style(ui_style::new_tab_button)
                .on_press(Message::ConnectToSession(session.id.clone())),
            button(text("Edit").size(13))
                .padding([6, 16])
                .style(ui_style::menu_button(false))
                .on_press(Message::EditSession(session.id.clone())),
            button(text("Delete").size(13))
                .padding([6, 16])
                .style(ui_style::tab_close_button)
                .on_press(Message::DeleteSession(session.id.clone())),
        ]
        .spacing(8),
    );

    let final_card = card_content.padding(12);

    container(final_card)
        .width(Length::Fixed(360.0))
        .style(ui_style::panel)
        .into()
}
