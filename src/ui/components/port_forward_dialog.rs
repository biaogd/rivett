use crate::session::config::{PortForwardRule, SessionConfig};
use crate::ui::Message;
use crate::ui::style as ui_style;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    session: &'a SessionConfig,
    local_port: &'a str,
    remote_host: &'a str,
    remote_port: &'a str,
    error: Option<&'a String>,
) -> Element<'a, Message> {
    let header = row![
        column![
            text("Port Forwarding").size(18).style(ui_style::header_text),
            text(format!("Session: {}", session.name))
                .size(12)
                .style(ui_style::muted_text),
        ]
        .spacing(3),
        container("").width(Length::Fill),
        button(text("✕").size(13))
            .padding(8)
            .style(ui_style::tab_close_button)
            .on_press(Message::ClosePortForwarding),
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    let error_banner = if let Some(err) = error {
        container(
            text(format!("⚠️ {}", err))
                .size(12)
                .color(iced::Color::from_rgb(0.9, 0.3, 0.3)),
        )
        .padding(10)
        .width(Length::Fill)
        .style(ui_style::error_banner)
    } else {
        container("").height(0.0)
    };

    let list = if session.port_forwards.is_empty() {
        column![
            text("No port forwards yet.")
                .size(12)
                .style(ui_style::muted_text),
            text("Add one below to start forwarding.")
                .size(12)
                .style(ui_style::muted_text),
        ]
        .spacing(4)
    } else {
        session
            .port_forwards
            .iter()
            .fold(column![], |column, rule| {
                column.push(render_rule_row(rule))
            })
            .spacing(6)
    };

    let form = column![
        text("Add forward").size(12).style(ui_style::muted_text),
        row![
            column![
                text("Local port").size(11).style(ui_style::muted_text),
                text_input("8080", local_port)
                    .on_input(Message::PortForwardLocalPortChanged)
                    .padding([8, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fixed(110.0)),
            ]
            .spacing(4),
            column![
                text("Remote host").size(11).style(ui_style::muted_text),
                text_input("127.0.0.1", remote_host)
                    .on_input(Message::PortForwardRemoteHostChanged)
                    .padding([8, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fixed(200.0)),
            ]
            .spacing(4),
            column![
                text("Remote port").size(11).style(ui_style::muted_text),
                text_input("3306", remote_port)
                    .on_input(Message::PortForwardRemotePortChanged)
                    .padding([8, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fixed(110.0)),
            ]
            .spacing(4),
            container("").width(Length::Fill),
            button(text("Add").size(12))
                .padding([8, 14])
                .style(ui_style::primary_button_style)
                .on_press(Message::AddPortForward),
        ]
        .spacing(12)
        .align_y(Alignment::End),
    ]
    .spacing(6);

    container(
        column![header, error_banner, list, container("").height(8.0), form]
            .spacing(12),
    )
    .width(Length::Fixed(560.0))
    .padding(20)
    .style(ui_style::panel)
    .into()
}

fn render_rule_row<'a>(rule: &'a PortForwardRule) -> Element<'a, Message> {
    let status = if rule.enabled { "Enabled" } else { "Disabled" };
    row![
        text(format!(
            "{} → {}:{}",
            rule.local_port, rule.remote_host, rule.remote_port
        ))
        .size(12),
        container("").width(Length::Fill),
        button(text(status).size(11))
            .padding([4, 8])
            .style(ui_style::menu_button(rule.enabled))
            .on_press(Message::TogglePortForward(rule.id.clone())),
        button(text("Delete").size(11))
            .padding([4, 8])
            .style(ui_style::menu_item_destructive)
            .on_press(Message::DeletePortForward(rule.id.clone())),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
