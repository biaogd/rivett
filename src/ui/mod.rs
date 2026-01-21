mod components;
mod message;
mod state;
mod style;
mod terminal_widget;
mod views;

use iced::{Element, Length, Settings, Task, Theme};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::SessionManager;
use crate::core::session::Session;
use crate::platform::PlatformServices;
use crate::session::{SessionConfig, SessionStorage};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::Read;
use style as ui_style;

// Re-export types from sub-modules
pub use message::{ActiveView, Message};
pub use state::{SessionState, SessionTab};

#[derive(Debug)]
pub struct App {
    #[allow(dead_code)]
    sessions: SessionManager,
    #[allow(dead_code)]
    platform: PlatformServices,
    tabs: Vec<SessionTab>,
    active_tab: usize,
    show_menu: bool,
    // Session management
    active_view: ActiveView,
    saved_sessions: Vec<SessionConfig>,
    session_storage: SessionStorage,
    editing_session: Option<SessionConfig>,
    // Form state
    form_name: String,
    form_host: String,
    form_port: String,
    form_username: String,
    form_password: String,
    auth_method_password: bool,
    validation_error: Option<String>,
    window_width: u32,
    window_height: u32,
    last_error: Option<(String, std::time::Instant)>, // (error message, timestamp)
    // Quick Connect
    show_quick_connect: bool,
    quick_connect_query: String,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let storage = SessionStorage::new();
        let saved_sessions = storage.load_sessions().unwrap_or_else(|e| {
            eprintln!("Failed to load sessions: {}", e);
            Vec::new()
        });

