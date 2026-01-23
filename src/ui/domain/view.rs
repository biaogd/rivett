use iced::{Alignment, Element, Length};

use crate::ui::message::{ActiveView, Message};
use crate::ui::{components, views};
use crate::ui::style as ui_style;
use crate::ui::App;

impl App {
    pub fn view(&self, _window: iced::window::Id) -> Element<'_, Message> {
        use iced::widget::container::transparent;
        use iced::widget::{Space, button, column, container, row, stack, text_input};

        let mut content = match self.active_view {
            ActiveView::Terminal => views::terminal::render(
                &self.tabs,
                self.active_tab,
                &self.ime_preedit,
                self.terminal_font_size,
            ),
            ActiveView::SessionManager => views::session_manager::render(
                &self.saved_sessions,
                &self.session_search_query,
                self.editing_session.as_ref(),
                &self.form_name,
                &self.form_host,
                &self.form_port,
                &self.form_username,
                &self.form_password,
                self.auth_method_password,
                self.validation_error.as_ref(),
                self.session_menu_open.as_deref(),
            ),
        };
        if self.active_view == ActiveView::Terminal && !self.show_quick_connect {
            let (cursor_col, cursor_row) = self
                .tabs
                .get(self.active_tab)
                .map(|tab| tab.emulator.cursor_position())
                .unwrap_or((0, 0));
            let cursor_x = cursor_col as f32 * self.cell_width();
            let cursor_y = cursor_row as f32 * self.cell_height() + self.cell_height();

            let ime_input = text_input("", &self.ime_buffer)
                .on_input(Message::ImeBufferChanged)
                .on_paste(|_| Message::ImePaste)
                .id(self.ime_input_id.clone())
                .size(1)
                .padding(0)
                .width(Length::Fixed(1.0))
                .style(ui_style::ime_input);
            let ime_layer = column![
                Space::new()
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(cursor_y)),
                row![
                    Space::new()
                        .width(Length::Fixed(cursor_x))
                        .height(Length::Fixed(1.0)),
                    ime_input
                ]
            ]
            .width(Length::Fill)
            .height(Length::Fill);
            content = stack![content, ime_layer].into();
        }

        // Build layout from top to bottom: tab_bar (if terminal) -> content -> status_bar
        let mut main_layout = column![];

        // Tab bar at the top (only in terminal view)
        main_layout = main_layout.push(views::tab_bar::render(
            &self.tabs,
            self.active_tab,
        ));

        // Main content
        main_layout = main_layout.push(content);

        // Status bar at the bottom
        main_layout = main_layout.push(views::status_bar::render(
            &self.tabs,
            self.active_tab,
            self.active_view,
            self.show_menu,
            self.sftp_panel_open,
        ));

        let base_container = container(main_layout.spacing(0).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::app_background);

        let content_view: Element<'_, Message> = if self.show_menu {
            let left_menu = container(views::sidebar::render(self.active_view))
                .width(Length::Fixed(200.0))
                .height(Length::Fill)
                .padding(12)
                .style(ui_style::dropdown_menu);

            container(row![left_menu, base_container].spacing(0))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            base_container.into()
        };

        let main_view: Element<'_, Message> = if self.sftp_panel_open {
            let handle = iced::widget::mouse_area(
                container(Space::new())
                    .width(Length::Fixed(10.0))
                    .height(Length::Fill),
            )
            .interaction(iced::mouse::Interaction::ResizingHorizontally)
            .on_press(Message::SftpDragStart);

            let sftp_content = container(views::sftp::render(
                &self.sftp_local_path,
                &self.sftp_remote_path,
                &self.sftp_local_entries,
                self.sftp_local_error.as_deref(),
            ))
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill);

            let sftp_panel = container(
                row![handle, sftp_content]
                    .spacing(0)
                    .align_y(Alignment::Center),
            )
            .width(Length::Fixed(self.sftp_panel_width))
            .height(Length::Fill)
            .style(ui_style::drawer_panel);

            let backdrop = button(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::modal_backdrop)
            .on_press(Message::ToggleSftpPanel);

            let overlay = container(
                iced::widget::mouse_area(sftp_panel).on_press(Message::Ignore),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End);

            let layered = stack![content_view, backdrop, overlay];

            iced::widget::mouse_area(layered)
                .on_move(Message::SftpDragMove)
                .on_release(Message::SftpDragEnd)
                .into()
        } else {
            content_view
        };

        // Quick Connect overlay
        let view_with_quick_connect = if self.show_quick_connect {
            // Center the popover
            let popover = container(views::quick_connect::render(
                &self.quick_connect_query,
                &self.saved_sessions,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

            // Dark semi-transparent overlay
            let overlay = button(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(transparent),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::modal_backdrop)
            .on_press(Message::ToggleQuickConnect);

            stack![main_view, overlay, popover].into()
        } else {
            main_view
        };

        // Session Dialog overlay (on top of everything)
        if self.active_view == ActiveView::SessionManager && self.editing_session.is_some() {
            // Dark semi-transparent backdrop
            let backdrop = button(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::modal_backdrop)
            .on_press(Message::CancelSessionEdit);

            // Centered dialog wrapped in mouse_area to capture clicks
            let dialog_content = components::session_dialog::render(
                self.editing_session.as_ref(),
                &self.saved_sessions,
                &self.form_name,
                &self.form_host,
                &self.form_port,
                &self.form_username,
                &self.form_password,
                &self.form_key_path,
                &self.form_key_passphrase,
                self.auth_method_password,
                self.show_password,
                &self.connection_test_status,
                self.validation_error.as_ref(),
            );

            // Wrap in mouse_area to prevent click-through
            let dialog = container(
                iced::widget::mouse_area(dialog_content).on_press(Message::Ignore), // Capture clicks but do nothing
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

            stack![view_with_quick_connect, backdrop, dialog].into()
        } else {
            view_with_quick_connect
        }
    }

}
