use crate::ui::SessionTab;
use crate::ui::style as ui_style;
use crate::ui::{ActiveView, Message};
use iced::widget::{button, container, row, text};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    tabs: &'a [SessionTab],
    active_tab: usize,
    active_view: ActiveView,
    show_menu: bool,
    sftp_panel_open: bool,
) -> Element<'a, Message> {
    let current_tab = tabs.get(active_tab);
    let status_left = if let Some(tab) = current_tab {
        match active_view {
            ActiveView::Terminal => format!("{}  ● Connected 120ms", tab.title),
            ActiveView::SessionManager => "Session Manager".to_string(),
        }
    } else {
        match active_view {
            ActiveView::SessionManager => "Session Manager".to_string(),
            ActiveView::Terminal => "No active session".to_string(),
        }
    };

    let menu_button = if !show_menu {
        row![
            button(text("≡").size(20))
                .padding([4, 8])
                .style(ui_style::menu_button(show_menu))
                .on_press(Message::ToggleMenu),
            text("│").size(12).style(ui_style::muted_text),
        ]
    } else {
        row![]
    };

    let status_bar = row![
        menu_button,
        text(status_left).size(12),
        container("").width(Length::Fill),
        button(text("SFTP").size(12))
            .padding([4, 10])
            .style(ui_style::menu_button(sftp_panel_open))
            .on_press(Message::ToggleSftpPanel),
        text("UTF-8").size(12).style(ui_style::muted_text),
        text("│").size(12).style(ui_style::muted_text),
        text("24x120").size(12).style(ui_style::muted_text),
        text("│").size(12).style(ui_style::muted_text),
        text("↑ 3.2MB/s").size(12).style(ui_style::muted_text),
    ]
    .align_y(Alignment::Center)
    .spacing(8);

    container(status_bar)
        .width(Length::Fill)
        .padding([6, 12])
        .style(ui_style::status_bar)
        .into()
}
