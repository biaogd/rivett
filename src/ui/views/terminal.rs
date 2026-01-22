use crate::ui::Message;
use crate::ui::state::{SessionState, SessionTab, Spinner};
use crate::ui::style as ui_style;
use crate::ui::terminal_widget;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    tabs: &'a [SessionTab],
    active_tab: usize,
    ime_preedit: &'a str,
) -> Element<'a, Message> {
    if tabs.is_empty() {
        return column![
            container(
                column![
                    text("No open tabs").size(24).style(ui_style::header_text),
                    text("Create a new session to get started").style(ui_style::muted_text),
                    iced::widget::button(text("Create Session"))
                        .on_press(Message::CreateNewSession)
                        .padding([10, 20])
                        .style(ui_style::save_button)
                ]
                .spacing(20)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
        ]
        .into();
    }

    let (
        current_chrome_cache,
        current_line_caches,
        current_emulator,
        current_tab_state,
        _current_spinner_cache,
    ) = if let Some(tab) = tabs.get(active_tab) {
        (
            &tab.chrome_cache,
            &tab.line_caches,
            tab.emulator.clone(),
            &tab.state,
            &tab.spinner_cache,
        )
    } else {
        // Should be covered by is_empty check, but safe fallback
        (
            &tabs[0].chrome_cache,
            &tabs[0].line_caches,
            tabs[0].emulator.clone(),
            &tabs[0].state,
            &tabs[0].spinner_cache,
        )
    };

    match current_tab_state {
        SessionState::Connecting(start_time) => {
            let _elapsed = start_time.elapsed().as_secs_f32();

            let spinner = iced::widget::canvas(Spinner::new(*start_time))
                .width(Length::Fixed(50.0))
                .height(Length::Fixed(50.0));

            container(
                column![
                    spinner,
                    text("Connecting...").size(16).style(ui_style::muted_text)
                ]
                .spacing(20)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        }
        SessionState::Failed(err) => {
            let current_tab_index = active_tab;

            container(
                column![
                    text("❌ Connection Failed")
                        .size(24)
                        .color(iced::Color::from_rgb(0.8, 0.2, 0.2)),
                    text(err).size(14).style(ui_style::muted_text),
                    row![
                        iced::widget::button(text("🔄 Retry").size(14))
                            .padding([8, 16])
                            .on_press(Message::RetryConnection(current_tab_index)),
                        iced::widget::button(text("✏️ Edit").size(14))
                            .padding([8, 16])
                            .on_press(Message::EditSessionConfig(current_tab_index)),
                    ]
                    .spacing(12)
                ]
                .spacing(20)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        }
        _ => iced::widget::responsive(move |size| {
            let _cols = (size.width / terminal_widget::CELL_WIDTH) as usize;
            let _rows = (size.height / terminal_widget::CELL_HEIGHT) as usize;

            container(
                terminal_widget::TerminalView::new(
                    current_emulator.clone(),
                    current_chrome_cache,
                    current_line_caches,
                    if ime_preedit.is_empty() {
                        None
                    } else {
                        Some(ime_preedit)
                    },
                )
                .view(),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0)
            .style(ui_style::terminal_content)
            .into()
        })
        .into(),
    }
}
