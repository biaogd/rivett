use iced::widget::text::Wrapping;
use iced::widget::{
    Id, button, column, container, progress_bar, row, scrollable, svg, text, text_input, tooltip,
};
use iced::{Alignment, Element, Length, Padding};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::ui::Message;
use crate::ui::state::{
    SftpContextAction, SftpContextMenu, SftpEntry, SftpPane, SftpTransfer, SftpTransferDirection,
    SftpTransferStatus,
};
use crate::ui::style as ui_style;

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
    rename_input_id: &'a Id,
    rename_target: Option<&'a crate::ui::state::SftpPendingAction>,
    rename_value: &'a str,
    hovered_file: Option<&'a (SftpPane, String)>,
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
            container(column![text("No files").size(12).style(ui_style::muted_text),].spacing(6))
                .padding(pad_trbl(0, list_padding_right, 0, list_padding_left)),
        )
        .id(local_scroll_id.clone())
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else {
        let mut rows = column![];
        for entry in local_entries {
            let size = entry
                .size
                .map(format_size)
                .unwrap_or_else(|| "-".to_string());
            let modified = entry
                .modified
                .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            let selected = local_selected == Some(entry.name.as_str());
            let hovered = hovered_file
                .map(|(p, n)| *p == SftpPane::Local && n == &entry.name)
                .unwrap_or(false);
            rows = rows.push(file_row(
                entry.name.clone(),
                size,
                modified,
                entry.is_dir,
                selected,
                hovered,
                Message::SftpFileDragStart(SftpPane::Local, entry.name.clone()),
                name_column_width,
                SftpPane::Local,
                context_menu,
                rename_input_id,
                rename_target,
                rename_value,
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
            container(column![text("Loading...").size(12).style(ui_style::muted_text),].spacing(6))
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
            container(column![text("No files").size(12).style(ui_style::muted_text),].spacing(6))
                .padding(pad_trbl(0, list_padding_right, 0, list_padding_left)),
        )
        .id(remote_scroll_id.clone())
        .direction(ui_style::thin_scrollbar())
        .style(ui_style::scrollable_style)
        .height(Length::Fill)
    } else {
        let mut rows = column![];
        for entry in remote_entries {
            let size = entry
                .size
                .map(format_size)
                .unwrap_or_else(|| "-".to_string());
            let modified = entry
                .modified
                .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            let selected = remote_selected == Some(entry.name.as_str());
            let hovered = hovered_file
                .map(|(p, n)| *p == SftpPane::Remote && n == &entry.name)
                .unwrap_or(false);
            rows = rows.push(file_row(
                entry.name.clone(),
                size,
                modified,
                entry.is_dir,
                selected,
                hovered,
                Message::SftpFileDragStart(SftpPane::Remote, entry.name.clone()),
                name_column_width,
                SftpPane::Remote,
                context_menu,
                rename_input_id,
                rename_target,
                rename_value,
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
        container(make_list_header()).padding(pad_trbl(
            1,
            list_padding_right,
            1,
            list_padding_left
        )),
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
        container(make_list_header()).padding(pad_trbl(
            1,
            list_padding_right,
            1,
            list_padding_left
        )),
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

    let queue_content_width = (panel_width - 24.0).max(200.0);
    let transfer_name_width = (queue_content_width * (3.6 / 11.0)).max(140.0);

    let mut queue_rows = column![];
    for transfer in transfers.iter().rev().take(6) {
        let (status, progress) = transfer_status(transfer);
        queue_rows = queue_rows.push(transfer_row(
            transfer,
            status,
            progress,
            transfer_name_width,
        ));
    }
    if transfers.is_empty() {
        queue_rows = queue_rows.push(text("No transfers").size(12).style(ui_style::muted_text));
    }
    let queue_rows = queue_rows.spacing(8);

    let queue = column![
        row![
            text("Transfers").size(12).style(ui_style::muted_text),
            container("").width(Length::Fill),
            button(text("Clear").size(12))
                .padding([2, 6])
                .style(ui_style::icon_button)
                .on_press(Message::SftpTransferClearDone),
        ]
        .align_y(Alignment::Center),
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
            text(if remote_loading {
                "Loading"
            } else {
                "Disconnected"
            })
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
                button(text(label).size(13))
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

        container(column![
            iced::widget::Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(y)),
            row![
                iced::widget::Space::new()
                    .width(Length::Fixed(x))
                    .height(Length::Fill),
                menu_panel
            ],
        ])
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

pub fn delete_dialog<'a>(name: &'a str, is_dir: bool) -> Element<'a, Message> {
    let title = text("Delete").size(16).style(ui_style::header_text);
    let message = if is_dir {
        format!("Delete folder \"{}\"?", name)
    } else {
        format!("Delete file \"{}\"?", name)
    };

    let hint = text(message).size(13).style(ui_style::muted_text);

    let actions = row![
        container("").width(Length::Fill),
        button(text("Cancel").size(12))
            .padding([6, 12])
            .style(ui_style::secondary_button_style)
            .on_press(Message::SftpDeleteCancel),
        button(text("Delete").size(12))
            .padding([6, 12])
            .style(ui_style::destructive_button_style)
            .on_press(Message::SftpDeleteConfirm),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(
        column![title, hint, actions]
            .spacing(12)
            .width(Length::Fixed(360.0)),
    )
    .padding(16)
    .style(ui_style::dialog_container)
    .into()
}

fn transfer_row(
    transfer: &SftpTransfer,
    status: String,
    progress: f32,
    name_width: f32,
) -> Element<'static, Message> {
    let progress_bar = container(progress_bar(0.0..=1.0, progress))
        .height(Length::Fixed(6.0))
        .width(Length::Fill);

    let display_name = truncate_name(&transfer.name, name_width, 13.0);
    let action_cell: Element<'static, Message> = match &transfer.status {
        SftpTransferStatus::Uploading => row![
            action_button(
                "Pause",
                icon_svg(PAUSE_SVG),
                Message::SftpTransferPause(transfer.id),
            ),
            action_button(
                "Cancel",
                icon_svg(CANCEL_SVG),
                Message::SftpTransferCancel(transfer.id),
            ),
        ]
        .spacing(4)
        .into(),
        SftpTransferStatus::Paused => row![
            action_button(
                "Resume",
                icon_svg(RESUME_SVG),
                Message::SftpTransferResume(transfer.id),
            ),
            action_button(
                "Cancel",
                icon_svg(CANCEL_SVG),
                Message::SftpTransferCancel(transfer.id),
            ),
        ]
        .spacing(4)
        .into(),
        SftpTransferStatus::Queued => action_button(
            "Cancel",
            icon_svg(CANCEL_SVG),
            Message::SftpTransferCancel(transfer.id),
        ),
        SftpTransferStatus::Failed(_) | SftpTransferStatus::Canceled => action_button(
            "Retry",
            icon_svg(RETRY_SVG),
            Message::SftpTransferRetry(transfer.id),
        ),
        _ => container("").into(),
    };

    let status_icon = match &transfer.status {
        SftpTransferStatus::Queued => icon_svg(QUEUED_SVG),
        SftpTransferStatus::Uploading => match transfer.direction {
            SftpTransferDirection::Upload => icon_svg(UPLOADING_SVG),
            SftpTransferDirection::Download => icon_svg(DOWNLOADING_SVG),
        },
        SftpTransferStatus::Paused => icon_svg(PAUSED_SVG),
        SftpTransferStatus::Completed => icon_svg(CHECK_SVG),
        SftpTransferStatus::Failed(_) => icon_svg(ERROR_SVG),
        SftpTransferStatus::Canceled => icon_svg(CANCEL_STATUS_SVG),
    };

    container(
        row![
            text(display_name)
                .size(13)
                .wrapping(Wrapping::None)
                .width(Length::FillPortion(3)),
            progress_bar.width(Length::FillPortion(5)),
            row![
                status_icon,
                text(status)
                    .size(13)
                    .style(ui_style::muted_text)
                    .wrapping(Wrapping::None),
            ]
            .align_y(Alignment::Center)
            .spacing(4)
            .width(Length::FillPortion(2)),
            container(action_cell)
                .width(Length::FillPortion(1))
                .center_x(Length::Fill),
        ]
        .align_y(Alignment::Center)
        .spacing(6),
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
    let rate = transfer_rate(transfer);
    let percent = (progress * 100.0).round() as u32;
    let status = match &transfer.status {
        SftpTransferStatus::Queued => format!("{} queued", direction),
        SftpTransferStatus::Uploading => {
            if transfer.bytes_total > 0 {
                format!("{}% · {}", percent, rate)
            } else {
                format!("{} · {}", direction, rate)
            }
        }
        SftpTransferStatus::Paused => format!("Paused · {}", rate),
        SftpTransferStatus::Completed => format!("{} completed", direction),
        SftpTransferStatus::Failed(_) => format!("{} failed", direction),
        SftpTransferStatus::Canceled => format!("{} canceled", direction),
    };
    (status, progress)
}

fn action_button(
    label: &'static str,
    icon: Element<'static, Message>,
    message: Message,
) -> Element<'static, Message> {
    let tip = container(text(label).size(11).style(ui_style::tooltip_text)).padding([4, 8]);

    tooltip(
        button(icon.map(|_| Message::Ignore))
            .padding([2, 6])
            .style(ui_style::icon_button)
            .on_press(message),
        tip,
        tooltip::Position::Top,
    )
    .style(ui_style::tooltip_style)
    .gap(6)
    .into()
}

fn transfer_rate(transfer: &SftpTransfer) -> String {
    if let Some(rate) = transfer.last_rate_bps {
        return format!("{}/s", format_size(rate));
    }
    "--".to_string()
}

fn pad_trbl(top: u16, right: u16, bottom: u16, left: u16) -> Padding {
    Padding {
        top: top.into(),
        right: right.into(),
        bottom: bottom.into(),
        left: left.into(),
    }
}

fn file_icon(
    name: &str,
    is_dir: bool,
) -> (
    fn(&iced::Theme) -> iced::widget::text::Style,
    Element<'static, Message>,
) {
    if is_dir {
        return (ui_style::header_text, icon_svg(FOLDER_SVG));
    }

    let lower = name.to_lowercase();
    if is_image_file(&lower) {
        return (ui_style::muted_text, icon_svg(IMAGE_SVG));
    }
    if is_archive_file(&lower) {
        return (ui_style::muted_text, icon_svg(ARCHIVE_SVG));
    }
    if is_executable_file(&lower) {
        return (ui_style::muted_text, icon_svg(EXEC_SVG));
    }

    (ui_style::header_text, icon_svg(FILE_SVG))
}

fn icon_svg(data: &str) -> Element<'static, Message> {
    let handle = svg::Handle::from_memory(data.as_bytes().to_vec());
    container(
        svg(handle)
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0)),
    )
    .width(Length::Fixed(22.0))
    .height(Length::Fixed(20.0))
    .center_x(Length::Fixed(22.0))
    .center_y(Length::Fixed(20.0))
    .into()
}

