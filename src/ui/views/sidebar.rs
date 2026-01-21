use crate::ui::Message;
use crate::ui::style as ui_style;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn render<'a>() -> Element<'a, Message> {
    column![
        text("MENU").size(12).style(ui_style::muted_text),
        container("").width(Length::Fill).height(8.0),
        button(
            row![text("📂").size(18), text("Sessions").size(15),]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .padding([10, 14])
        .style(ui_style::menu_item)
        .on_press(Message::ShowSessionManager),
        button(
            row![text("📁").size(18), text("SFTP").size(15),]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .padding([10, 14])
        .style(ui_style::menu_item)
        .on_press(Message::ShowSftp),
        button(
            row![text("🔀").size(18), text("Forwarding").size(15),]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .padding([10, 14])
        .style(ui_style::menu_item)
        .on_press(Message::ShowPortForwarding),
        container("")
            .width(Length::Fill)
            .height(1.0)
            .style(ui_style::menu_divider),
        button(
            row![text("⚙️").size(18), text("Settings").size(15),]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .padding([8, 12])
        .style(ui_style::menu_item)
        .on_press(Message::ShowSettings),
        Space::new().height(Length::Fill),
        button(
            row![text("«").size(14), text("Collapse").size(12),]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .width(Length::Fill)
        .padding([6, 12])
        .style(ui_style::menu_item)
        .on_press(Message::ToggleMenu),
    ]
    .spacing(2)
    .into()
}
