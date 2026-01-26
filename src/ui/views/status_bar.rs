use crate::ui::SessionTab;
use crate::ui::style as ui_style;
use crate::ui::{ActiveView, Message};
use iced::widget::{button, container, row, text};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    tabs: &'a [SessionTab],
    active_tab: usize,
    active_view: ActiveView,
    sftp_panel_open: bool,
) -> Element<'a, Message> {
    let current_tab = tabs.get(active_tab);
    let (status_left, connection_label, sftp_enabled) = if let Some(tab) = current_tab {
        match active_view {
            ActiveView::Terminal => {
                let is_local = matches!(
                    tab.session.as_ref().map(|session| session.backend.as_ref()),
                    Some(crate::core::backend::SessionBackend::Local { .. })
                );
                let label = if is_local { "Local" } else { "SSH" };
                (tab.title.to_string(), label, !is_local)
            }
            ActiveView::SessionManager => ("Session Manager".to_string(), "", false),
        }
    } else {
        match active_view {
            ActiveView::SessionManager => ("Session Manager".to_string(), "", false),
            ActiveView::Terminal => ("No active session".to_string(), "", false),
        }
    };

    let menu_button = row![];

    let sftp_button = if sftp_enabled {
        button(text("SFTP").size(12))
            .padding([4, 10])
            .style(ui_style::menu_button(sftp_panel_open))
            .on_press(Message::ToggleSftpPanel)
    } else {
        button(text("SFTP").size(12))
            .padding([4, 10])
            .style(ui_style::menu_button_disabled())
            .on_press(Message::Ignore)
    };

    let status_bar = row![
        menu_button,
        text(status_left).size(12),
        container("").width(Length::Fill),
        sftp_button,
        text(connection_label).size(12).style(ui_style::muted_text),
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
        .padding([3, 12])
        .style(ui_style::status_bar)
        .into()
}