fn is_image_file(name: &str) -> bool {
    matches!(
        name.rsplit('.').next(),
        Some("png")
            | Some("jpg")
            | Some("jpeg")
            | Some("gif")
            | Some("bmp")
            | Some("webp")
            | Some("svg")
    )
}

fn is_archive_file(name: &str) -> bool {
    matches!(
        name.rsplit('.').next(),
        Some("zip")
            | Some("tar")
            | Some("gz")
            | Some("tgz")
            | Some("bz2")
            | Some("7z")
            | Some("rar")
    )
}

fn is_executable_file(name: &str) -> bool {
    matches!(
        name.rsplit('.').next(),
        Some("sh") | Some("bat") | Some("exe") | Some("app") | Some("run")
    )
}

const FILE_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M7 3h7l5 5v13a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1Z" stroke="#9AA0A6" stroke-width="1.6"/><path d="M14 3v6h6" stroke="#9AA0A6" stroke-width="1.6"/></svg>"###;
const FOLDER_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M3 6a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6Z" stroke="#0A84FF" stroke-width="1.6"/></svg>"###;
const IMAGE_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="4" y="5" width="16" height="14" rx="2" stroke="#AF52DE" stroke-width="1.6"/><path d="M8 13l3-3 5 6" stroke="#AF52DE" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/><circle cx="9" cy="9" r="1.5" fill="#AF52DE"/></svg>"###;
const ARCHIVE_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="6" y="3" width="12" height="4" stroke="#FF9F0A" stroke-width="1.6"/><rect x="6" y="7" width="12" height="14" rx="2" stroke="#FF9F0A" stroke-width="1.6"/><path d="M12 10v8" stroke="#FF9F0A" stroke-width="1.6"/><path d="M10 12h4" stroke="#FF9F0A" stroke-width="1.6"/></svg>"###;
const EXEC_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="4" y="4" width="16" height="16" rx="3" stroke="#34C759" stroke-width="1.6"/><path d="M9 8l6 4-6 4V8Z" fill="#34C759"/></svg>"###;

