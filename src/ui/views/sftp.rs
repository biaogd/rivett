use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input, Id,
};
use iced::widget::text::Wrapping;
use iced::{Alignment, Element, Length, Padding};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::ui::style as ui_style;
use crate::ui::Message;
use crate::ui::state::{
    SftpContextAction, SftpContextMenu, SftpEntry, SftpPane, SftpTransfer,
    SftpTransferDirection, SftpTransferStatus,
};

pub fn render<'a>(
    local_path: &'a str,
    remote_path: &'a str,
    local_entries: &'a [SftpEntry],
    local_error: Option<&'a str>,
    remote_entries: &'a [SftpEntry],
    remote_error: Option<&'a str>,
    remote_loading: bool,
    local_selected: Option<&'a str>,
    remote_selected: Option<&'a str>,
    name_column_width: f32,
    context_menu: Option<&'a SftpContextMenu>,
    panel_width: f32,
    panel_height: f32,
    transfers: &'a [SftpTransfer],
    active_tab: usize,
) -> Element<'a, Message> {
    let list_padding_left = 14;
    let list_padding_right = 6;
    let local_scroll_id = Id::new("sftp-local-list");
    let remote_scroll_id = Id::new("sftp-remote-list");

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
            container(
                column![
                    text("Failed to load local files")
                        .size(12)
                        .style(ui_style::muted_text),
                    text(error).size(11).style(ui_style::muted_text),
                ]
                .spacing(6),
            )
            .padding(pad_trbl(0, list_padding_right, 0, list_padding_left)),
        )
        .id(local_scroll_id.clone())
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else if local_entries.is_empty() {
        scrollable(
            container(
                column![text("No files").size(12).style(ui_style::muted_text),].spacing(6),
            )
            .padding(pad_trbl(0, list_padding_right, 0, list_padding_left)),
        )
        .id(local_scroll_id.clone())
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else {
        let mut rows = column![];
        for entry in local_entries {
            let size = entry.size.map(format_size).unwrap_or_else(|| "-".to_string());
            let modified = entry
                .modified
                .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            let selected = local_selected == Some(entry.name.as_str());
            rows = rows.push(file_row(
                entry.name.clone(),
                size,
                modified,
                entry.is_dir,
                selected,
                Message::SftpLocalEntryPressed(entry.name.clone(), entry.is_dir),
                name_column_width,
                SftpPane::Local,
                context_menu,
            ));
        }

        scrollable(rows.spacing(2))
            .id(local_scroll_id.clone())
            .direction(ui_style::thin_scrollbar())
            .style(ui_style::scrollable_style)
            .height(Length::Fill)
    };

    let remote_list = if remote_loading {
        scrollable(
            container(
                column![text("Loading...").size(12).style(ui_style::muted_text),].spacing(6),
            )
            .padding(pad_trbl(0, list_padding_right, 0, list_padding_left)),
        )
        .id(remote_scroll_id.clone())
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else if let Some(error) = remote_error {
        scrollable(
            container(
                column![
                    text("Failed to load remote files")
                        .size(12)
                        .style(ui_style::muted_text),
                    text(error).size(11).style(ui_style::muted_text),
                ]
                .spacing(6),
            )
            .padding(pad_trbl(0, list_padding_right, 0, list_padding_left)),
        )
        .id(remote_scroll_id.clone())
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else if remote_entries.is_empty() {
        scrollable(
            container(
                column![text("No files").size(12).style(ui_style::muted_text),].spacing(6),
            )
            .padding(pad_trbl(0, list_padding_right, 0, list_padding_left)),
        )
        .id(remote_scroll_id.clone())
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else {
        let mut rows = column![];
        for entry in remote_entries {
            let size = entry.size.map(format_size).unwrap_or_else(|| "-".to_string());
            let modified = entry
                .modified
                .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            let selected = remote_selected == Some(entry.name.as_str());
            rows = rows.push(file_row(
                entry.name.clone(),
                size,
                modified,
                entry.is_dir,
                selected,
                Message::SftpRemoteEntryPressed(entry.name.clone(), entry.is_dir),
                name_column_width,
                SftpPane::Remote,
                context_menu,
            ));
        }
        scrollable(rows.spacing(2))
            .id(remote_scroll_id.clone())
            .direction(ui_style::thin_scrollbar())
            .style(ui_style::scrollable_style)
            .height(Length::Fill)
    };

    let make_list_header = || {
        row![
            text("Name")
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(name_column_width))
                .wrapping(Wrapping::None),
            text("Size")
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(64.0))
                .wrapping(Wrapping::None),
            text("Modified")
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(120.0))
                .wrapping(Wrapping::None),
        ]
        .align_y(Alignment::Center)
    };

    let local_list_panel = column![
        container(make_list_header())
            .padding(pad_trbl(1, list_padding_right, 1, list_padding_left)),
        container("")
            .height(1.0)
            .width(Length::Fill)
            .style(ui_style::divider),
        container(local_list)
            .padding([2, 0])
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
            .padding([6, 0])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::panel),
    ]
    .spacing(6)
    .width(Length::FillPortion(1))
    .height(Length::Fill);

    let remote_list_panel = column![
        container(make_list_header())
            .padding(pad_trbl(1, list_padding_right, 1, list_padding_left)),
        container("")
            .height(1.0)
            .width(Length::Fill)
            .style(ui_style::divider),
        container(remote_list)
            .padding([2, 0])
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
            .padding([6, 0])
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

    let mut queue_rows = column![];
    for transfer in transfers
        .iter()
        .filter(|transfer| transfer.tab_index == active_tab)
        .rev()
        .take(6)
    {
        let (status, progress) = transfer_status(transfer);
        queue_rows = queue_rows.push(transfer_row(
            transfer.name.clone(),
            status,
            progress,
        ));
    }
    if transfers
        .iter()
        .all(|transfer| transfer.tab_index != active_tab)
    {
        queue_rows = queue_rows.push(
            text("No transfers").size(12).style(ui_style::muted_text),
        );
    }
    let queue_rows = queue_rows.spacing(8);

    let queue = column![
        text("Transfers").size(12).style(ui_style::muted_text),
        container(
            scrollable(queue_rows)
                .direction(ui_style::thin_scrollbar())
                .style(ui_style::scrollable_style)
                .height(Length::Fill),
        )
        .padding([8, 0])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ui_style::panel),
    ]
    .spacing(8)
    .height(Length::Fixed(180.0));

    let base = column![
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
    .height(Length::Fill);

    let base: Element<'_, Message> = iced::widget::mouse_area(base)
        .on_press(Message::SftpCloseContextMenu)
        .into();

    let overlay: Element<'_, Message> = if let Some(menu) = context_menu {
        let menu_width = 160.0;
        let menu_height = 120.0;
        let padding = 8.0;
        let max_x = (panel_width - menu_width - padding).max(padding);
        let max_y = (panel_height - menu_height - padding).max(padding);
        let x = menu.position.x.clamp(padding, max_x);
        let y = menu.position.y.clamp(padding, max_y);

        let actions = match menu.pane {
            SftpPane::Local => vec![
                ("Upload", SftpContextAction::Upload, false),
                ("Rename", SftpContextAction::Rename, false),
                ("Delete", SftpContextAction::Delete, true),
            ],
            SftpPane::Remote => vec![
                ("Download", SftpContextAction::Download, false),
                ("Rename", SftpContextAction::Rename, false),
                ("Delete", SftpContextAction::Delete, true),
            ],
        };

        let mut menu_column = column![];
        for (label, action, destructive) in actions {
            let button_style = if destructive {
                ui_style::menu_item_destructive
            } else {
                ui_style::menu_item_button
            };
            menu_column = menu_column.push(
                button(text(label).size(12))
                    .padding([6, 10])
                    .style(button_style)
                    .width(Length::Fill)
                    .on_press(Message::SftpContextAction(
                        menu.pane,
                        menu.name.clone(),
                        action,
                    )),
            );
        }

        let menu_panel = iced::widget::mouse_area(
            container(menu_column.spacing(4))
                .padding(8)
                .width(Length::Fixed(menu_width))
                .style(ui_style::popover_menu),
        )
        .on_press(Message::Ignore);

        container(
            column![
                iced::widget::Space::new()
                    .width(Length::Fill)
                    .height(Length::Fixed(y)),
                row![
                    iced::widget::Space::new()
                        .width(Length::Fixed(x))
                        .height(Length::Fill),
                    menu_panel
                ],
            ],
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    iced::widget::stack![base, overlay].into()
}

fn transfer_row(name: String, status: String, progress: f32) -> Element<'static, Message> {
    let progress_bar = container(progress_bar(0.0..=1.0, progress))
        .height(Length::Fixed(6.0))
        .width(Length::Fill);

    container(
        column![
            row![
                text(name).size(12),
                container("").width(Length::Fill),
                text(status).size(11).style(ui_style::muted_text),
            ]
            .align_y(Alignment::Center),
            progress_bar,
        ]
        .spacing(4),
    )
    .padding(pad_trbl(0, 8, 0, 8))
    .into()
}

