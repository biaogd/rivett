use crate::session::SessionConfig;
use crate::ui::Message;
use crate::ui::style as ui_style;
use iced::widget::{button, column, container, row, stack, text};
use iced::{Element, Length, Renderer, Theme};

pub fn render<'a>(session: &'a SessionConfig, menu_open: bool) -> Element<'a, Message> {
    let connection_info = format!("{}@{}:{}", session.username, session.host, session.port);

    let mut card_content: iced::widget::Column<'a, Message, Theme, Renderer> = column![
        row![
            text(session.name.clone()).size(14).style(ui_style::header_text),
            container("").width(Length::Fill),
            button(text("⋮").size(16))
                .padding([2, 6])
                .style(ui_style::icon_button)
                .on_press(Message::ToggleSessionMenu(session.id.clone())),
        ],
        text(connection_info).size(12).style(ui_style::muted_text),
    ]
    .spacing(6);

    // Only show last connected if it exists
    if let Some(dt) = session.last_connected {
        card_content = card_content.push(container("").height(4.0)).push(
            text(format!("Last connected: {}", dt.format("%Y-%m-%d %H:%M")))
                .size(12)
                .style(ui_style::muted_text),
        );
    }

    card_content = card_content.push(container("").height(10.0)).push(
        row![
            button(text("Connect").size(12))
                .padding([6, 16])
                .style(ui_style::primary_button_style)
                .on_press(Message::ConnectToSession(session.id.clone())),
            container("").width(Length::Fill),
        ]
        .spacing(8),
    );

    let base_card = container(card_content.padding(16)).width(Length::Fill);

    let content: Element<'a, Message> = if menu_open {
        let menu = iced::widget::mouse_area(
            container(
                column![
                    button(text("Edit").size(12))
                        .padding([6, 10])
                        .style(ui_style::menu_item_button)
                        .width(Length::Fill)
                        .on_press(Message::EditSession(session.id.clone())),
                    button(text("Delete").size(12))
                        .padding([6, 10])
                        .style(ui_style::menu_item_destructive)
                        .width(Length::Fill)
                        .on_press(Message::DeleteSession(session.id.clone())),
                ]
                .spacing(4),
            )
            .padding(8)
            .width(Length::Fixed(120.0))
            .style(ui_style::popover_menu),
        )
        .on_press(Message::Ignore);

        let overlay = container(
            column![row![container("").width(Length::Fill), menu]].spacing(0),
        )
        .width(Length::Fill)
        .padding(8);

        stack![base_card, overlay].into()
    } else {
        base_card.into()
    };

    container(content)
        .width(Length::Fixed(320.0))
        .style(ui_style::panel)
        .into()
}