        (
            Self {
                sessions: SessionManager::new(),
                platform: PlatformServices::new(),
                tabs: Vec::new(),
                active_tab: 0,
                show_menu: true,
                active_view: ActiveView::SessionManager,
                saved_sessions,
                session_storage: storage,
                editing_session: None,
                // Form defaults
                form_name: String::new(),
                form_host: String::new(),
                form_port: "22".to_string(),
                form_username: String::new(),
                form_password: String::new(),
                auth_method_password: true,
                validation_error: None,
                window_width: 1024, // Default assumption
                window_height: 768,
                last_error: None,
                show_quick_connect: false,
                quick_connect_query: String::new(),
            },
            Task::none(), // Removed Command::perform(Self::connect_to_localhost(), ...)
        )
    }

    pub fn title(&self) -> String {
        if self.tabs.is_empty() {
            "SSH GUI - No Sessions".to_string()
        } else {
            format!("SSH GUI - {}", self.tabs[self.active_tab].title)
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let mut commands = Vec::new();

        match message {
            Message::CreateLocalTab => {
                self.show_quick_connect = false;
                let system = native_pty_system();
                // Create a generic PTY size
                let size = PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                };

                match system.openpty(size) {
                    Ok(pair) => {
                        let mut cmd = CommandBuilder::new("zsh"); // Default to zsh or use SHELL env
                        cmd.env("TERM", "xterm-256color");
                        cmd.env("LANG", "en_US.UTF-8");
                        cmd.env("LC_ALL", "en_US.UTF-8");
                        // TODO: Use std::env::var("SHELL").unwrap_or("bash".into())

                        match pair.slave.spawn_command(cmd) {
                            Ok(_) => {
                                println!("Local: process spawned");
                                let master = pair.master;
                                let mut reader = master.try_clone_reader().unwrap();

                                // Create generic session
                                let backend = crate::core::backend::SessionBackend::Local {
                                    master: Arc::new(std::sync::Mutex::new(master)),
                                };
                                let session = crate::core::session::Session::new(backend);

                                // Create RX/TX
                                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

                                // Spawn reader task
                                std::thread::spawn(move || {
                                    println!("Local: reader thread started");
                                    let mut buf = [0u8; 1024];
                                    loop {
                                        match reader.read(&mut buf) {
                                            Ok(n) if n > 0 => {
                                                if let Err(e) = tx.send(buf[..n].to_vec()) {
                                                    println!(
                                                        "Local: failed to send to channel: {}",
                                                        e
                                                    );
                                                    break;
                                                }
                                            }
                                            Ok(_) => break,
                                            Err(e) => {
                                                println!("Local: read error: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                    println!("Local: reader thread ended");
                                });

                                let mut tab = SessionTab::new("Local Shell");
                                tab.state = SessionState::Connected;
                                tab.session = Some(session.clone());
                                tab.rx = Some(Arc::new(Mutex::new(rx)));

                                // Get the terminal output receiver for responses like CPR
                                if let Some(mut output_rx) = tab.emulator.take_output_receiver() {
                                    let session_clone = session.clone();
                                    // Spawn task to write terminal responses back to PTY
                                    std::thread::spawn(move || {
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        rt.block_on(async {
                                            while let Some(data) = output_rx.recv().await {
                                                if let Err(e) = session_clone.write(&data).await {
                                                    println!("Failed to write terminal response to PTY: {}", e);
                                                    break;
                                                }
                                            }
                                        });
                                    });
                                }

                                self.tabs.push(tab);
                                let tab_index = self.tabs.len() - 1;
                                self.active_tab = tab_index;
                                self.active_view = ActiveView::Terminal;

                                // Start reading loop (same as SSH)
                                // No change needed for SSH yet, focusing on local shell which explicitly spawns a command.
                                // But we need to start the pumping loop.
                                // We can't access tab.rx easily because it's wrapped.
                                // But we have a clone if we didn't move it?
                                // Or we can retrieve it.

                                // Let's clone rx before moving to tab? No, UnboundedReceiver is not Clone.
                                // Arc<Mutex<...>> IS Clone.
                                if let Some(tab) = self.tabs.get_mut(tab_index) {
                                    if let Some(rx) = &tab.rx {
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
                                        commands.push(read_task);

                                        // Trigger initial resize (Critical for correct size)
                                        let width = self.window_width;
                                        let height = self.window_height;
                                        if width > 0 && height > 0 {
                                            let sidebar_width =
                                                if self.show_menu { 180.0 } else { 0.0 };
                                            let h_padding = 24.0;
                                            let v_padding = 120.0;

                                            let term_w =
                                                (width as f32 - sidebar_width - h_padding).max(0.0);
                                            let term_h = (height as f32 - v_padding).max(0.0);

                                            let cols =
                                                (term_w / terminal_widget::CELL_WIDTH) as usize;
                                            let rows =
                                                (term_h / terminal_widget::CELL_HEIGHT) as usize;

                                            commands.push(Task::done(Message::TerminalResize(
                                                cols, rows,
                                            )));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Failed to spawn shell: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to open PTY: {}", e);
                    }
                }
            }
            // Message::CreateSession => { ... } // Removed
            Message::SelectTab(index) => {
                println!("UI: Selecting tab {}", index);
                if index < self.tabs.len() {
                    self.active_tab = index;
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
                    let sidebar_width = if self.show_menu { 180.0 } else { 0.0 };
                    let h_padding = 24.0;
                    let v_padding = 120.0;

                    let term_w = (width as f32 - sidebar_width - h_padding).max(0.0);
                    let term_h = (height as f32 - v_padding).max(0.0);

                    let cols = (term_w / terminal_widget::CELL_WIDTH) as usize;
                    let rows = (term_h / terminal_widget::CELL_HEIGHT) as usize;

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
                self.show_menu = false;
                // TODO: Show settings panel
            }
            Message::CreateNewSession => {
                self.editing_session = Some(SessionConfig::new(
                    String::new(),
                    String::new(),
                    22,
                    String::new(),
                ));
                self.form_name.clear();
                self.form_host.clear();
                self.form_port = String::from("22");
                self.form_username.clear();
                self.form_password.clear();
                self.auth_method_password = false;
                self.validation_error = None;
            }
            Message::EditSession(id) => {
                if let Some(session) = self.saved_sessions.iter().find(|s| s.id == id).cloned() {
                    self.form_name = session.name.clone();
                    self.form_host = session.host.clone();
                    self.form_port = session.port.to_string();
                    self.form_username = session.username.clone();
                    if let Some(pass) = &session.password {
                        self.form_password = pass.clone();
                        self.auth_method_password = true;
                    } else {
                        self.form_password.clear();
                        self.auth_method_password = false;
                    }
                    if let crate::session::config::AuthMethod::Password = session.auth_method {
                        self.auth_method_password = true;
                    }
                    self.editing_session = Some(session);
                    self.validation_error = None;
                }
            }
            Message::DeleteSession(id) => {
                if let Err(e) = self
                    .session_storage
                    .delete_session(&id, &mut self.saved_sessions)
                {
                    eprintln!("Failed to delete session: {}", e);
                }
            }
            Message::ConnectToSession(id) => {
                if let Some(session) = self.saved_sessions.iter().find(|s| s.id == id) {
                    let name = session.name.clone();
                    let host = session.host.clone();
                    let port = session.port;
                    let username = session.username.clone();
                    let password = session.password.clone().unwrap_or_default();
                    println!(
                        "Connecting to {}:{} with user '{}' and password '{}'",
                        host, port, username, password
                    );

                    self.tabs.push(SessionTab::new(&name));
                    self.active_tab = self.tabs.len() - 1;
                    self.active_view = ActiveView::Terminal; // Switch to terminal view immediately
                    let tab_index = self.active_tab;

                    return Task::perform(
                        async move {
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(10),
                                crate::ssh::SshSession::connect(&host, port, &username, &password),
                            )
                            .await
                            {
                                Ok(Ok((session, rx))) => {
                                    Ok((Arc::new(Mutex::new(session)), Arc::new(Mutex::new(rx))))
                                }
                                Ok(Err(e)) => Err(e.to_string()),
                                Err(_) => Err("Connection timeout (10s)".to_string()),
                            }
                        },
                        move |result| Message::SessionConnected(result, tab_index),
                    );
                }
            }
            Message::SaveSession => {
                if let Some(ref mut session) = self.editing_session {
                    // Validation
                    if self.form_name.trim().is_empty() {
                        self.validation_error = Some("Session name is required".to_string());
                        return Task::none();
                    }

                    if self.form_host.trim().is_empty() {
                        self.validation_error = Some("Host is required".to_string());
                        return Task::none();
                    }

                    if self.form_username.trim().is_empty() {
                        self.validation_error = Some("Username is required".to_string());
                        return Task::none();
                    }

                    let port = match self.form_port.parse::<u16>() {
                        Ok(p) if p > 0 => p,
                        _ => {
                            self.validation_error =
                                Some("Port must be a number between 1 and 65535".to_string());
                            return Task::none();
                        }
                    };

                    // Validate password if using password authentication
                    if self.auth_method_password && self.form_password.trim().is_empty() {
                        self.validation_error =
                            Some("Password is required for password authentication".to_string());
                        return Task::none();
                    }

                    session.name = self.form_name.clone();
                    session.host = self.form_host.clone();
                    session.port = port;
                    session.username = self.form_username.clone();

                    // Save auth method and password
                    if self.auth_method_password {
                        session.auth_method = crate::session::config::AuthMethod::Password;
                        session.password = Some(self.form_password.clone());
                    } else {
                        session.auth_method = crate::session::config::AuthMethod::PrivateKey {
                            path: "~/.ssh/id_rsa".to_string(),
                        };
                        session.password = None;
                    }

                    if let Err(e) = self
                        .session_storage
                        .save_session(session.clone(), &mut self.saved_sessions)
                    {
                        self.validation_error = Some(format!("Failed to save: {}", e));
                        return Task::none();
                    }

                    self.editing_session = None;
                    self.validation_error = None;
                }
            }
            Message::CancelSessionEdit => {
                self.editing_session = None;
                self.validation_error = None;
            }
            Message::CloseSessionManager => {
                self.active_view = ActiveView::Terminal;
            }
            Message::ToggleAuthMethod => {
                self.auth_method_password = !self.auth_method_password;
                self.validation_error = None;
            }
            Message::ClearValidationError => {
                self.validation_error = None;
            }
            Message::SessionNameChanged(value) => {
                self.form_name = value;
                self.validation_error = None;
            }
            Message::SessionHostChanged(value) => {
                self.form_host = value;
                self.validation_error = None;
            }
            Message::SessionPortChanged(value) => {
                if value.chars().all(|c| c.is_numeric()) {
                    self.form_port = value;
                    self.validation_error = None;
                }
            }
            Message::SessionUsernameChanged(value) => {
                self.form_username = value;
                self.validation_error = None;
            }
            Message::SessionPasswordChanged(value) => {
                self.form_password = value;
                self.validation_error = None;
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
                            let sidebar_width = if self.show_menu { 180.0 } else { 0.0 };
                            let h_padding = 24.0;
                            let v_padding = 120.0;

                            let term_w = (width as f32 - sidebar_width - h_padding).max(0.0);
                            let term_h = (height as f32 - v_padding).max(0.0);

                            let cols = (term_w / terminal_widget::CELL_WIDTH) as usize;
                            let rows = (term_h / terminal_widget::CELL_HEIGHT) as usize;

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
                if let Some(tab) = self.tabs.get_mut(tab_index) {
                    if data.is_empty() {
                        tab.state = SessionState::Disconnected;
                        return Task::none();
                    }

                    if let Some(tx) = &tab.parser_tx {
                        let _ = tx.send(data);
                    } else {
                        tab.emulator.process_input(&data);
                        tab.mark_full_damage();
                    }
                }
            }
            Message::TerminalDamaged(tab_index, damage) => {
                if let Some(tab) = self.tabs.get_mut(tab_index) {
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
            }
            Message::TerminalMousePress(col, line) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_press(col, line);
                    tab.mark_full_damage();
                }
            }
            Message::TerminalMouseDrag(col, line) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_drag(col, line);
                    tab.mark_full_damage();
                }
            }
            Message::TerminalMouseRelease => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_release();
                    tab.mark_full_damage();
                }
            }
            Message::TerminalMouseDoubleClick(col, line) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_double_click(col, line);
                    tab.mark_full_damage();
                }
            }
            Message::TerminalResize(cols, rows) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.resize(cols, rows);
                    tab.ensure_line_caches(rows);
                    tab.mark_full_damage();

                    // Resize session (SSH or Local)
                    if let Some(session) = &tab.session {
                        let session = session.clone();
                        return Task::perform(
                            async move {
                                // We ignore error for now
                                let _ = session.resize(cols as u16, rows as u16).await;
                            },
                            |_| Message::TerminalInput(vec![]),
                        );
                    }
                }
            }
            Message::WindowResized(width, height) => {
                self.window_width = width;
                self.window_height = height;

                let sidebar_width = if self.show_menu { 180.0 } else { 0.0 };
                // Approximate padding/chrome
                let h_padding = 24.0;
                let v_padding = 120.0;

                let term_w = (width as f32 - sidebar_width - h_padding).max(0.0);
                let term_h = (height as f32 - v_padding).max(0.0);

                let cols = (term_w / terminal_widget::CELL_WIDTH) as usize;
                let rows = (term_h / terminal_widget::CELL_HEIGHT) as usize;

                return Task::done(Message::TerminalResize(cols, rows));
            }
            Message::ScrollWheel(delta) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if delta.abs() > 0.001 {
                        let clamped_delta = delta.clamp(-100.0, 100.0);
                        tab.emulator.scroll(clamped_delta);
                        tab.mark_full_damage();
                    }
                }
            }
            Message::ToggleQuickConnect => {
                self.show_quick_connect = !self.show_quick_connect;
                if self.show_quick_connect {
                    self.quick_connect_query = String::new(); // Reset query on open
                }
            }
            Message::QuickConnectQueryChanged(query) => {
                self.quick_connect_query = query;
            }
            Message::SelectQuickConnectSession(name) => {
                self.show_quick_connect = false;
                return Task::perform(async move { name }, Message::ConnectToSession);
            }
            Message::TerminalInput(data) => {
                if data.is_empty() {
                    return Task::none();
                }
                // Send to Session (if connected)
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let Some(session) = &tab.session {
                        let session = session.clone();
                        let data_to_send = data.clone();

                        return Task::perform(
                            async move {
                                let write_future = session.write(&data_to_send);
                                match tokio::time::timeout(
                                    std::time::Duration::from_millis(2000),
                                    write_future,
                                )
                                .await
                                {
                                    Ok(Ok(_)) => {}
                                    Ok(Err(e)) => println!("UI: Write error: {}", e),
                                    Err(_) => println!("UI: Write timeout - session unresponsive"),
                                }
                            },
                            |_| Message::TerminalInput(vec![]),
                        );
                    } else {
                        println!("UI: Tab {} ignoring input (no session)", self.active_tab);
                    }
                } else {
                    println!("UI: Tab {} ignoring input (invalid index)", self.active_tab);
                }
            }
            Message::Tick(_now) => {
                // Spinner animation
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let SessionState::Connecting(_) = tab.state {
                        tab.spinner_cache.clear();
                    }
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
            Message::Copy => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(content) = tab.emulator.copy_selection() {
                        return iced::clipboard::write(content);
                    }
                }
            }
            Message::Paste => {
                return iced::clipboard::read().map(Message::ClipboardReceived);
            }
            Message::ClipboardReceived(content) => {
                if let Some(text) = content {
                    return Task::done(Message::TerminalInput(text.as_bytes().to_vec()));
                }
            }
            Message::Ignore => {}
        }
        Task::batch(commands)
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::container::transparent;
        use iced::widget::{Space, button, column, container, row, stack};

        let content = match self.active_view {
            ActiveView::Terminal => views::terminal::render(&self.tabs, self.active_tab),
            ActiveView::SessionManager => views::session_manager::render(
                &self.saved_sessions,
                self.editing_session.as_ref(),
                &self.form_name,
                &self.form_host,
                &self.form_port,
                &self.form_username,
                &self.form_password,
                self.auth_method_password,
                self.validation_error.as_ref(),
            ),
        };

        // Build layout from top to bottom: tab_bar (if terminal) -> content -> status_bar
        let mut main_layout = column![];

        // Tab bar at the top (only in terminal view)
        if self.active_view == ActiveView::Terminal {
            main_layout = main_layout.push(views::tab_bar::render(
                &self.tabs,
                self.active_tab,
                self.active_view,
            ));
        }

        // Main content
        main_layout = main_layout.push(content);

        // Status bar at the bottom
        main_layout = main_layout.push(views::status_bar::render(
            &self.tabs,
            self.active_tab,
            self.active_view,
            self.show_menu,
        ));

        let base_container = container(main_layout.spacing(0).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::app_background);

        let main_view: Element<'_, Message> = if self.show_menu {
            let left_menu = container(views::sidebar::render(self.active_view))
                .width(Length::Fixed(180.0))
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
                self.auth_method_password,
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

    pub fn run(_settings: Settings) -> iced::Result {
        iced::application(App::new, App::update, App::view)
            .title(App::title)
            .theme(|_: &App| Theme::Light)
            .subscription(App::subscription)
            .run()
    }

    // Old subscription removed

    // Add separate timer subscription method if needed, or combine:
    fn subscription(&self) -> iced::Subscription<Message> {
        use iced::event::{self, Event};
        use iced::keyboard;

        let mut subs = Vec::new();

        // Add Tick subscription for render throttling (approx 60 FPS check rate)
        subs.push(iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick));

        if self.active_view == ActiveView::Terminal {
            let keyboard_subscription = event::listen().map(|event| {
                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                        // Handle Cmd+C / Cmd+V
                        if modifiers.command() {
                            match key {
                                keyboard::Key::Character(c) if c.as_str() == "c" => {
                                    return Message::Copy;
                                }
                                keyboard::Key::Character(c) if c.as_str() == "v" => {
                                    return Message::Paste;
                                }
                                _ => {}
                            }
                        }

                        if let Some(data) = crate::terminal::input::map_key_to_input(key, modifiers)
                        {
                            Message::TerminalInput(data)
                        } else {
                            Message::TerminalInput(vec![]) // NoOp
                        }
                    }
                    Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                        use iced::mouse::ScrollDelta;
                        let scroll_lines = match delta {
                            ScrollDelta::Lines { y, .. } => y * 3.0,
                            ScrollDelta::Pixels { y, .. } => y / 16.0,
                        };
                        Message::ScrollWheel(scroll_lines)
                    }
                    Event::Window(iced::window::Event::Resized(size)) => {
                        Message::WindowResized(size.width as u32, size.height as u32)
                    }
                    _ => Message::TerminalInput(vec![]),
                }
            });
            subs.push(keyboard_subscription);
        }

        // Ticking subscription if any tab is connecting
        let any_connecting = self
            .tabs
            .iter()
            .any(|tab| matches!(tab.state, SessionState::Connecting(_)));
        if any_connecting {
            subs.push(iced::time::every(std::time::Duration::from_millis(50)).map(Message::Tick));
        }

        // Hashable wrapper for Rx
        struct HashableRx(
            Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>,
            usize,
        );

        impl std::hash::Hash for HashableRx {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                (Arc::as_ptr(&self.0) as usize).hash(state);
                self.1.hash(state);
            }
        }
        impl PartialEq for HashableRx {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.0, &other.0) && self.1 == other.1
            }
        }
        impl Eq for HashableRx {}
        impl Clone for HashableRx {
            fn clone(&self) -> Self {
                Self(self.0.clone(), self.1)
            }
        }

        // Add PTY subscriptions
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(rx) = &tab.rx {
                let rx = rx.clone();

                subs.push(iced::Subscription::run_with(
                    HashableRx(rx, i),
                    |HashableRx(rx, idx)| {
                        let rx = rx.clone();
                        let idx = *idx;
                        iced::futures::stream::unfold(rx, move |rx| async move {
                            let result = {
                                let mut guard: tokio::sync::MutexGuard<
                                    '_,
                                    tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
                                > = rx.lock().await;

                                if let Some(first_chunk) = guard.recv().await {
                                    let mut batch = first_chunk;
                                    let mut count = 0;
                                    // Drain up to 100 pending chunks to batch them
                                    while count < 100 {
                                        match guard.try_recv() {
                                            Ok(chunk) => {
                                                batch.extend(chunk);
                                                count += 1;
                                            }
                                            Err(_) => break,
                                        }
                                    }
                                    Some(batch)
                                } else {
                                    None
                                }
                            };

                            match result {
                                Some(data) => Some((Message::TerminalDataReceived(idx, data), rx)),
                                None => {
                                    std::future::pending::<()>().await;
                                    None
                                }
                            }
                        })
                    },
                ));
            }
        }

        // Add damage subscriptions
        struct HashableDamageRx(
            Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<crate::terminal::TerminalDamage>>>,
            usize,
        );

        impl std::hash::Hash for HashableDamageRx {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                (Arc::as_ptr(&self.0) as usize).hash(state);
                self.1.hash(state);
            }
        }
        impl PartialEq for HashableDamageRx {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.0, &other.0) && self.1 == other.1
            }
        }
        impl Eq for HashableDamageRx {}
        impl Clone for HashableDamageRx {
            fn clone(&self) -> Self {
                Self(self.0.clone(), self.1)
            }
        }

        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(rx) = &tab.damage_rx {
                let rx = rx.clone();
                subs.push(iced::Subscription::run_with(
                    HashableDamageRx(rx, i),
                    |HashableDamageRx(rx, idx)| {
                        let rx = rx.clone();
                        let idx = *idx;
                        iced::futures::stream::unfold(rx, move |rx| async move {
                            let result = {
                                let mut guard: tokio::sync::MutexGuard<
                                    '_,
                                    tokio::sync::mpsc::UnboundedReceiver<
                                        crate::terminal::TerminalDamage,
                                    >,
                                > = rx.lock().await;
                                guard.recv().await
                            };

                            match result {
                                Some(damage) => {
                                    Some((Message::TerminalDamaged(idx, damage), rx))
                                }
                                None => {
                                    std::future::pending::<()>().await;
                                    None
                                }
                            }
                        })
                    },
                ));
            }
        }

        iced::Subscription::batch(subs)
    }
}
