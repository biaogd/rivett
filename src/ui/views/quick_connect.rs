use crate::session::SessionConfig;
use crate::ui::Message;
use crate::ui::style as ui_style;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    quick_connect_query: &'a str,
    saved_sessions: &'a [SessionConfig],
) -> Element<'a, Message> {
    // 1. Search Bar
    let search_bar = text_input("Search sessions...", quick_connect_query)
        .on_input(Message::QuickConnectQueryChanged)
        .padding(10)
        .size(14)
        .style(ui_style::search_input);

    // 2. Remote Sessions List
    let filtered_sessions: Vec<_> = saved_sessions
        .iter()
        .filter(|s| {
            quick_connect_query.is_empty()
                || s.name
                    .to_lowercase()
                    .contains(&quick_connect_query.to_lowercase())
                || s.host
                    .to_lowercase()
                    .contains(&quick_connect_query.to_lowercase())
        })
        .collect();

    let sessions_list: Element<'_, Message> = if filtered_sessions.is_empty() {
        container(
            text("No matching sessions")
                .size(14)
                .style(ui_style::muted_text),
        )
        .padding(20)
        .center_x(Length::Fill)
        .into()
    } else {
        column(
            filtered_sessions
                .iter()
                .map(|session| {
                    button(
                        row![
                            text(">_")
                                .size(14)
                                .style(ui_style::muted_text)
                                .width(Length::Fixed(24.0)),
                            column![
                                text(&session.name).size(14),
                                text(format!("{}:{}", session.host, session.port))
                                    .size(12)
                                    .style(ui_style::muted_text),
                            ]
                            .spacing(2),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding(10)
                    .style(ui_style::quick_connect_item)
                    .on_press(Message::SelectQuickConnectSession(session.id.clone()))
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(2)
        .into()
    };

    let remote_section = column![
        text("REMOTE SESSIONS")
            .size(11)
            .style(ui_style::quick_connect_section_header),
        sessions_list
    ]
    .spacing(8);

    // 3. Local System Section
    let local_section = column![
        text("LOCAL SYSTEM")
            .size(11)
            .style(ui_style::quick_connect_section_header),
        button(
            row![
                text("💻").size(16).width(Length::Fixed(24.0)),
                text("Local Terminal (Bash)").size(14),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(10)
        .style(ui_style::quick_connect_item)
        .on_press(Message::CreateLocalTab),
    ]
    .spacing(8);

    // 4. Footer Hints
    let footer = row![
        text("↑↓ NAVIGATE")
            .size(10)
            .style(ui_style::quick_connect_footer_hint),
        text("↩ SELECT")
            .size(10)
            .style(ui_style::quick_connect_footer_hint),
        Space::new().width(Length::Fill),
        text("⌘ MANAGE ALL")
            .size(10)
            .style(ui_style::quick_connect_footer_hint),
    ]
    .spacing(16)
    .padding(8);

    // Assemble Content
    let content = column![
        search_bar,
        Space::new().height(16.0),
        scrollable(column![
            remote_section,
            Space::new().height(24.0),
            local_section
        ])
        .height(Length::Fill),
        Space::new().height(16.0),
        footer
    ]
    .spacing(0)
    .padding(24)
    .width(Length::Fixed(600.0))
    .height(Length::Fixed(450.0));

    container(content)
        .style(ui_style::quick_connect_container)
        .into()
}
