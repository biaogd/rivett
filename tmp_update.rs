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
                                commands.push(self.focus_terminal_ime());

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
                                                if self.show_menu { 200.0 } else { 0.0 };
                                            let h_padding = 24.0;
                                            let v_padding = 120.0;

                                            let term_w =
                                                (width as f32 - sidebar_width - h_padding).max(0.0);
                                            let term_h = (height as f32 - v_padding).max(0.0);

                                            let cols = (term_w / self.cell_width()) as usize;
                                            let rows = (term_h / self.cell_height()) as usize;

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
                self.session_menu_open = None;
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
                self.session_menu_open = None;
                if let Err(e) = self
                    .session_storage
                    .delete_session(&id, &mut self.saved_sessions)
                {
                    eprintln!("Failed to delete session: {}", e);
                }
            }
            Message::ConnectToSession(id) => {
                self.session_menu_open = None;
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

                    let connect_task = Task::perform(
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
                    return Task::batch(vec![connect_task, self.focus_terminal_ime()]);
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
                commands.push(self.focus_terminal_ime());
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

                let sidebar_width = if self.show_menu { 200.0 } else { 0.0 };
                // Approximate padding/chrome
                let h_padding = 24.0;
                let v_padding = 120.0;

                let term_w = (width as f32 - sidebar_width - h_padding).max(0.0);
                let term_h = (height as f32 - v_padding).max(0.0);

                let cols = (term_w / self.cell_width()) as usize;
                let rows = (term_h / self.cell_height()) as usize;

                self.pending_resize = Some((cols, rows, std::time::Instant::now()));
                return Task::done(Message::TerminalResize(cols, rows));
            }
            Message::WindowOpened(_id) => {}
            Message::WindowClosed(id) => {
                if Some(id) == self.main_window {
                    self.main_window = None;
                    return iced::exit();
                }
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
            Message::ToggleSessionMenu(id) => {
                if self.session_menu_open.as_deref() == Some(id.as_str()) {
                    self.session_menu_open = None;
                } else {
                    self.session_menu_open = Some(id);
                }
            }
            Message::CloseSessionMenu => {
                self.session_menu_open = None;
            }
            Message::ImeBufferChanged(value) => {
                if self.ime_ignore_next_input {
                    self.ime_ignore_next_input = false;
                    self.ime_buffer.clear();
                    return Task::none();
                }

                let prev = self.ime_buffer.clone();
                self.ime_buffer = value.clone();
                if self.active_view != ActiveView::Terminal || self.show_quick_connect {
                    return Task::none();
                }

                if value == prev {
                    return Task::none();
                }

                if value.starts_with(&prev) {
                    let suffix = &value[prev.len()..];
                    if suffix.is_empty() {
                        return Task::none();
                    }
                    return Task::done(Message::TerminalInput(suffix.as_bytes().to_vec()));
                }

                if prev.starts_with(&value) {
                    let removed = prev.chars().count().saturating_sub(value.chars().count());
                    if removed == 0 {
                        return Task::none();
                    }
                    let mut data = Vec::with_capacity(removed);
                    data.extend(std::iter::repeat(0x08u8).take(removed));
                    return Task::done(Message::TerminalInput(data));
                }

                let mut data = Vec::new();
                let remove_count = prev.chars().count();
                data.extend(std::iter::repeat(0x08u8).take(remove_count));
                data.extend(value.as_bytes());
                if data.is_empty() {
                    return Task::none();
                }
                return Task::done(Message::TerminalInput(data));
            }
            Message::ImePaste => {
                self.ime_ignore_next_input = true;
                self.ime_buffer.clear();
                return iced::clipboard::read().map(Message::ClipboardReceived);
            }
            Message::ImeFocusChanged(focused) => {
                self.ime_focused = focused;
                if self.active_view == ActiveView::Terminal && !self.show_quick_connect && !focused
                {
                    commands.push(self.focus_terminal_ime());
                }
            }
            Message::RuntimeEvent(event, window) => {
                if Some(window) == self.main_window {
                    match event {
                        iced::event::Event::Window(iced::window::Event::Focused) => {
                            self.ime_focused = false;
                            self.reload_settings();
                            if self.active_view == ActiveView::Terminal
                                && !self.show_quick_connect
                            {
                                commands.push(self.focus_terminal_ime());
                                commands.push(self.recalc_terminal_size());
                            }
                            return Task::batch(commands);
                        }
                        iced::event::Event::Window(iced::window::Event::Unfocused) => {
                            self.ime_focused = false;
                            return Task::none();
                        }
                        _ => {}
                    }
                }
                if Some(window) != self.main_window
                    || self.active_view != ActiveView::Terminal
                    || self.show_quick_connect
                {
                    return Task::none();
                }

                match event {
                    iced::event::Event::InputMethod(event) => {
                        match event {
                            iced_core::input_method::Event::Opened
                            | iced_core::input_method::Event::Closed
                            | iced_core::input_method::Event::Commit(_) => {
                                self.ime_preedit.clear();
                            }
                            iced_core::input_method::Event::Preedit(content, _) => {
                                self.ime_preedit = content;
                            }
                        }
                        return Task::none();
                    }
                    iced::event::Event::Window(iced::window::Event::Resized(size)) => {
                        return Task::done(Message::WindowResized(
                            size.width as u32,
                            size.height as u32,
                        ));
                    }
                    iced::event::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key,
                        modifiers,
                        text,
                        ..
                    }) => {
                        let message = {
                            if matches!(
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
                                    iced::keyboard::Key::Character(ref c) if c.as_str() == "c" => {
                                        Message::Copy
                                    }
                                    iced::keyboard::Key::Character(ref c) if c.as_str() == "v" => {
                                        if self.ime_focused {
                                            Message::Ignore
                                        } else {
                                            Message::Paste
                                        }
                                    }
                                    _ => Message::Ignore,
                                }
                            } else if modifiers.command()
                                && matches!(key, iced::keyboard::Key::Character(ref c) if c.as_str() == "t")
                            {
                                Message::CreateLocalTab
                            } else {
                                let s = text.as_ref().map(|t| t.as_str()).unwrap_or("");
                                if !s.is_empty() && !s.chars().any(|c| c.is_control()) {
                                    if self.ime_focused {
                                        Message::Ignore
                                    } else {
                                        Message::TerminalInput(s.as_bytes().to_vec())
                                    }
                                } else if matches!(key, iced::keyboard::Key::Character(_))
                                    && !modifiers.control()
                                {
                                    if s.is_empty() || self.ime_focused {
                                        Message::Ignore
                                    } else {
                                        Message::TerminalInput(s.as_bytes().to_vec())
                                    }
                                } else if let Some(data) =
                                    crate::terminal::input::map_key_to_input(key, modifiers)
                                {
                                    Message::TerminalInput(data)
                                } else {
                                    Message::Ignore
                                }
                            }
                        };

                        if matches!(message, Message::Ignore) {
                            return Task::none();
                        }
                        return Task::done(message);
                    }
                    iced::event::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                        let delta_y = match delta {
                            iced::mouse::ScrollDelta::Lines { y, .. } => y,
                            iced::mouse::ScrollDelta::Pixels { y, .. } => y / 20.0,
                        };
                        return Task::done(Message::ScrollWheel(delta_y));
                    }
                    _ => {}
                }
            }
            Message::TerminalInput(data) => {
                if data.is_empty() {
                    return Task::none();
                }
                // Send to Session (if connected)
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let Some(session) = &tab.session {
                        let session = session.clone();
                        let data_to_send = self.maybe_wrap_bracketed_paste(&data);

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
                    self.ime_ignore_next_input = true;
                    self.ime_buffer.clear();
                    return Task::done(Message::TerminalInput(self.bracketed_paste_bytes(&text)));
                }
            }
            Message::Ignore => {}
        }
        Task::batch(commands)
    }

