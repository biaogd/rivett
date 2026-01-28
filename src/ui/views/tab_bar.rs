use crate::ui::Message;
use crate::ui::SessionTab;
use crate::ui::style as ui_style;
use iced::widget::{Space, button, container, responsive, row, text};
use iced::{Alignment, Element, Length};

fn truncate_title(title: &str, max_chars: usize) -> String {
    if max_chars <= 3 {
        return "...".to_string();
    }
    if title.chars().count() <= max_chars {
        return title.to_string();
    }
    let truncated: String = title.chars().take(max_chars - 3).collect();
    format!("{}...", truncated)
}

pub fn render<'a>(tabs: &'a [SessionTab], active_tab: usize) -> Element<'a, Message> {
    let inner = responsive(move |size| {
        let spacing = 4.0;
        let padding = 24.0;
        let plus_width = 44.0;

        let count = tabs.len().max(1) as f32;
        let available = (size.width - padding - plus_width).max(80.0);
        let tab_width = ((available - spacing * (count - 1.0)) / count).clamp(80.0, 200.0);
        let sessions_width = tab_width.min(120.0);
        let text_room = (tab_width - 44.0).max(8.0);
        let max_chars = (text_room / 7.0).floor().max(4.0) as usize;

        let tabs_row =
            tabs.iter()
                .enumerate()
                .fold(row![].spacing(spacing), |row, (index, tab)| {
                    let is_active = index == active_tab;
                    let title = truncate_title(&tab.title, max_chars);

                    let close_button: Element<'_, Message> = if index == 0 {
                        container(Space::new()).width(Length::Fixed(12.0)).into()
                    } else {
                        button(text("×").size(14))
                            .padding([0, 4])
                            .style(ui_style::tab_close_button)
                            .on_press(Message::CloseTab(index))
                            .into()
                    };

                    let tab_content = row![
                        text(title).size(13),
                        container("").width(Length::Fill),
                        close_button
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center);

                    let width = if index == 0 {
                        sessions_width
                    } else {
                        tab_width
                    };

                    row.push(
                        button(tab_content)
                            .padding([8, 12])
                            .width(Length::Fixed(width))
                            .style(ui_style::compact_tab(is_active))
                            .on_press(Message::SelectTab(index)),
                    )
                });

        let mut tab_bar = row![tabs_row].align_y(Alignment::Center).spacing(8);

        tab_bar = tab_bar.push(
            button(text("+").size(16))
                .padding([6, 12])
                .style(ui_style::new_tab_button)
                .on_press(Message::ToggleQuickConnect),
        );

        tab_bar.into()
    });

    container(inner)
        .width(Length::Fill)
        .height(Length::Shrink)
        .padding([8, 12])
        .style(ui_style::tab_bar)
        .into()
}
