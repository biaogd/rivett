use iced::Task;

use crate::ui::App;
use crate::ui::message::{ActiveView, Message};

pub(in crate::ui) fn handle(app: &mut App, message: Message) -> Option<Task<Message>> {
    match message {
        Message::WindowResized(width, height) => {
            app.window_width = width;
            app.window_height = height;
            app.sftp_dragging = false;

            let sidebar_width = if app.show_menu { 200.0 } else { 0.0 };
            let h_padding = 24.0;
            let v_padding = 120.0;

            let term_w = (width as f32 - sidebar_width - h_padding).max(0.0);
            let term_h = (height as f32 - v_padding).max(0.0);

            let cols = (term_w / app.cell_width()) as usize;
            let rows = (term_h / app.cell_height()) as usize;

            if width > 0 {
                let max_width = (width as f32 - 240.0).max(320.0);
                app.sftp_panel_width = app.sftp_panel_width.clamp(280.0, max_width);
            }
            app.pending_resize = Some((cols, rows, std::time::Instant::now()));
            Some(Task::done(Message::TerminalResize(cols, rows)))
        }
        Message::WindowOpened(_id) => Some(Task::none()),
        Message::WindowClosed(id) => {
            if Some(id) == app.main_window {
                app.main_window = None;
                Some(iced::exit())
            } else {
                Some(Task::none())
            }
        }
        _ => None,
    }
}

pub(in crate::ui) fn handle_runtime_event(
    app: &mut App,
    event: &iced::event::Event,
    window: iced::window::Id,
) -> Option<Task<Message>> {
    if Some(window) == app.main_window {
        if app.sftp_panel_open
            && app
                .sftp_state_for_tab(app.active_tab)
                .map(|state| state.rename_target.is_some())
                .unwrap_or(false)
        {
            if let iced::event::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) =
                event
            {
                if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                ) {
                    return Some(Task::done(Message::SftpRenameCancel));
                }
            }
        }

        match event {
            iced::event::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                if app.sftp_file_dragging.is_some() {
                    return Some(Task::done(Message::SftpFileDragEnd));
                }
            }
            iced::event::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                if app.sftp_file_dragging.is_some() {
                    return Some(Task::done(Message::SftpFileDragUpdate(*position)));
                }
            }
            iced::event::Event::Window(iced::window::Event::Focused) => {
                app.ime_focused = false;
                app.reload_settings();
                if app.active_view == ActiveView::Terminal && !app.show_quick_connect {
                    return Some(Task::batch(vec![
                        app.focus_terminal_ime(),
                        app.recalc_terminal_size(),
                    ]));
                }
                return Some(Task::none());
            }
            iced::event::Event::Window(iced::window::Event::Unfocused) => {
                app.ime_focused = false;
                return Some(Task::none());
            }
            iced::event::Event::Window(iced::window::Event::Resized(size)) => {
                return Some(Task::done(Message::WindowResized(
                    size.width as u32,
                    size.height as u32,
                )));
            }
            _ => {}
        }
    }

    None
}
