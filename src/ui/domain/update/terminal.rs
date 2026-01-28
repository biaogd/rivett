use iced::Task;

use crate::terminal::input::map_key_to_input;
use crate::ui::App;
use crate::ui::message::{ActiveView, Message};
use crate::ui::state::SessionState;

pub(in crate::ui) fn handle(app: &mut App, message: Message) -> Option<Task<Message>> {
    match message {
        Message::TerminalDataReceived(tab_index, data) => {
            let next_rx = app.tabs.get(tab_index).and_then(|tab| tab.rx.clone());
            if let Some(tab) = app.tabs.get_mut(tab_index) {
                if data.is_empty() {
                    tab.state = SessionState::Disconnected;
                    return Some(Task::none());
                }

                if let Some(tx) = &tab.parser_tx {
                    if tx.send(data.clone()).is_err() {
                        tracing::warn!("parser thread unavailable, falling back to direct parse");
                        tab.emulator.process_input(&data);
                        tab.mark_full_damage();
                    }
                } else {
                    tab.emulator.process_input(&data);
                    tab.mark_full_damage();
                }
            }
            if let Some(rx) = next_rx {
                return Some(Task::perform(
                    async move {
                        let mut guard = rx.lock().await;
                        match guard.recv().await {
                            Some(data) => {
                                use std::sync::Mutex;
                                use std::sync::OnceLock;
                                use std::sync::atomic::{AtomicUsize, Ordering};
                                use std::time::Instant;

                                static RX_BYTES: AtomicUsize = AtomicUsize::new(0);
                                static LAST_LOG: OnceLock<Mutex<Instant>> = OnceLock::new();

                                RX_BYTES.fetch_add(data.len(), Ordering::Relaxed);
                                let last_log = LAST_LOG.get_or_init(|| Mutex::new(Instant::now()));
                                let mut last = last_log.lock().unwrap();
                                if last.elapsed().as_secs() >= 1 {
                                    let bytes = RX_BYTES.swap(0, Ordering::Relaxed);
                                    tracing::info!("ui recv {} bytes/s (tab {})", bytes, tab_index);
                                    *last = Instant::now();
                                }
                                (tab_index, data)
                            }
                            None => {
                                tracing::debug!("recv loop closed for tab {}", tab_index);
                                (tab_index, vec![])
                            }
                        }
                    },
                    |(idx, data)| Message::TerminalDataReceived(idx, data),
                ));
            }
            Some(Task::none())
        }
        Message::TerminalDamaged(tab_index, damage) => {
            if let Some(tab) = app.tabs.get_mut(tab_index) {
                match damage {
                    crate::terminal::TerminalDamage::Full => {
                        tab.pending_damage_full = true;
                        tab.pending_damage_lines.clear();
                        tab.is_dirty = true;
                        tab.last_data_received = std::time::Instant::now();
                    }
                    crate::terminal::TerminalDamage::Partial(lines) => {
                        tab.add_damage_lines(&lines);
                    }
                }
            }
            Some(Task::none())
        }
        Message::TerminalMousePress(col, line) => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                tab.emulator.on_mouse_press(col, line);
                tab.mark_full_damage();
            }
            Some(Task::none())
        }
        Message::TerminalMouseDrag(col, line) => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                tab.emulator.on_mouse_drag(col, line);
                tab.mark_full_damage();
            }
            Some(Task::none())
        }
        Message::TerminalMouseRelease => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                tab.emulator.on_mouse_release();
                tab.mark_full_damage();
            }
            Some(Task::none())
        }
        Message::TerminalMouseDoubleClick(col, line) => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                tab.emulator.on_mouse_double_click(col, line);
                tab.mark_full_damage();
            }
            Some(Task::none())
        }
        Message::TerminalResize(cols, rows) => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                tab.emulator.resize(cols, rows);
                tab.ensure_line_caches(rows);
                tab.mark_full_damage();

                if let Some(session) = &tab.session {
                    let session = session.clone();
                    return Some(Task::perform(
                        async move {
                            let _ = session.resize(cols as u16, rows as u16).await;
                        },
                        |_| Message::TerminalInput(vec![]),
                    ));
                }
            }
            Some(Task::none())
        }
        Message::ScrollWheel(delta) => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                if delta.abs() > 0.001 {
                    let clamped_delta = delta.clamp(-100.0, 100.0);
                    tab.emulator.scroll(clamped_delta);
                    tab.mark_full_damage();
                }
            }
            Some(Task::none())
        }
        Message::TerminalInput(data) => {
            if data.is_empty() {
                return Some(Task::none());
            }

            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                if let Some(session) = &tab.session {
                    let session = session.clone();
                    let data_to_send = app.maybe_wrap_bracketed_paste(&data);

                    return Some(Task::perform(
                        async move {
                            let write_future = session.write(&data_to_send);
                            match tokio::time::timeout(
                                std::time::Duration::from_millis(2000),
                                write_future,
                            )
                            .await
                            {
                                Ok(Ok(_)) => {}
                                Ok(Err(e)) => tracing::warn!("ui write error: {}", e),
                                Err(_) => tracing::warn!("ui write timeout - session unresponsive"),
                            }
                        },
                        |_| Message::TerminalInput(vec![]),
                    ));
                } else {
                    println!("UI: Tab {} ignoring input (no session)", app.active_tab);
                }
            } else {
                println!("UI: Tab {} ignoring input (invalid index)", app.active_tab);
            }
            Some(Task::none())
        }
        Message::Copy => {
            if let Some(tab) = app.tabs.get(app.active_tab) {
                if let Some(content) = tab.emulator.copy_selection() {
                    return Some(iced::clipboard::write(content));
                }
            }
            Some(Task::none())
        }
        Message::Paste => Some(iced::clipboard::read().map(Message::ClipboardReceived)),
        Message::ClipboardReceived(content) => {
            if let Some(text) = content {
                app.ime_ignore_next_input = true;
                app.ime_buffer.clear();
                return Some(Task::done(Message::TerminalInput(
                    app.bracketed_paste_bytes(&text),
                )));
            }
            Some(Task::none())
        }
        Message::ImeBufferChanged(value) => {
            if app.ime_ignore_next_input {
                app.ime_ignore_next_input = false;
                app.ime_buffer.clear();
                return Some(Task::none());
            }

            let prev = app.ime_buffer.clone();
            app.ime_buffer = value.clone();
            if app.active_view != ActiveView::Terminal || app.show_quick_connect {
                return Some(Task::none());
            }

            if !app.ime_focused {
                app.ime_buffer.clear();
                return Some(Task::none());
            }

            if !app.ime_preedit.is_empty() {
                return Some(Task::none());
            }

            if value == prev {
                return Some(Task::none());
            }

            if value.starts_with(&prev) {
                let suffix = &value[prev.len()..];
                if suffix.is_empty() {
                    return Some(Task::none());
                }
                return Some(Task::done(Message::TerminalInput(
                    suffix.as_bytes().to_vec(),
                )));
            }

            if prev.starts_with(&value) {
                let removed = prev.chars().count().saturating_sub(value.chars().count());
                if removed == 0 {
                    return Some(Task::none());
                }
                let mut data = Vec::with_capacity(removed);
                data.extend(std::iter::repeat(0x08u8).take(removed));
                return Some(Task::done(Message::TerminalInput(data)));
            }

            let mut data = Vec::new();
            let remove_count = prev.chars().count();
            data.extend(std::iter::repeat(0x08u8).take(remove_count));
            data.extend(value.as_bytes());
            if data.is_empty() {
                return Some(Task::none());
            }
            Some(Task::done(Message::TerminalInput(data)))
        }
        Message::ImePaste => {
            app.ime_ignore_next_input = true;
            app.ime_buffer.clear();
            Some(iced::clipboard::read().map(Message::ClipboardReceived))
        }
        Message::ImeFocusChanged(focused) => {
            app.ime_focused = focused;
            if app.active_view == ActiveView::Terminal && !app.show_quick_connect && !focused {
                return Some(app.focus_terminal_ime());
            }
            Some(Task::none())
        }
        _ => None,
    }
}

