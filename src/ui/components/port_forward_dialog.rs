use crate::session::config::{PortForwardRule, SessionConfig};
use crate::ui::state::PortForwardStatus;
use crate::ui::Message;
use crate::ui::style as ui_style;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length};
use std::collections::HashMap;

pub fn render_manage_inline<'a>(
    session: &'a SessionConfig,
    local_host: &'a str,
    local_port: &'a str,
    remote_host: &'a str,
    remote_port: &'a str,
    error: Option<&'a String>,
) -> Element<'a, Message> {
    render_manage_body(
        session,
        local_host,
        local_port,
        remote_host,
        remote_port,
        error,
    )
}

pub fn render_list<'a>(
    session: &'a SessionConfig,
    statuses: Option<&'a HashMap<String, PortForwardStatus>>,
) -> Element<'a, Message> {
    list_view(session, statuses)
}

fn render_manage_body<'a>(
    session: &'a SessionConfig,
    local_host: &'a str,
    local_port: &'a str,
    remote_host: &'a str,
    remote_port: &'a str,
    error: Option<&'a String>,
) -> Element<'a, Message> {
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

    let list = manage_list_view(session);

    let form = column![
        text("Add forward").size(12).style(ui_style::muted_text),
        row![
            column![
                text("Local address").size(11).style(ui_style::muted_text),
                text_input("127.0.0.1", local_host)
                    .on_input(Message::PortForwardLocalHostChanged)
                    .padding([7, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fill),
            ]
            .spacing(4)
            .width(Length::FillPortion(2)),
            container("").width(10.0),
            column![
                text("Local port").size(11).style(ui_style::muted_text),
                text_input("8080", local_port)
                    .on_input(Message::PortForwardLocalPortChanged)
                    .padding([7, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fill),
            ]
            .spacing(4)
            .width(Length::FillPortion(1)),
            container("").width(10.0),
            column![
                text("Remote host").size(11).style(ui_style::muted_text),
                text_input("127.0.0.1", remote_host)
                    .on_input(Message::PortForwardRemoteHostChanged)
                    .padding([7, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fill),
            ]
            .spacing(4)
            .width(Length::FillPortion(2)),
            container("").width(10.0),
            column![
                text("Remote port").size(11).style(ui_style::muted_text),
                text_input("3306", remote_port)
                    .on_input(Message::PortForwardRemotePortChanged)
                    .padding([7, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fill),
            ]
            .spacing(4)
            .width(Length::FillPortion(1)),
        ]
        .spacing(10)
        .align_y(Alignment::End),
        row![
            container("").width(Length::Fill),
            button(text("Add").size(12))
                .padding([7, 14])
                .style(ui_style::primary_button_style)
                .on_press(Message::AddPortForward),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(6);

    column![
        error_banner,
        container(list).style(ui_style::panel).padding(12),
        form
    ]
    .spacing(14)
    .into()
}

fn list_view<'a>(
    session: &'a SessionConfig,
    statuses: Option<&'a HashMap<String, PortForwardStatus>>,
) -> Element<'a, Message> {
    if session.port_forwards.is_empty() {
        return column![
            text("No port forwards yet.")
                .size(12)
                .style(ui_style::muted_text),
            text("Add one below to start forwarding.")
                .size(12)
                .style(ui_style::muted_text),
        ]
        .spacing(4)
        .into();
    }

    let header_row = row![
        text("Local address")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Local port")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Remote host")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Remote port")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Status")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let header = container(header_row)
        .padding([6, 10])
        .style(ui_style::table_header);

    let mut rows = column![header].spacing(0);
    for (index, rule) in session.port_forwards.iter().enumerate() {
        let row = container(render_rule_row(
            rule,
            statuses.and_then(|map| map.get(&rule.id)),
        ))
        .padding([8, 12]);
        rows = rows.push(row);
        if index + 1 < session.port_forwards.len() {
            rows = rows.push(
                container("")
                    .height(1.0)
                    .width(Length::Fill)
                    .style(ui_style::divider),
            );
        }
    }

    rows.into()
}

fn manage_list_view<'a>(session: &'a SessionConfig) -> Element<'a, Message> {
    if session.port_forwards.is_empty() {
        return column![
            text("No port forwards yet.")
                .size(12)
                .style(ui_style::muted_text),
            text("Add one below to start forwarding.")
                .size(12)
                .style(ui_style::muted_text),
        ]
        .spacing(4)
        .into();
    }

    let header = row![
        text("Local address")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Local port")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Remote host")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Remote port")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Actions")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::Fixed(70.0)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    session
        .port_forwards
        .iter()
        .fold(column![header], |column, rule| {
            column.push(render_manage_row(rule))
        })
        .spacing(8)
        .into()
}

fn render_rule_row<'a>(
    rule: &'a PortForwardRule,
    status: Option<&'a PortForwardStatus>,
) -> Element<'a, Message> {
    let local_host = if rule.local_host.is_empty() {
        "127.0.0.1"
    } else {
        rule.local_host.as_str()
    };
    let status_color = match status {
        Some(PortForwardStatus::Pending) => Color::from_rgb8(10, 132, 255),
        Some(PortForwardStatus::Active) => Color::from_rgb8(52, 199, 89),
        Some(PortForwardStatus::Error(_)) => Color::from_rgb(0.9, 0.3, 0.3),
        None => Color::from_rgb8(180, 180, 186),
    };
    let dot = container(iced::widget::Space::new()
        .width(Length::Fixed(10.0))
        .height(Length::Fixed(10.0)))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(status_color)),
            border: Border {
                color: status_color,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..iced::widget::container::Style::default()
        });

    row![
        text(local_host).size(13).width(Length::FillPortion(2)),
        text(format!("{}", rule.local_port))
            .size(13)
            .width(Length::FillPortion(1)),
        text(&rule.remote_host)
            .size(13)
            .width(Length::FillPortion(2)),
        text(format!("{}", rule.remote_port))
            .size(13)
            .width(Length::FillPortion(1)),
        container(dot)
            .width(Length::FillPortion(1))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn render_manage_row<'a>(rule: &'a PortForwardRule) -> Element<'a, Message> {
    let local_host = if rule.local_host.is_empty() {
        "127.0.0.1"
    } else {
        rule.local_host.as_str()
    };

    row![
        text(local_host).size(13).width(Length::FillPortion(2)),
        text(format!("{}", rule.local_port))
            .size(13)
            .width(Length::FillPortion(1)),
        text(&rule.remote_host)
            .size(13)
            .width(Length::FillPortion(2)),
        text(format!("{}", rule.remote_port))
            .size(13)
            .width(Length::FillPortion(1)),
        button(text("Delete").size(12))
            .padding([4, 10])
            .style(ui_style::menu_item_destructive)
            .on_press(Message::DeletePortForward(rule.id.clone()))
            .width(Length::Fixed(70.0)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}
