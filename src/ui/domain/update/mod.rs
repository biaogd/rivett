mod local;
mod sessions;
mod terminal;
mod window;

use iced::Task;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::session::Session;
use crate::ui::message::{ActiveView, Message};
use crate::ui::state::SessionState;
use crate::ui::App;

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let mut commands = Vec::new();

        match message {
            Message::CreateLocalTab => {
                return local::create_local_tab(self);
            }
            // Message::CreateSession => { ... } // Removed
            Message::SelectTab(index) => {
                println!("UI: Selecting tab {}", index);
                if index < self.tabs.len() {
                    self.active_tab = index;
                    if self.active_view == ActiveView::Terminal && !self.show_quick_connect {
                        commands.push(self.focus_terminal_ime());
                    }
                }
            }
            Message::CloseTab(index) => {
                if index < self.tabs.len() {
                    self.tabs.remove(index);
                    if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                        self.active_tab -= 1;
                    }
                }
            }
            Message::ToggleMenu => {
                self.show_menu = !self.show_menu;

                // Recalculate layout and trigger resize
                let width = self.window_width;
                let height = self.window_height;

                if width > 0 && height > 0 {
                    let sidebar_width = if self.show_menu { 200.0 } else { 0.0 };
                    let h_padding = 24.0;
                    let v_padding = 120.0;

                    let term_w = (width as f32 - sidebar_width - h_padding).max(0.0);
                    let term_h = (height as f32 - v_padding).max(0.0);

                    let cols = (term_w / self.cell_width()) as usize;
                    let rows = (term_h / self.cell_height()) as usize;

                    println!("Menu toggled. Resizing to {}x{}", cols, rows);
                    return Task::done(Message::TerminalResize(cols, rows));
                }
            }
            Message::ShowSessionManager => {
                self.show_quick_connect = false;
                self.show_menu = false;
                self.active_view = ActiveView::SessionManager;
                self.editing_session = None;
            }
            Message::ShowSftp => {
                self.show_menu = false;
                // TODO: Show SFTP interface
            }
            Message::ShowPortForwarding => {
                self.show_menu = false;
                // TODO: Show port forwarding manager
            }
            Message::ShowSettings => {
                self.show_quick_connect = false;
                self.session_menu_open = None;
                self.open_settings_window();
            }
            Message::WindowResized(_, _) | Message::WindowOpened(_) | Message::WindowClosed(_) => {
                if let Some(task) = window::handle(self, message) {
                    return task;
                }
            }
            Message::RuntimeEvent(event, window_id) => {
                if let Some(task) = window::handle_runtime_event(self, &event, window_id) {
                    return task;
                }
                if let Some(task) = terminal::handle_runtime_event(self, &event, window_id) {
                    return task;
                }
            }
            Message::CreateNewSession
            | Message::EditSession(_)
            | Message::DeleteSession(_)
            | Message::ConnectToSession(_)
            | Message::SaveSession
            | Message::CancelSessionEdit
            | Message::CloseSessionManager
            | Message::ToggleAuthMethod
            | Message::ClearValidationError
            | Message::SessionNameChanged(_)
            | Message::SessionHostChanged(_)
            | Message::SessionPortChanged(_)
            | Message::SessionUsernameChanged(_)
            | Message::SessionPasswordChanged(_)
            | Message::ToggleSessionMenu(_)
            | Message::CloseSessionMenu => {
                return sessions::handle(self, message);
            }
            Message::SessionConnected(result, tab_index) => match result {
                Ok((session, rx)) => {
                    if let Some(tab) = self.tabs.get_mut(tab_index) {
                        tab.title = format!("{} (Connected)", tab.title);
                        tab.ssh_handle = Some(session.clone()); // Store SSH handle
                        tab.session = None; // Not fully ready (shell not opened)
                        tab.rx = Some(rx.clone());
                        tab.state = SessionState::Connected; // Transition to Connected

                        // Open Shell
                        let session_clone = session.clone();
                        let open_shell_task = Task::perform(
                            async move {
                                let mut guard = session_clone.lock().await;
                                match guard.open_shell().await {
                                    Ok(id) => Ok(id),
                                    Err(e) => Err(e.to_string()),
                                }
                            },
                            move |result| Message::ShellOpened(result, tab_index),
                        );

                        // Start reading loop
                        let rx_clone = rx.clone();
                        let read_task = Task::perform(
                            async move {
                                let mut guard = rx_clone.lock().await;
                                match guard.recv().await {
                                    Some(data) => (tab_index, data),
                                    None => (tab_index, vec![]),
                                }
                            },
                            |(idx, data)| Message::TerminalDataReceived(idx, data),
                        );

                        return Task::batch(vec![open_shell_task, read_task]);
                    }
                }
                Err(e) => {
                    // Record the error with timestamp
                    self.last_error = Some((e.clone(), std::time::Instant::now()));

                    if let Some(tab) = self.tabs.get_mut(tab_index) {
                        tab.title = format!("{} (Failed)", tab.title);
                        tab.state = SessionState::Failed(e.clone()); // Transition to Failed
                    }
                    println!("Connection failed: {}", e);
                }
            },
            Message::ShellOpened(result, tab_index) => match result {
                Ok(id) => {
                    if let Some(tab) = self.tabs.get_mut(tab_index) {
                        println!("Shell opened on channel {:?} for tab {}", id, tab_index);

                        // Create Unified Session
                        if let Some(ssh_handle) = &tab.ssh_handle {
                            let backend = crate::core::backend::SessionBackend::Ssh {
                                session: ssh_handle.clone(),
                                channel_id: id,
                            };
                            tab.session = Some(Session::new(backend));

                            // Wire up terminal responses (CPR) for SSH
                            if let Some(mut output_rx) = tab.emulator.take_output_receiver() {
                                if let Some(session) = &tab.session {
                                    let session_clone = session.clone();
                                    std::thread::spawn(move || {
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        rt.block_on(async {
                                            while let Some(data) = output_rx.recv().await {
                                                // println!("SSH: Sending terminal response: {} bytes", data.len());
                                                // Add timeout to prevent hanging if connection is dead
                                                let write_future = session_clone.write(&data);
                                                match tokio::time::timeout(std::time::Duration::from_millis(1000), write_future).await {
                                                    Ok(Ok(_)) => {},
                                                    Ok(Err(e)) => {
    println!("SSH: Failed to write terminal response: {}", e);
                                                        break;
                                                    },
                                                    Err(_) => {
                                                        println!("SSH: Write terminal response timeout - connection might be dead");
                                                        // We don't break here immediately, hoping it's temporary? 
                                                        // Or we should? If TCP is stuck, it's stuck.
                                                    }
                                                }
                                            }
                                        });
                                    });
                                }
                            }
                        }

                        // Trigger initial resize based on current window size
                        let width = self.window_width;
                        let height = self.window_height;
                        if width > 0 && height > 0 {
                            let sidebar_width = if self.show_menu { 200.0 } else { 0.0 };
                            let h_padding = 24.0;
                            let v_padding = 120.0;

                            let term_w = (width as f32 - sidebar_width - h_padding).max(0.0);
                            let term_h = (height as f32 - v_padding).max(0.0);

                            let cols = (term_w / self.cell_width()) as usize;
                            let rows = (term_h / self.cell_height()) as usize;

                            return Task::done(Message::TerminalResize(cols, rows));
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to open shell: {}", e);
                    if let Some(tab) = self.tabs.get_mut(tab_index) {
                        tab.state = SessionState::Failed(format!("Failed to open shell: {}", e));
                    }
                }
            },
            Message::TerminalDataReceived(tab_index, data) => {
                if let Some(task) = terminal::handle(self, Message::TerminalDataReceived(tab_index, data)) {
                    return task;
                }
            }
            Message::TerminalDamaged(_, _)
            | Message::TerminalMousePress(_, _)
            | Message::TerminalMouseDrag(_, _)
            | Message::TerminalMouseRelease
            | Message::TerminalMouseDoubleClick(_, _)
            | Message::TerminalResize(_, _)
            | Message::ScrollWheel(_)
            | Message::TerminalInput(_)
            | Message::Copy
            | Message::Paste
            | Message::ClipboardReceived(_)
            | Message::ImeBufferChanged(_)
            | Message::ImeFocusChanged(_)
            | Message::ImePaste => {
                if let Some(task) = terminal::handle(self, message) {
                    return task;
                }
            }
            Message::ToggleQuickConnect => {
                self.show_quick_connect = !self.show_quick_connect;
                if self.show_quick_connect {
                    self.quick_connect_query = String::new(); // Reset query on open
                } else if self.active_view == ActiveView::Terminal {
                    commands.push(self.focus_terminal_ime());
                }
            }
            Message::QuickConnectQueryChanged(query) => {
                self.quick_connect_query = query;
            }
            Message::SelectQuickConnectSession(name) => {
                self.show_quick_connect = false;
                return Task::perform(async move { name }, Message::ConnectToSession);
            }
            Message::Tick(_now) => {
                crate::platform::maybe_setup_macos_menu();
                if crate::platform::take_settings_request() {
                    self.show_quick_connect = false;
                    self.session_menu_open = None;
                    self.open_settings_window();
                }

                // Spinner animation
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let SessionState::Connecting(_) = tab.state {
                        tab.spinner_cache.clear();
                    }
                }

                if let Some((cols, rows, at)) = self.pending_resize {
                    if std::time::Instant::now().duration_since(at)
                        > std::time::Duration::from_millis(120)
                    {
                        self.pending_resize = None;
                        return Task::done(Message::TerminalResize(cols, rows));
                    }
                }

                if self.active_view == ActiveView::Terminal
                    && !self.show_quick_connect
                    && std::time::Instant::now().duration_since(self.last_ime_focus_check)
                        > std::time::Duration::from_millis(120)
                {
                    self.last_ime_focus_check = std::time::Instant::now();
                    commands.push(
                        iced::widget::operation::is_focused(self.ime_input_id.clone())
                            .map(Message::ImeFocusChanged),
                    );
                }

                // Throttled rendering with debounce
                let now = std::time::Instant::now();
                for tab in &mut self.tabs {
                    if tab.is_dirty {
                        let stable_enough = now.duration_since(tab.last_data_received)
                            > std::time::Duration::from_millis(5);
                        let force_update = now.duration_since(tab.last_redraw_time)
                            > std::time::Duration::from_millis(16);

                        if stable_enough || force_update {
                            tab.chrome_cache.clear();
                            if tab.pending_damage_full {
                                for cache in &mut tab.line_caches {
                                    cache.clear();
                                }
                            } else {
                                tab.pending_damage_lines.sort_unstable();
                                tab.pending_damage_lines.dedup();
                                for &line in &tab.pending_damage_lines {
                                    if let Some(cache) = tab.line_caches.get_mut(line) {
                                        cache.clear();
                                    }
                                }
                            }
                            tab.pending_damage_full = false;
                            tab.pending_damage_lines.clear();
                            tab.is_dirty = false;
                            tab.last_redraw_time = now;
                        }
                    }
                }
            }
            Message::RetryConnection(tab_index) => {
                // Actually retry the SSH connection
                if let Some(tab) = self.tabs.get_mut(tab_index) {
                    tab.state = SessionState::Connecting(std::time::Instant::now());

                    // For now, we need the session config to retry
                    // TODO: Store session config with each tab for retry
                    // As a workaround, try to find matching saved session
                    if let Some(saved_session) = self.saved_sessions.first() {
                        let host = saved_session.host.clone();
                        let port = saved_session.port;
                        let username = saved_session.username.clone();
                        let password = saved_session.password.clone().unwrap_or_default();

                        return Task::perform(
                            async move {
                                // Add timeout wrapper
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(10),
                                    crate::ssh::SshSession::connect(
                                        &host, port, &username, &password,
                                    ),
                                )
                                .await
                                {
                                    Ok(Ok((session, rx))) => Ok((
                                        Arc::new(Mutex::new(session)),
                                        Arc::new(Mutex::new(rx)),
                                    )),
                                    Ok(Err(e)) => Err(e.to_string()),
                                    Err(_) => Err("Connection timeout (10s)".to_string()),
                                }
                            },
                            move |result| Message::SessionConnected(result, tab_index),
                        );
                    }
                }
            }
            Message::EditSessionConfig(tab_index) => {
                // Switch to session manager and load the session for editing
                if tab_index < self.tabs.len() {
                    self.active_view = ActiveView::SessionManager;
                    // TODO: Load the session config for editing
                }
            }
            Message::Ignore => {}
        }
        Task::batch(commands)
    }

}
