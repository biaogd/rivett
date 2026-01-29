use crate::session::config::{PortForwardDirection, PortForwardRule, SessionConfig};
use crate::ui::Message;
use crate::ui::state::PortForwardStatus;
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
    direction: PortForwardDirection,
    error: Option<&'a String>,
) -> Element<'a, Message> {
    render_manage_body(
        session,
        local_host,
        local_port,
        remote_host,
        remote_port,
        direction,
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
    direction: PortForwardDirection,
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

    let (local_host_label, local_port_label, remote_host_label, remote_port_label) = match direction
    {
        PortForwardDirection::Local => (
            "Local bind address",
            "Local bind port",
            "Remote target host",
            "Remote target port",
        ),
        PortForwardDirection::Remote => (
            "Local target host",
            "Local target port",
            "Remote bind address",
            "Remote bind port",
        ),
        PortForwardDirection::Dynamic => (
            "Local bind address",
            "Local bind port",
            "Target host (SOCKS)",
            "Target port (SOCKS)",
        ),
    };

    let direction_selector = row![
        button(text("Local").size(12))
            .padding([6, 12])
            .style(ui_style::compact_tab(
                direction == PortForwardDirection::Local
            ))
            .on_press(if direction == PortForwardDirection::Local {
                Message::Ignore
            } else {
                Message::PortForwardDirectionChanged(PortForwardDirection::Local)
            }),
        button(text("Remote").size(12))
            .padding([6, 12])
            .style(ui_style::compact_tab(
                direction == PortForwardDirection::Remote
            ))
            .on_press(if direction == PortForwardDirection::Remote {
                Message::Ignore
            } else {
                Message::PortForwardDirectionChanged(PortForwardDirection::Remote)
            }),
        button(text("Dynamic").size(12))
            .padding([6, 12])
            .style(ui_style::compact_tab(
                direction == PortForwardDirection::Dynamic
            ))
            .on_press(if direction == PortForwardDirection::Dynamic {
                Message::Ignore
            } else {
                Message::PortForwardDirectionChanged(PortForwardDirection::Dynamic)
            }),
    ]
    .spacing(6);

    let direction_hint = match direction {
        PortForwardDirection::Local => "Bind locally, forward to remote target.",
        PortForwardDirection::Remote => "Bind remotely, forward back to local target.",
        PortForwardDirection::Dynamic => "Start a local SOCKS5 proxy.",
    };

    let inputs_row = row![
        column![
            text(local_host_label).size(11).style(ui_style::muted_text),
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
            text(local_port_label).size(11).style(ui_style::muted_text),
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
            text(remote_host_label).size(11).style(ui_style::muted_text),
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
            text(remote_port_label).size(11).style(ui_style::muted_text),
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
    .align_y(Alignment::End);

    let form = column![
        text("Add forward").size(12).style(ui_style::muted_text),
        row![
            direction_selector,
            container("").width(Length::Fill),
            text(direction_hint).size(11).style(ui_style::muted_text),
        ]
        .align_y(Alignment::Center),
        container(inputs_row).style(ui_style::panel).padding(12),
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
    .spacing(8);

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
        text("Type")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Bind address")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Bind port")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Target host")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Target port")
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
        text("Type")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Bind address")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Bind port")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(1)),
        text("Target host")
            .size(13)
            .style(ui_style::muted_text)
            .width(Length::FillPortion(2)),
        text("Target port")
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
    let (direction_label, bind_host, bind_port, target_host, target_port) =
        rule_display_values(rule);
    let status_color = match status {
        Some(PortForwardStatus::Pending) => Color::from_rgb8(10, 132, 255),
        Some(PortForwardStatus::Active) => Color::from_rgb8(52, 199, 89),
        Some(PortForwardStatus::Error(_)) => Color::from_rgb(0.9, 0.3, 0.3),
        None => Color::from_rgb8(180, 180, 186),
    };
    let dot = container(
        iced::widget::Space::new()
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0)),
    )
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
        text(direction_label).size(13).width(Length::FillPortion(1)),
        text(bind_host).size(13).width(Length::FillPortion(2)),
        text(format!("{}", bind_port))
            .size(13)
            .width(Length::FillPortion(1)),
        text(target_host).size(13).width(Length::FillPortion(2)),
        text(format!("{}", target_port))
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
    let (direction_label, bind_host, bind_port, target_host, target_port) =
        rule_display_values(rule);

    row![
        text(direction_label).size(13).width(Length::FillPortion(1)),
        text(bind_host).size(13).width(Length::FillPortion(2)),
        text(format!("{}", bind_port))
            .size(13)
            .width(Length::FillPortion(1)),
        text(target_host).size(13).width(Length::FillPortion(2)),
        text(format!("{}", target_port))
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

fn rule_display_values<'a>(rule: &'a PortForwardRule) -> (&'a str, &'a str, u16, &'a str, u16) {
    let local_host = if rule.local_host.is_empty() {
        "127.0.0.1"
    } else {
        rule.local_host.as_str()
    };
    let remote_host = if rule.remote_host.is_empty() {
        "127.0.0.1"
    } else {
        rule.remote_host.as_str()
    };

    match rule.direction {
        PortForwardDirection::Local => (
            "Local",
            local_host,
            rule.local_port,
            remote_host,
            rule.remote_port,
        ),
        PortForwardDirection::Remote => (
            "Remote",
            remote_host,
            rule.remote_port,
            local_host,
            rule.local_port,
        ),
        PortForwardDirection::Dynamic => ("Dynamic", local_host, rule.local_port, "SOCKS", 0),
    }
}