const CHECK_SVG: &str = r###"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" stroke="#34C759" stroke-width="2.0"/><path d="M8.2 12.2l2.4 2.5 5.2-5.4" stroke="#34C759" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/></svg>"###;
const ERROR_SVG: &str = r###"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" stroke="#FF453A" stroke-width="2.0"/><path d="M12 7.2v6.4" stroke="#FF453A" stroke-width="2.0" stroke-linecap="round"/><circle cx="12" cy="16.8" r="1.2" fill="#FF453A"/></svg>"###;
const UPLOADING_SVG: &str = r###"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" stroke="#0A84FF" stroke-width="2.0"/><path d="M12 7.5v9" stroke="#0A84FF" stroke-width="2.0" stroke-linecap="round"/><path d="M9 10.5L12 7.5l3 3" stroke="#0A84FF" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/></svg>"###;
const DOWNLOADING_SVG: &str = r###"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" stroke="#0A84FF" stroke-width="2.0"/><path d="M12 7.5v9" stroke="#0A84FF" stroke-width="2.0" stroke-linecap="round"/><path d="M15 13.5l-3 3-3-3" stroke="#0A84FF" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/></svg>"###;
const QUEUED_SVG: &str = r###"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" stroke="#8E8E93" stroke-width="2.0"/><path d="M12 7.5v5.3l3.8 2.2" stroke="#8E8E93" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/></svg>"###;
const CANCEL_STATUS_SVG: &str = r###"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" stroke="#FF9F0A" stroke-width="2.0"/><path d="M8.5 8.5l7 7M15.5 8.5l-7 7" stroke="#FF9F0A" stroke-width="2.0" stroke-linecap="round"/></svg>"###;
const CANCEL_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M7 7l10 10M17 7l-10 10" stroke="#FF453A" stroke-width="2.0" stroke-linecap="round"/></svg>"###;
const RETRY_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M20 12a8 8 0 1 1-2.3-5.7" stroke="#0A84FF" stroke-width="2.0" stroke-linecap="round"/><path d="M20 4v6h-6" stroke="#0A84FF" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/></svg>"###;
const PAUSED_SVG: &str = r###"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" stroke="#FF9F0A" stroke-width="2.0"/><path d="M9.5 8.5v7" stroke="#FF9F0A" stroke-width="2.0" stroke-linecap="round"/><path d="M14.5 8.5v7" stroke="#FF9F0A" stroke-width="2.0" stroke-linecap="round"/></svg>"###;
const PAUSE_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M9 7.5v9M15 7.5v9" stroke="#FF9F0A" stroke-width="2.0" stroke-linecap="round"/></svg>"###;
const RESUME_SVG: &str = r###"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M9 7.5l7 4.5-7 4.5V7.5Z" fill="#34C759"/></svg>"###;

