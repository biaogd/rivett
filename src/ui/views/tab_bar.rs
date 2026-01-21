use crate::ui::SessionTab;
use crate::ui::style as ui_style;
use crate::ui::{ActiveView, Message};
use iced::widget::{button, container, row, text};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    tabs: &'a [SessionTab],
    active_tab: usize,
    active_view: ActiveView,
) -> Element<'a, Message> {
    let mut tabs_row = tabs
        .iter()
        .enumerate()
        .fold(row![].spacing(4), |row, (index, tab)| {
            let is_active = index == active_tab && active_view == ActiveView::Terminal;

            // Tab with close button
            let tab_content = row![
                text(&tab.title).size(13),
                button(text("×").size(14))
                    .padding([0, 4])
                    .style(ui_style::tab_close_button)
                    .on_press(Message::CloseTab(index)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            row.push(
                button(tab_content)
                    .padding([8, 16])
                    .style(ui_style::compact_tab(is_active))
                    .on_press(Message::SelectTab(index)),
            )
        });

    // Only show '+' button if we are NOT in the Session Manager view
    if active_view != ActiveView::SessionManager {
        tabs_row = tabs_row.push(
            button(text("+").size(16))
                .padding([6, 12])
                .style(ui_style::new_tab_button)
                .on_press(Message::ToggleQuickConnect),
        );
    }

    let tab_bar = tabs_row
        .push(container("").width(Length::Fill))
        .align_y(Alignment::Center)
        .spacing(8);

    container(tab_bar)
        .width(Length::Fill)
        .padding([8, 12])
        .style(ui_style::tab_bar)
        .into()
}