fn transfer_status(transfer: &SftpTransfer) -> (String, f32) {
    let total = transfer.bytes_total.max(1);
    let progress = (transfer.bytes_sent as f32 / total as f32).clamp(0.0, 1.0);
    let direction = match transfer.direction {
        SftpTransferDirection::Upload => "Upload",
        SftpTransferDirection::Download => "Download",
    };
    let status = match &transfer.status {
        SftpTransferStatus::Queued => format!("{} queued", direction),
        SftpTransferStatus::Uploading => {
            if transfer.bytes_total > 0 {
                format!(
                    "{} {} / {}",
                    direction,
                    format_size(transfer.bytes_sent),
                    format_size(transfer.bytes_total),
                )
            } else {
                format!("{} in progress", direction)
            }
        }
        SftpTransferStatus::Completed => format!("{} completed", direction),
        SftpTransferStatus::Failed(_) => format!("{} failed", direction),
    };
    (status, progress)
}

fn pad_trbl(top: u16, right: u16, bottom: u16, left: u16) -> Padding {
    Padding {
        top: top.into(),
        right: right.into(),
        bottom: bottom.into(),
        left: left.into(),
    }
}

fn file_row(
    name: String,
    size: String,
    modified: String,
    is_dir: bool,
    selected: bool,
    on_press: Message,
    name_column_width: f32,
    pane: SftpPane,
    context_menu: Option<&SftpContextMenu>,
) -> Element<'static, Message> {
    let name_style = if is_dir {
        ui_style::header_text
    } else {
        ui_style::muted_text
    };

    let display_name = truncate_name(&name, name_column_width, 13.0);
    let row_button = button(
        row![
            text(display_name)
                .size(13)
                .style(name_style)
                .width(Length::Fixed(name_column_width))
                .wrapping(Wrapping::None),
            text(size)
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(64.0))
                .wrapping(Wrapping::None),
            text(modified)
                .size(12)
                .style(ui_style::muted_text)
                .width(Length::Fixed(120.0))
                .wrapping(Wrapping::None),
        ]
        .align_y(Alignment::Center),
    )
    .padding(pad_trbl(2, 6, 2, 14))
    .width(Length::Fill)
    .style(ui_style::sftp_row_button(selected))
    .on_press(on_press);

    let row_area = iced::widget::mouse_area(row_button)
        .on_right_press(Message::SftpOpenContextMenu(pane, name.clone()))
        .on_press(Message::Ignore);

    let menu_open = context_menu
        .map(|menu| menu.pane == pane && menu.name == name)
        .unwrap_or(false);

    if !menu_open {
        return row_area.into();
    }

    row_area.into()
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

fn truncate_name(name: &str, max_width: f32, font_size: f32) -> String {
    let char_width = (font_size * 0.6).max(6.0);
    let max_chars = ((max_width - 8.0) / char_width).floor() as usize;
    let max_cols = max_chars.max(4);

    if name.width() <= max_cols {
        return name.to_string();
    }

    let mut truncated = String::new();
    let ellipsis_cols = 3;
    let target_cols = max_cols.saturating_sub(ellipsis_cols).max(1);
    let mut used_cols = 0;
    for ch in name.chars() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if used_cols + width > target_cols {
            break;
        }
        used_cols += width;
        truncated.push(ch);
    }
    truncated.push_str("...");
    truncated
}