pub(in crate::ui) fn handle_runtime_event(
    app: &mut App,
    event: &iced::event::Event,
    window: iced::window::Id,
) -> Option<Task<Message>> {
    if Some(window) != app.main_window
        || app.active_view != ActiveView::Terminal
        || app.show_quick_connect
    {
        return Some(Task::none());
    }

    match event {
        iced::event::Event::InputMethod(event) => {
            match event {
                iced_core::input_method::Event::Opened => {
                    app.ime_focused = true;
                    app.ime_preedit.clear();
                }
                iced_core::input_method::Event::Closed => {
                    app.ime_focused = false;
                    app.ime_preedit.clear();
                }
                iced_core::input_method::Event::Commit(text) => {
                    app.ime_preedit.clear();
                    app.ime_ignore_next_input = true;
                    app.ime_buffer.clear();
                    if !text.is_empty() {
                        return Some(Task::done(Message::TerminalInput(text.as_bytes().to_vec())));
                    }
                }
                iced_core::input_method::Event::Preedit(content, _) => {
                    app.ime_preedit = content.clone();
                }
            }
            Some(Task::none())
        }
        iced::event::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) => {
            let message = {
                if app.ime_focused
                    && !app.ime_preedit.is_empty()
                    && matches!(
                        key,
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace)
                            | iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
                    )
                {
                    Message::Ignore
                } else if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace)
                ) {
                    Message::TerminalInput(vec![0x7f])
                } else if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
                ) {
                    Message::TerminalInput(vec![0x1b, b'[', b'3', b'~'])
                } else if modifiers.command() {
                    match key {
                        iced::keyboard::Key::Character(c) if c.as_str() == "c" => Message::Copy,
                        iced::keyboard::Key::Character(c) if c.as_str() == "v" => {
                            if app.ime_focused {
                                Message::Ignore
                            } else {
                                Message::Paste
                            }
                        }
                        _ => Message::Ignore,
                    }
                } else if modifiers.command()
                    && matches!(key, iced::keyboard::Key::Character(c) if c.as_str() == "t")
                {
                    Message::CreateLocalTab
                } else {
                    let s = text.as_ref().map(|t| t.as_str()).unwrap_or("");
                    if !s.is_empty() && !s.chars().any(|c| c.is_control()) {
                        if app.ime_focused || !app.ime_preedit.is_empty() {
                            Message::Ignore
                        } else {
                            Message::TerminalInput(s.as_bytes().to_vec())
                        }
                    } else if matches!(key, iced::keyboard::Key::Character(_))
                        && !modifiers.control()
                    {
                        if s.is_empty() || app.ime_focused || !app.ime_preedit.is_empty() {
                            Message::Ignore
                        } else {
                            Message::TerminalInput(s.as_bytes().to_vec())
                        }
                    } else if let Some(data) = map_key_to_input(key.clone(), *modifiers) {
                        Message::TerminalInput(data)
                    } else {
                        Message::Ignore
                    }
                }
            };

            if matches!(message, Message::Ignore) {
                return Some(Task::none());
            }
            Some(Task::done(message))
        }
        iced::event::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
            let delta_y = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => *y,
                iced::mouse::ScrollDelta::Pixels { y, .. } => *y / 20.0,
            };
            Some(Task::done(Message::ScrollWheel(delta_y)))
        }
        _ => Some(Task::none()),
    }
}
