mod style;
mod terminal_widget;

use iced::{Alignment, Element, Length, Settings, Task, Theme};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::SessionManager;
use crate::platform::PlatformServices;
use crate::session::{SessionConfig, SessionStorage};
use crate::terminal::TerminalEmulator;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::Read;
use style as ui_style;

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    Terminal,
    SessionManager,
}

#[derive(Debug, Clone)]
pub enum Message {
    // CreateSession, // Removed unused
    CreateLocalTab,
    SelectTab(usize),
    CloseTab(usize),
    ToggleMenu,
    // Menu actions
    ShowSessionManager,
    ShowSftp,
    ShowPortForwarding,
    ShowSettings,
    // Session management
    CreateNewSession,
    EditSession(String),
    DeleteSession(String),
    ConnectToSession(String),
    SaveSession,
    CancelSessionEdit,
    CloseSessionManager,
    ToggleAuthMethod,
    #[allow(dead_code)]
    ClearValidationError,
    // Session form fields
    SessionNameChanged(String),
    SessionHostChanged(String),
    SessionPortChanged(String),
    SessionUsernameChanged(String),
    SessionPasswordChanged(String),
    // SSH Connection
    SessionConnected(
        Result<
            (
                Arc<Mutex<crate::ssh::SshSession>>,
                Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>,
            ),
            String,
        >,
        usize,
    ),
    ShellOpened(Result<russh::ChannelId, String>, usize),
    TerminalDataReceived(usize, Vec<u8>),
    TerminalInput(Vec<u8>),
    // Terminal Mouse Events
    TerminalMousePress(usize, usize),
    TerminalMouseDrag(usize, usize),
    TerminalMouseRelease,
    TerminalMouseDoubleClick(usize, usize),
    TerminalResize(usize, usize),
    WindowResized(u32, u32),
    ScrollWheel(f32), // delta in lines
    Tick(std::time::Instant),
    RetryConnection(usize),   // tab index to retry
    EditSessionConfig(usize), // tab index to edit
    Copy,
    Paste,
    ClipboardReceived(Option<String>),
}