fn file_row(
    name: String,
    size: String,
    modified: String,
    is_dir: bool,
    selected: bool,
    hovered: bool,
    on_press: Message,
    name_column_width: f32,
    pane: SftpPane,
    _context_menu: Option<&SftpContextMenu>,
    rename_input_id: &Id,
    rename_target: Option<&crate::ui::state::SftpPendingAction>,
    rename_value: &str,
) -> Element<'static, Message> {
    let (name_style, icon) = file_icon(&name, is_dir);
    let is_renaming = rename_target
        .map(|target| target.pane == pane && target.name == name)
        .unwrap_or(false);

    let display_name = truncate_name(&name, name_column_width, 14.0);
    let name_cell: Element<'static, Message> = if is_renaming {
        text_input("New name", rename_value)
            .on_input(Message::SftpRenameInput)
            .on_submit(Message::SftpRenameConfirm)
            .id(rename_input_id.clone())
            .padding([4, 8])
            .size(13)
            .style(ui_style::dialog_input)
            .width(Length::Fixed(name_column_width))
            .into()
    } else {
        text(display_name)
            .size(14)
            .style(name_style)
            .width(Length::Fixed(name_column_width))
            .wrapping(Wrapping::None)
            .into()
    };
    let row_container = container(
        row![
            icon,
            name_cell,
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
    .padding(pad_trbl(2, 6, 2, 10))
    .width(Length::Fill)
    .style(ui_style::sftp_row_container(selected, hovered));

    let row_area = iced::widget::mouse_area(row_container)
        .on_right_press(Message::SftpOpenContextMenu(pane, name.clone()))
        .on_enter(Message::SftpFileHover(Some((pane, name.clone()))))
        .on_exit(Message::SftpFileHover(None))
        .on_press(if is_renaming {
            Message::Ignore
        } else {
            on_press
        });
    // Start drag/select on MouseDown

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
