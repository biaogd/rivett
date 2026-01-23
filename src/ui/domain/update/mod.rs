mod local;
mod sessions;
mod terminal;
mod window;

use iced::Task;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::session::Session;
use crate::ui::message::{ActiveView, Message};
use crate::ui::state::{SessionState, SftpEntry};
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
                    if index == 0 {
                        self.active_view = ActiveView::SessionManager;
                    } else {
                        self.active_view = ActiveView::Terminal;
                        self.last_terminal_tab = index;
                        if !self.show_quick_connect {
                            commands.push(self.focus_terminal_ime());
                        }
                    }
                    if self.sftp_panel_open {
                        if let Some(task) = start_remote_list(self, self.active_tab) {
                            return task;
                        }
                    }
                }
            }
            Message::CloseTab(index) => {
                if index == 0 {
                    return Task::none();
                }
                if index < self.tabs.len() {
                    self.tabs.remove(index);
                    if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                        self.active_tab -= 1;
                    }
                    if self.last_terminal_tab == index {
                        self.last_terminal_tab = self.active_tab;
                    } else if self.last_terminal_tab > index {
                        self.last_terminal_tab -= 1;
                    }
                    if self.active_tab == 0 {
                        self.active_view = ActiveView::SessionManager;
                    } else {
                        self.active_view = ActiveView::Terminal;
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
                self.active_view = ActiveView::SessionManager;
                self.editing_session = None;
                self.active_tab = 0;
            }
            Message::ToggleSftpPanel => {
                self.sftp_panel_open = !self.sftp_panel_open;
                self.sftp_dragging = false;
                if self.sftp_panel_open {
                    if self.window_width > 0 {
                        self.sftp_panel_width =
                            (self.window_width as f32 * 0.45).clamp(420.0, 720.0);
                    }
                    let result = load_local_entries(&self.sftp_local_path);
                    match result {
                        Ok(entries) => {
                            self.sftp_local_entries = entries;
                            self.sftp_local_error = None;
                        }
                        Err(err) => {
                            self.sftp_local_entries.clear();
                            self.sftp_local_error = Some(err);
                        }
                    }
                    if let Some(task) = start_remote_list(self, self.active_tab) {
                        return task;
                    }
                }
            }
            Message::SftpDragStart => {
                self.sftp_dragging = true;
            }
            Message::SftpDragEnd => {
                self.sftp_dragging = false;
            }
            Message::SftpDragMove(point) => {
                if self.sftp_dragging && self.window_width > 0 {
                    let max_width = (self.window_width as f32 - 240.0).max(320.0);
                    let width = (self.window_width as f32 - point.x).clamp(280.0, max_width);
                    self.sftp_panel_width = width;
                }
            }
            Message::SftpLocalPathChanged(path) => {
                self.sftp_local_path = path;
                let result = load_local_entries(&self.sftp_local_path);
                match result {
                    Ok(entries) => {
                        self.sftp_local_entries = entries;
                        self.sftp_local_error = None;
                    }
                    Err(err) => {
                        self.sftp_local_entries.clear();
                        self.sftp_local_error = Some(err);
                    }
                }
            }
            Message::SftpRemotePathChanged(path) => {
                self.sftp_remote_path = path;
                if let Some(task) = start_remote_list(self, self.active_tab) {
                    return task;
                }
            }
            Message::SftpRemoteLoaded(tab_index, result) => {
                if tab_index != self.active_tab {
                    return Task::none();
                }
                self.sftp_remote_loading = false;
                match result {
                    Ok(entries) => {
                        self.sftp_remote_entries = entries;
                        self.sftp_remote_error = None;
                    }
                    Err(err) => {
                        self.sftp_remote_entries.clear();
                        self.sftp_remote_error = Some(err);
                    }
                }
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
            | Message::TogglePasswordVisibility
            | Message::SessionKeyPathChanged(_)
            | Message::SessionKeyPassphraseChanged(_)
            | Message::SessionSearchChanged(_)
            | Message::TestConnection
            | Message::TestConnectionResult(_)
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
                        let password = saved_session.password.clone();
                        let auth_method = saved_session.auth_method.clone();
                        let key_passphrase = saved_session.key_passphrase.clone();

                        return Task::perform(
                            async move {
                                match crate::ssh::SshSession::connect(
                                    &host,
                                    port,
                                    &username,
                                    auth_method,
                                    password,
                                    key_passphrase,
                                )
                                .await
                                {
                                    Ok((session, rx)) => Ok((
                                        Arc::new(Mutex::new(session)),
                                        Arc::new(Mutex::new(rx)),
                                    )),
                                    Err(e) => Err(e.to_string()),
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

fn load_local_entries(path: &str) -> Result<Vec<SftpEntry>, String> {
    let expanded = expand_tilde(path);
    let target = if expanded.trim().is_empty() {
        expand_tilde("~")
    } else {
        expanded
    };

    let dir = std::fs::read_dir(&target)
        .map_err(|e| format!("Failed to read {}: {}", target, e))?;

    let mut entries = Vec::new();
    for entry in dir {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let meta = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let is_dir = meta.is_dir();
        let size = if is_dir { None } else { Some(meta.len()) };
        let modified = meta
            .modified()
            .ok()
            .map(|time| chrono::DateTime::<chrono::Local>::from(time));
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        entries.push(SftpEntry {
            name,
            size,
            modified,
            is_dir,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            let rest = path.trim_start_matches("~/").trim_start_matches('~');
            if rest.is_empty() {
                return home.to_string_lossy().to_string();
            }
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn start_remote_list(app: &mut App, tab_index: usize) -> Option<Task<Message>> {
    if tab_index == 0 || tab_index >= app.tabs.len() {
        app.sftp_remote_entries.clear();
        app.sftp_remote_error = Some("No active SSH session".to_string());
        app.sftp_remote_loading = false;
        return None;
    }

    let tab = app.tabs.get(tab_index)?;
    let session = match &tab.session {
        Some(session) => session.clone(),
        None => {
            app.sftp_remote_entries.clear();
            app.sftp_remote_error = Some("No active SSH session".to_string());
            app.sftp_remote_loading = false;
            return None;
        }
    };

    let sftp_session = tab.sftp_session.clone();
    let path = normalize_remote_path(&app.sftp_remote_path);
    app.sftp_remote_loading = true;
    app.sftp_remote_error = None;

    Some(Task::perform(
        async move { load_remote_entries(session, sftp_session, path).await },
        move |result| Message::SftpRemoteLoaded(tab_index, result),
    ))
}

async fn load_remote_entries(
    session: crate::core::session::Session,
    sftp_session: Arc<Mutex<Option<russh_sftp::client::SftpSession>>>,
    path: String,
) -> Result<Vec<SftpEntry>, String> {
    use chrono::TimeZone;

    let dir_entries = {
        let mut guard = sftp_session.lock().await;
        if guard.is_none() {
            let ssh = match session.backend.as_ref() {
                crate::core::backend::SessionBackend::Ssh { session, .. } => session.clone(),
                _ => return Err("No SSH session".to_string()),
            };
            let mut ssh_guard = ssh.lock().await;
            let created = ssh_guard
                .open_sftp()
                .await
                .map_err(|e| format!("SFTP init failed: {}", e))?;
            *guard = Some(created);
        }
        let sftp = guard.as_ref().ok_or_else(|| "SFTP not available".to_string())?;
        sftp.read_dir(path)
            .await
            .map_err(|e| format!("Failed to read remote dir: {}", e))?
    };

    let mut entries = Vec::new();
    for entry in dir_entries {
        let meta = entry.metadata();
        let is_dir = meta.is_dir();
        if entry.file_name().starts_with('.') {
            continue;
        }
        let size = if is_dir { None } else { meta.size };
        let modified = meta
            .mtime
            .and_then(|t| chrono::Local.timestamp_opt(t as i64, 0).single());
        entries.push(SftpEntry {
            name: entry.file_name(),
            size,
            modified,
            is_dir,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

fn normalize_remote_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "~" {
        ".".to_string()
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        format!("./{}", rest)
    } else {
        trimmed.to_string()
    }
}