#[derive(Debug)]
pub struct App {
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
                                                println!("Local: read {} bytes from PTY", n);
                                                if let Err(e) = tx.send(buf[..n].to_vec()) {
                                                    println!(
                                                        "Local: failed to send to channel: {}",
                                                        e
                                                    );
                                                    break;
                                                }
                                            }
                                            Ok(_) => {
                                                println!("Local: read 0 bytes");
                                                break;
                                            }
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
                                tab.session = Some(session);
                                tab.rx = Some(Arc::new(Mutex::new(rx)));

                                self.tabs.push(tab);
                                let tab_index = self.tabs.len() - 1;
                                self.active_tab = tab_index;
                                self.active_view = ActiveView::Terminal;

                                // Start reading loop (same as SSH)
                                // We need access to rx again? no, we put it in tab.
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
                println!("UI: Received {} bytes for tab {}", data.len(), tab_index);

                // Update the specific tab's emulator
                if let Some(tab) = self.tabs.get_mut(tab_index) {
                    for byte in &data {
                        tab.emulator.process_input(*byte);
                    }
                    tab.cache.clear();

                    // Continue reading for this tab
                    if !data.is_empty() {
                        if let Some(rx) = &tab.rx {
                            let rx = rx.clone();
                            return Task::perform(
                                async move {
                                    let mut guard = rx.lock().await;
                                    match guard.recv().await {
                                        Some(data) => (tab_index, data),
                                        None => (tab_index, vec![]),
                                    }
                                },
                                |(idx, data)| Message::TerminalDataReceived(idx, data),
                            );
                        }
                    } else {
                        println!("Stream ended for tab {}", tab_index);
                        tab.state = SessionState::Disconnected;
                    }
                }
            }
            Message::TerminalMousePress(col, line) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_press(col, line);
                    tab.cache.clear();
                }
            }
            Message::TerminalMouseDrag(col, line) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_drag(col, line);
                    tab.cache.clear();
                }
            }
            Message::TerminalMouseRelease => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_release();
                    tab.cache.clear();
                }
            }
            Message::TerminalMouseDoubleClick(col, line) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.on_mouse_double_click(col, line);
                    tab.cache.clear();
                }
            }
            Message::TerminalResize(cols, rows) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.emulator.resize(cols, rows);

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
                        tab.cache.clear();
                    }
                }
            }
            Message::TerminalInput(data) => {
                if !data.is_empty() {
                    println!("UI: Terminal Input: {} bytes", data.len());
                }
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
                                if let Err(e) = session.write(&data_to_send).await {
                                    println!("UI: Write error: {}", e);
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
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let SessionState::Connecting(_) = tab.state {
                        tab.spinner_cache.clear();
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
        }
        Task::batch(commands)
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, row};

        let content = match self.active_view {
            ActiveView::Terminal => self.terminal_view(),
            ActiveView::SessionManager => self.session_manager_view(),
        };

        let mut main_layout = column![];

        if !self.tabs.is_empty() {
            main_layout = main_layout.push(self.global_tab_bar());
        }

        let main_layout = main_layout
            .push(content)
            .push(self.global_status_bar())
            .spacing(0)
            .height(Length::Fill);

        let base_container = container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::app_background);

        if self.show_menu {
            let left_menu = container(self.sidebar_menu())
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
        }
    }

    fn terminal_view(&self) -> Element<'_, Message> {
        use iced::widget::{column, container, row, text};

        if self.tabs.is_empty() {
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

        let (current_tab_cache, current_emulator, current_tab_state, _current_spinner_cache) =
            if let Some(tab) = self.tabs.get(self.active_tab) {
                (
                    &tab.cache,
                    tab.emulator.clone(),
                    &tab.state,
                    &tab.spinner_cache,
                )
            } else {
                // Should be covered by is_empty check, but safe fallback
                (
                    &self.tabs[0].cache,
                    self.tabs[0].emulator.clone(),
                    &self.tabs[0].state,
                    &self.tabs[0].spinner_cache,
                )
            };

        match current_tab_state {
            SessionState::Connecting(start_time) => {
                let _elapsed = start_time.elapsed().as_secs_f32();
                // We'll use a simple container with text for now, or a custom widget
                // Given Iced complexity, let's start with a centered text that says "Connecting..."
                // To animate, we effectively need a Canvas widget.

                // Let's us terminal_widget's new Spinner capability if we add it,
                // OR just use a canvas here.

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
                let current_tab_index = self.active_tab;

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
                    terminal_widget::TerminalView::new(current_emulator.clone(), current_tab_cache)
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

    fn session_manager_view(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, scrollable, text};

        let title_bar = row![
            text("Session Manager").size(20),
            container("").width(Length::Fill),
            button(text("+ New").size(14))
                .padding([8, 16])
                .style(ui_style::new_tab_button)
                .on_press(Message::CreateNewSession),
            button(text("✕").size(16))
                .padding([8, 12])
                .style(ui_style::tab_close_button)
                .on_press(Message::CloseSessionManager),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([12, 16]);

        // Left panel: Session list
        let session_list: Element<Message> = if self.saved_sessions.is_empty() {
            column![
                container("").height(Length::Fixed(60.0)),
                text("No saved sessions")
                    .size(14)
                    .style(ui_style::muted_text),
                container("").height(Length::Fixed(8.0)),
                text("Click '+ New' to create")
                    .size(12)
                    .style(ui_style::muted_text),
            ]
            .align_x(Alignment::Center)
            .into()
        } else {
            iced::widget::responsive(move |size| {
                let card_width = 320.0;
                let spacing = 16.0;
                let padding = 24.0;
                // Calculate columns based on available width
                let cols = ((size.width - padding) / (card_width + spacing))
                    .floor()
                    .max(1.0) as usize;

                let chunks = self.saved_sessions.chunks(cols);
                let mut content = column![].spacing(spacing).padding(12);

                for chunk in chunks {
                    let mut row = row![].spacing(spacing);
                    for session in chunk {
                        row = row.push(self.session_card(session));
                    }
                    content = content.push(row);
                }

                scrollable(content).height(Length::Fill).into()
            })
            .into()
        };

        // Right panel: Form or empty state
        let right_panel = if self.editing_session.is_some() {
            container(
                container(scrollable(self.session_form_content()).height(Length::Fill))
                    .padding(16)
                    .height(Length::Fill)
                    .style(ui_style::panel),
            )
            .width(Length::Fixed(400.0)) // Fixed width for form instead of portion
            .height(Length::Fill)
            .padding(12)
        } else {
            container("")
                .width(Length::Fixed(0.0))
                .height(Length::Fixed(0.0))
        };

        let content = column![
            container(title_bar)
                .width(Length::Fill)
                .style(ui_style::tab_bar),
            row![
                container(session_list)
                    .width(Length::Fill) // Take remaining space
                    .height(Length::Fill),
                right_panel,
            ]
            .height(Length::Fill),
        ]
        .spacing(0);

        // Return content directly (shell adds background)
        content.into()
    }

    fn session_card(&self, session: &SessionConfig) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, text};

        let connection_info = format!("{}@{}:{}", session.username, session.host, session.port);

        let mut card_content = column![
            row![
                text(session.name.clone()).size(16),
                container("").width(Length::Fill),
            ],
            text(connection_info).size(13).style(ui_style::muted_text),
        ]
        .spacing(4);

        // Only show last connected if it exists
        if let Some(dt) = session.last_connected {
            card_content = card_content.push(container("").height(4.0)).push(
                text(format!("Last connected: {}", dt.format("%Y-%m-%d %H:%M")))
                    .size(12)
                    .style(ui_style::muted_text),
            );
        }

        card_content = card_content.push(container("").height(8.0)).push(
            row![
                button(text("Connect").size(13))
                    .padding([6, 16])
                    .style(ui_style::new_tab_button)
                    .on_press(Message::ConnectToSession(session.id.clone())),
                button(text("Edit").size(13))
                    .padding([6, 16])
                    .style(ui_style::menu_button(false))
                    .on_press(Message::EditSession(session.id.clone())),
                button(text("Delete").size(13))
                    .padding([6, 16])
                    .style(ui_style::tab_close_button)
                    .on_press(Message::DeleteSession(session.id.clone())),
            ]
            .spacing(8),
        );

        let final_card = card_content.padding(12);

        container(final_card)
            .width(Length::Fixed(360.0))
            .style(ui_style::panel)
            .into()
    }

    fn session_form_content(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, text, text_input};

        let is_new = self
            .editing_session
            .as_ref()
            .map(|s| !self.saved_sessions.iter().any(|saved| saved.id == s.id))
            .unwrap_or(false);

        let title = if is_new {
            "New Session"
        } else {
            "Edit Session"
        };

        let form_header = row![
            text(title).size(15),
            container("").width(Length::Fill),
            button(text("Save").size(12))
                .padding([5, 10])
                .style(ui_style::new_tab_button)
                .on_press(Message::SaveSession),
            button(text("Cancel").size(12))
                .padding([5, 10])
                .style(ui_style::tab_close_button)
                .on_press(Message::CancelSessionEdit),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding(iced::Padding::default().bottom(10));

        let error_banner = if let Some(ref error) = self.validation_error {
            container(
                text(format!("⚠️ {}", error))
                    .size(12)
                    .color(iced::Color::from_rgb(0.8, 0.2, 0.2)),
            )
            .padding(10)
            .width(Length::Fill)
            .style(ui_style::panel)
        } else {
            container("")
        };

        column![
            form_header,
            error_banner,
            container("").height(8.0),
            text("Name").size(11).style(ui_style::muted_text),
            text_input("Production Server", &self.form_name)
                .on_input(Message::SessionNameChanged)
                .padding(8)
                .size(12),
            container("").height(8.0),
            text("Host").size(11).style(ui_style::muted_text),
            text_input("example.com", &self.form_host)
                .on_input(Message::SessionHostChanged)
                .padding(8)
                .size(12),
            container("").height(8.0),
            row![
                column![
                    text("Port").size(11).style(ui_style::muted_text),
                    text_input("22", &self.form_port)
                        .on_input(Message::SessionPortChanged)
                        .padding(8)
                        .size(12)
                        .width(Length::Fixed(80.0)),
                ]
                .spacing(3),
                container("").width(Length::Fixed(12.0)),
                column![
                    text("Username").size(11).style(ui_style::muted_text),
                    text_input("user", &self.form_username)
                        .on_input(Message::SessionUsernameChanged)
                        .padding(8)
                        .size(12)
                        .width(Length::Fill),
                ]
                .spacing(3)
                .width(Length::Fill),
            ],
            container("").height(8.0),
            text("Authentication").size(11).style(ui_style::muted_text),
            row![
                button(text("🔑 Private Key").size(11))
                    .padding([6, 12])
                    .style(move |theme, status| {
                        if !self.auth_method_password {
                            ui_style::new_tab_button(theme, status)
                        } else {
                            (ui_style::menu_button(false))(theme, status)
                        }
                    })
                    .on_press(if self.auth_method_password {
                        Message::ToggleAuthMethod
                    } else {
                        Message::ToggleAuthMethod // dummy, won't toggle if already selected
                    }),
                button(text("🔒 Password").size(11))
                    .padding([6, 12])
                    .style(move |theme, status| {
                        if self.auth_method_password {
                            ui_style::new_tab_button(theme, status)
                        } else {
                            (ui_style::menu_button(false))(theme, status)
                        }
                    })
                    .on_press(if !self.auth_method_password {
                        Message::ToggleAuthMethod
                    } else {
                        Message::ToggleAuthMethod // dummy
                    }),
            ]
            .spacing(6),
            container("").height(8.0),
            if !self.auth_method_password {
                column![
                    text("Private Key Path")
                        .size(11)
                        .style(ui_style::muted_text),
                    text_input("~/.ssh/id_rsa", "~/.ssh/id_rsa")
                        .padding(8)
                        .size(12),
                ]
                .spacing(3)
            } else {
                column![
                    text("Password").size(11).style(ui_style::muted_text),
                    text_input("", &self.form_password)
                        .on_input(Message::SessionPasswordChanged)
                        .padding(8)
                        .size(12)
                        .secure(true),
                ]
                .spacing(3)
            },
        ]
        .spacing(3)
        .into()
    }

    fn sidebar_menu(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, text};

        column![
            text("MENU").size(12).style(ui_style::muted_text),
            container("").width(Length::Fill).height(8.0),
            button(
                row![text("📂").size(18), text("Sessions").size(15),]
                    .spacing(8)
                    .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .padding([10, 14])
            .style(ui_style::menu_item)
            .on_press(Message::ShowSessionManager),
            button(
                row![text("📁").size(18), text("SFTP").size(15),]
                    .spacing(8)
                    .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .padding([10, 14])
            .style(ui_style::menu_item)
            .on_press(Message::ShowSftp),
            button(
                row![text("🔀").size(18), text("Forwarding").size(15),]
                    .spacing(8)
                    .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .padding([10, 14])
            .style(ui_style::menu_item)
            .on_press(Message::ShowPortForwarding),
            container("")
                .width(Length::Fill)
                .height(1.0)
                .style(ui_style::menu_divider),
            button(
                row![text("⚙️").size(18), text("Settings").size(15),]
                    .spacing(8)
                    .align_y(Alignment::Center)
            )
            .width(Length::Fill)
            .padding([8, 12])
            .style(ui_style::menu_item)
            .on_press(Message::ShowSettings),
        ]
        .spacing(2)
        .into()
    }

    fn global_status_bar(&self) -> Element<'_, Message> {
        use iced::widget::{button, container, row, text};

        let current_tab = self.tabs.get(self.active_tab);
        let status_left = if let Some(tab) = current_tab {
            if self.active_view == ActiveView::Terminal {
                format!("{}  ● Connected 120ms", tab.title)
            } else {
                "Session Manager".to_string()
            }
        } else {
            if self.active_view == ActiveView::SessionManager {
                "Session Manager".to_string()
            } else {
                "No active session".to_string()
            }
        };

        let status_bar = row![
            button(text("≡").size(20))
                .padding([4, 8])
                .style(ui_style::menu_button(self.show_menu))
                .on_press(Message::ToggleMenu),
            text("│").size(12).style(ui_style::muted_text),
            text(status_left).size(12),
            container("").width(Length::Fill),
            text("UTF-8").size(12).style(ui_style::muted_text),
            text("│").size(12).style(ui_style::muted_text),
            text("24x120").size(12).style(ui_style::muted_text),
            text("│").size(12).style(ui_style::muted_text),
            text("↑ 3.2MB/s").size(12).style(ui_style::muted_text),
        ]
        .align_y(Alignment::Center)
        .spacing(8);

        container(status_bar)
            .width(Length::Fill)
            .padding([6, 12])
            .style(ui_style::status_bar)
            .into()
    }

    fn global_tab_bar(&self) -> Element<'_, Message> {
        use iced::widget::{button, container, row, text};

        let mut tabs_row =
            self.tabs
                .iter()
                .enumerate()
                .fold(row![].spacing(4), |row, (index, tab)| {
                    let is_active =
                        index == self.active_tab && self.active_view == ActiveView::Terminal;

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
        if self.active_view != ActiveView::SessionManager {
            tabs_row = tabs_row
                .push(
                    button(text("+ SSH").size(14))
                        .padding([6, 12])
                        .style(ui_style::new_tab_button)
                        .on_press(Message::CreateNewSession),
                )
                .push(
                    button(text("+ Local").size(14))
                        .padding([6, 12])
                        .style(ui_style::new_tab_button)
                        .on_press(Message::CreateLocalTab),
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

        if self.active_view == ActiveView::Terminal {
            let keyboard_subscription = event::listen().map(|event| {
                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                        println!("UI: KeyPressed: {:?} modifiers: {:?}", key, modifiers);
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

        iced::Subscription::batch(subs)
    }
}

use crate::core::session::Session;
use iced::widget::canvas::Cache;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Connecting(std::time::Instant), // Instant for animation start time
    Connected,
    Disconnected,
    Failed(String),
}

#[derive(Debug)]
pub struct SessionTab {
    pub title: String,
    pub cache: Cache,
    pub state: SessionState,
    pub spinner_cache: Cache, // Cache for spinner drawing
    // Session (abstracted)
    pub session: Option<Session>,
    // Temporary storage for SSH handle before shell is opened
    pub ssh_handle: Option<Arc<Mutex<crate::ssh::SshSession>>>,
    pub rx: Option<Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>>,
    pub emulator: TerminalEmulator,
}

impl Clone for SessionTab {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            cache: iced::widget::canvas::Cache::new(),
            state: self.state.clone(),
            spinner_cache: iced::widget::canvas::Cache::new(),
            session: self.session.clone(),
            ssh_handle: self.ssh_handle.clone(),
            rx: self.rx.clone(),
            emulator: self.emulator.clone(),
        }
    }
}

impl SessionTab {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            cache: Cache::default(),
            state: SessionState::Connecting(std::time::Instant::now()),
            spinner_cache: Cache::default(),
            session: None,
            ssh_handle: None,
            rx: None,
            emulator: TerminalEmulator::new(),
        }
    }
}

// Simple Spinner definition
struct Spinner {
    start: std::time::Instant,
}

impl Spinner {
    fn new(start: std::time::Instant) -> Self {
        Self { start }
    }
}

impl<Message> iced::widget::canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());

        let center = frame.center();
        let radius = bounds.width.min(bounds.height) / 2.0;
        let time = self.start.elapsed().as_secs_f32();

        // Warning: Path::arc is not a direct method, use Path::circle for shadow
        let shadow = iced::widget::canvas::Path::circle(center, radius - 4.0);
        frame.stroke(
            &shadow,
            iced::widget::canvas::Stroke::default()
                .with_color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.1))
                .with_width(4.0),
        );

        let start_angle = time * 5.0;
        let end_angle = start_angle + 1.5; // quarter circle arc

        let arc = iced::widget::canvas::Path::new(|b| {
            b.arc(iced::widget::canvas::path::Arc {
                center,
                radius: radius - 4.0,
                start_angle: iced::Radians(start_angle),
                end_angle: iced::Radians(end_angle),
            });
        });

        frame.stroke(
            &arc,
            iced::widget::canvas::Stroke::default()
                .with_color(iced::Color::from_rgb(0.2, 0.4, 0.8))
                .with_width(4.0),
        );

        vec![frame.into_geometry()]
    }
}
