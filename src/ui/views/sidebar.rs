use crate::ui::style as ui_style;
use crate::ui::{ActiveView, Message};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

pub fn render<'a>(active_view: ActiveView) -> Element<'a, Message> {
    // Load SVG icons
    let grid_svg = iced::widget::svg(iced::widget::svg::Handle::from_memory(
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/grid.svg")).as_slice(),
    ))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0));

    let dir_svg = iced::widget::svg(iced::widget::svg::Handle::from_memory(
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/dir.svg")).as_slice(),
    ))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0));

    // Sessions button (highlighted when in SessionManager view)
    let sessions_btn = button(
        row![grid_svg, text("Sessions").size(13),]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([8, 12])
    .style(if active_view == ActiveView::SessionManager {
        ui_style::sidebar_button_active
    } else {
        ui_style::sidebar_button_inactive
    })
    .on_press(Message::ShowSessionManager);

    // SFTP button (would be highlighted when in SFTP view)
    let sftp_btn = button(
        row![dir_svg, text("SFTP").size(13),]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([8, 12])
    .style(ui_style::sidebar_button_inactive) // Always inactive for now
    .on_press(Message::Ignore); // Not implemented yet

    // RECENT section header
    let recent_header = container(
        text("RECENT")
            .size(10)
            .style(ui_style::sidebar_section_header),
    )
    .padding(Padding::new(12.0).top(14.0).bottom(6.0));

    // Recent sessions list (placeholder for now)
    let recent_items = column![
        sidebar_recent_item("prod-server"),
        sidebar_recent_item("db-main"),
        sidebar_recent_item("worker-01"),
    ]
    .spacing(2);

    // Assemble sidebar
    column![
        container("").height(10.0),
        sessions_btn,
        container("").height(4.0),
        sftp_btn,
        recent_header,
        recent_items,
    ]
    .spacing(0)
    .width(Length::Fill)
    .into()
}

fn sidebar_recent_item<'a>(name: &'a str) -> Element<'a, Message> {
    button(
        row![
            text(">_").size(11).width(Length::Fixed(18.0)),
            text(name).size(12),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([6, 12])
    .style(ui_style::sidebar_recent_item)
    .on_press(Message::Ignore) // Will connect to session later
    .into()
}
