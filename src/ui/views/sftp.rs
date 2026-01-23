use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length};

use crate::ui::style as ui_style;
use crate::ui::Message;
use crate::ui::state::SftpEntry;

pub fn render<'a>(
    local_path: &'a str,
    remote_path: &'a str,
    local_entries: &'a [SftpEntry],
    local_error: Option<&'a str>,
    remote_entries: &'a [SftpEntry],
    remote_error: Option<&'a str>,
    remote_loading: bool,
) -> Element<'a, Message> {
    let nav_buttons = || {
        row![
            button(text("<").size(12))
                .padding([4, 8])
                .style(ui_style::icon_button)
                .on_press(Message::Ignore),
            button(text(">").size(12))
                .padding([4, 8])
                .style(ui_style::icon_button)
                .on_press(Message::Ignore),
            button(text("^").size(12))
                .padding([4, 8])
                .style(ui_style::icon_button)
                .on_press(Message::Ignore),
            button(text("R").size(12))
                .padding([4, 8])
                .style(ui_style::icon_button)
                .on_press(Message::Ignore),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
    };

    let local_path_input = text_input("Local path", local_path)
        .on_input(Message::SftpLocalPathChanged)
        .padding([6, 10])
        .size(13)
        .style(ui_style::dialog_input)
        .width(Length::Fill);

    let remote_path_input = text_input("Remote path", remote_path)
        .on_input(Message::SftpRemotePathChanged)
        .padding([6, 10])
        .size(13)
        .style(ui_style::dialog_input)
        .width(Length::Fill);

    let local_list = if let Some(error) = local_error {
        scrollable(
            column![
                text("Failed to load local files")
                    .size(12)
                    .style(ui_style::muted_text),
                text(error).size(11).style(ui_style::muted_text),
            ]
            .spacing(6),
        )
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else if local_entries.is_empty() {
        scrollable(
            column![text("No files").size(12).style(ui_style::muted_text),].spacing(6),
        )
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else {
        let mut rows = column![];
        for entry in local_entries {
            let size = entry.size.map(format_size).unwrap_or_else(|| "-".to_string());
            let modified = entry
                .modified
                .map(|time| time.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "-".to_string());
            rows = rows.push(file_row(
                entry.name.clone(),
                size,
                modified,
                entry.is_dir,
            ));
        }

        scrollable(rows.spacing(2))
            .direction(ui_style::thin_scrollbar())
            .style(ui_style::scrollable_style)
            .height(Length::Fill)
    };

    let remote_list = if remote_loading {
        scrollable(
            column![text("Loading...").size(12).style(ui_style::muted_text),].spacing(6),
        )
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else if let Some(error) = remote_error {
        scrollable(
            column![
                text("Failed to load remote files")
                    .size(12)
                    .style(ui_style::muted_text),
                text(error).size(11).style(ui_style::muted_text),
            ]
            .spacing(6),
        )
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else if remote_entries.is_empty() {
        scrollable(
            column![text("No files").size(12).style(ui_style::muted_text),].spacing(6),
        )
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else {
        let mut rows = column![];
        for entry in remote_entries {
            let size = entry.size.map(format_size).unwrap_or_else(|| "-".to_string());
            let modified = entry
                .modified
                .map(|time| time.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "-".to_string());
            rows = rows.push(file_row(
                entry.name.clone(),
                size,
                modified,
                entry.is_dir,
            ));
        }
        scrollable(rows.spacing(2))
            .direction(ui_style::thin_scrollbar())
            .style(ui_style::scrollable_style)
            .height(Length::Fill)
    };

    let make_list_header = || {
        row![
            text("Name").size(12).style(ui_style::muted_text),
            container("").width(Length::Fill),
            text("Size")
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(64.0)),
            text("Modified")
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(80.0)),
        ]
        .align_y(Alignment::Center)
    };

    let local_list_panel = column![
        container(make_list_header()).padding([1, 6]),
        container("")
            .height(1.0)
            .width(Length::Fill)
            .style(ui_style::divider),
        container(local_list)
            .padding([2, 8])
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill);

    let local_panel = column![
        row![
            text("Local").size(14).style(ui_style::header_text),
            container("").width(Length::Fill),
            nav_buttons(),
        ]
        .align_y(Alignment::Center),
        local_path_input,
        container(local_list_panel)
            .padding([6, 6])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::panel),
    ]
    .spacing(6)
    .width(Length::FillPortion(1))
    .height(Length::Fill);

    let remote_list_panel = column![
        container(make_list_header()).padding([1, 6]),
        container("")
            .height(1.0)
            .width(Length::Fill)
            .style(ui_style::divider),
        container(remote_list)
            .padding([2, 8])
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill);

    let remote_panel = column![
        row![
            text("Remote").size(14).style(ui_style::header_text),
            container("").width(Length::Fill),
            nav_buttons(),
        ]
        .align_y(Alignment::Center),
        remote_path_input,
        container(remote_list_panel)
            .padding([6, 6])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::panel),
    ]
    .spacing(6)
    .width(Length::FillPortion(1))
    .height(Length::Fill);

    let panels = row![local_panel, remote_panel]
        .spacing(12)
        .height(Length::Fill);

    let queue_rows = column![
        transfer_row("upload.log", "Uploading", 0.62),
        transfer_row("backup.tar.gz", "Queued", 0.0),
        transfer_row("assets.zip", "Completed", 1.0),
    ]
    .spacing(8);

    let queue = column![
        text("Transfers").size(12).style(ui_style::muted_text),
        container(queue_rows)
            .padding(10)
            .width(Length::Fill)
            .style(ui_style::panel),
    ]
    .spacing(8)
    .height(Length::Fixed(160.0));

    column![
        row![
            text("SFTP").size(15).style(ui_style::header_text),
            container("").width(Length::Fill),
            text(if remote_loading { "Loading" } else { "Disconnected" })
                .size(12)
                .style(ui_style::muted_text),
        ]
        .align_y(Alignment::Center),
        panels,
        queue,
    ]
    .spacing(12)
    .height(Length::Fill)
    .into()
}

fn transfer_row<'a>(name: &'a str, status: &'a str, progress: f32) -> Element<'a, Message> {
    let progress_bar = container(progress_bar(0.0..=1.0, progress))
        .height(Length::Fixed(6.0))
        .width(Length::Fill);

    column![
        row![
            text(name).size(12),
            container("").width(Length::Fill),
            text(status).size(11).style(ui_style::muted_text),
        ]
        .align_y(Alignment::Center),
        progress_bar,
    ]
    .spacing(4)
    .into()
}

fn file_row(
    name: String,
    size: String,
    modified: String,
    is_dir: bool,
) -> Element<'static, Message> {
    let name_style = if is_dir {
        ui_style::header_text
    } else {
        ui_style::muted_text
    };

    button(
        row![
            text(name).size(13).style(name_style),
            container("").width(Length::Fill),
            text(size)
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(64.0)),
            text(modified)
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(80.0)),
        ]
        .align_y(Alignment::Center),
    )
    .padding([2, 6])
    .width(Length::Fill)
    .style(ui_style::menu_item_button)
    .on_press(Message::Ignore)
    .into()
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
