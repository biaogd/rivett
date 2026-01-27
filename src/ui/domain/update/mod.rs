mod local;
mod sessions;
mod terminal;
mod window;

use iced::Task;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::core::session::Session;
use crate::ui::App;
use crate::ui::message::{ActiveView, Message};
use crate::ui::state::{
    SessionState, SftpContextAction, SftpContextMenu, SftpEntry, SftpPane, SftpTransfer,
    SftpTransferDirection, SftpTransferStatus, SftpTransferUpdate,
};

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
                    let mut active_keys = HashSet::new();
                    for tab in &self.tabs {
                        if let Some(key) = &tab.sftp_key {
                            active_keys.insert(key.clone());
                        }
                    }
                    self.sftp_states.retain(|key, _| active_keys.contains(key));
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
            Message::ShowSessionManager => {
                self.show_quick_connect = false;
                self.active_view = ActiveView::SessionManager;
                self.editing_session = None;
                self.active_tab = 0;
            }
            Message::ToggleSftpPanel => {
                self.sftp_panel_open = !self.sftp_panel_open;
                self.sftp_dragging = false;
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.local_selected = None;
                    state.remote_selected = None;
                    state.local_last_click = None;
                    state.remote_last_click = None;
                    state.context_menu = None;
                    state.panel_cursor = None;
                }
                if self.sftp_panel_open {
                    if self.window_width > 0 {
                        let max_width = (self.window_width as f32 - 240.0).max(320.0);
                        if !self.sftp_panel_initialized {
                            self.sftp_panel_width =
                                (self.window_width as f32 * 0.45).clamp(420.0, 720.0);
                            self.sftp_panel_initialized = true;
                        } else {
                            self.sftp_panel_width = self.sftp_panel_width.clamp(280.0, max_width);
                        }
                    }
                    if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                        let result = load_local_entries(&state.local_path);
                        match result {
                            Ok(entries) => {
                                state.local_entries = entries;
                                state.local_error = None;
                            }
                            Err(err) => {
                                state.local_entries.clear();
                                state.local_error = Some(err);
                            }
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
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.local_path = path;
                    state.local_selected = None;
                    state.local_last_click = None;
                    state.context_menu = None;
                    let result = load_local_entries(&state.local_path);
                    match result {
                        Ok(entries) => {
                            state.local_entries = entries;
                            state.local_error = None;
                        }
                        Err(err) => {
                            state.local_entries.clear();
                            state.local_error = Some(err);
                        }
                    }
                }
            }
            Message::SftpRemotePathChanged(path) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.remote_path = path;
                    state.remote_selected = None;
                    state.remote_last_click = None;
                    state.context_menu = None;
                }
                if let Some(task) = start_remote_list(self, self.active_tab) {
                    return task;
                }
            }
            Message::SftpRemoteLoaded(tab_index, result) => {
                if let Some(state) = self.sftp_state_for_tab_mut(tab_index) {
                    state.remote_loading = false;
                    match result {
                        Ok((entries, resolved_path)) => {
                            state.remote_entries = entries;
                            state.remote_error = None;
                            if let Some(path) = resolved_path {
                                state.remote_path = path;
                            }
                        }
                        Err(err) => {
                            state.remote_entries.clear();
                            state.remote_error = Some(err);
                        }
                    }
                }
            }
            Message::SftpPanelCursorMoved(point) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.panel_cursor = Some(point);
                }
            }
            Message::SftpLocalEntryPressed(name, is_dir) => {
                return handle_local_click(self, name, is_dir);
            }
            Message::SftpRemoteEntryPressed(name, is_dir) => {
                return handle_remote_click(self, name, is_dir);
            }
            Message::SftpFileDragStart(pane, name) => {
                // Also select the item when dragging starts
                let mut tasks = Vec::new();
                match pane {
                    SftpPane::Local => {
                        let is_dir = self
                            .sftp_state_for_tab(self.active_tab)
                            .and_then(|s| s.local_entries.iter().find(|e| e.name == name))
                            .map(|e| e.is_dir)
                            .unwrap_or(false);
                        tasks.push(handle_local_click(self, name.clone(), is_dir));
                    }
                    SftpPane::Remote => {
                        let is_dir = self
                            .sftp_state_for_tab(self.active_tab)
                            .and_then(|s| s.remote_entries.iter().find(|e| e.name == name))
                            .map(|e| e.is_dir)
                            .unwrap_or(false);
                        tasks.push(handle_remote_click(self, name.clone(), is_dir));
                    }
                }
                self.sftp_file_dragging = Some((pane, name));
                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }
            Message::SftpFileDragUpdate(position) => {
                self.sftp_drag_position = Some(position);
            }
            Message::SftpFileDragEnd => {
                if let Some((source_pane, name)) = self.sftp_file_dragging.take() {
                    let cursor_x = self.sftp_drag_position.map(|p| p.x).unwrap_or(0.0);
                    let window_width = self.window_width as f32;
                    let panel_width = self.sftp_panel_width;

                    // Logic to determine drop target
                    // SFTP Panel Right = window_width
                    // SFTP Panel Left = window_width - panel_width
                    // Content Left = SFTP Panel Left + 10 (handle) + 12 (padding)
                    // Content Width = panel_width - 10 - 24
                    // Split X = Content Left + Content Width / 2.0

                    let content_left = window_width - panel_width + 22.0;
                    let content_width = panel_width - 34.0;
                    let split_x = content_left + content_width / 2.0;

                    let target_pane = if cursor_x < split_x {
                        SftpPane::Local
                    } else {
                        SftpPane::Remote
                    };

                    if source_pane != target_pane {
                        match (source_pane, target_pane) {
                            (SftpPane::Local, SftpPane::Remote) => {
                                if let Some(task) = start_upload(self, name) {
                                    return task;
                                }
                            }
                            (SftpPane::Remote, SftpPane::Local) => {
                                if let Some(task) = start_download(self, name) {
                                    return task;
                                }
                            }
                            _ => {}
                        }
                    }
                    self.sftp_drag_position = None;
                }
            }
            Message::SftpFileHover(hovered) => {
                self.sftp_hovered_file = hovered;
            }
            Message::SftpOpenContextMenu(pane, name) => {
                let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) else {
                    return Task::none();
                };
                let position = state.panel_cursor.unwrap_or(iced::Point::new(16.0, 16.0));
                if !name.is_empty() {
                    match pane {
                        SftpPane::Local => {
                            state.local_selected = Some(name.clone());
                        }
                        SftpPane::Remote => {
                            state.remote_selected = Some(name.clone());
                        }
                    }
                }
                state.context_menu = Some(SftpContextMenu {
                    pane,
                    name,
                    position,
                });
            }
            Message::SftpCloseContextMenu => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.context_menu = None;
                    if state.rename_target.is_some() {
                        state.rename_target = None;
                        state.rename_value.clear();
                    }
                }
            }
            Message::SftpContextAction(pane, name, action) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.context_menu = None;
                }

                if action == SftpContextAction::Refresh {
                    if let Some(state) = self.sftp_state_for_tab(self.active_tab) {
                        let path = match pane {
                            SftpPane::Local => state.local_path.clone(),
                            SftpPane::Remote => state.remote_path.clone(),
                        };
                        let message = match pane {
                            SftpPane::Local => Message::SftpLocalPathChanged(path),
                            SftpPane::Remote => Message::SftpRemotePathChanged(path),
                        };
                        return Task::done(message);
                    }
                    return Task::none();
                }

                if pane == SftpPane::Local && action == SftpContextAction::Upload {
                    if let Some(task) = start_upload(self, name.clone()) {
                        return task;
                    }
                }
                if pane == SftpPane::Remote && action == SftpContextAction::Download {
                    if let Some(task) = start_download(self, name.clone()) {
                        return task;
                    }
                }
                if action == SftpContextAction::Rename {
                    let is_dir = match pane {
                        SftpPane::Local => self
                            .sftp_state_for_tab(self.active_tab)
                            .and_then(|state| {
                                state
                                    .local_entries
                                    .iter()
                                    .find(|entry| entry.name == name)
                                    .map(|entry| entry.is_dir)
                            })
                            .unwrap_or(false),
                        SftpPane::Remote => self
                            .sftp_state_for_tab(self.active_tab)
                            .and_then(|state| {
                                state
                                    .remote_entries
                                    .iter()
                                    .find(|entry| entry.name == name)
                                    .map(|entry| entry.is_dir)
                            })
                            .unwrap_or(false),
                    };
                    if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                        state.rename_target = Some(crate::ui::state::SftpPendingAction {
                            pane,
                            name: name.clone(),
                            is_dir,
                        });
                        state.rename_value = name.clone();
                    }
                    return iced::widget::operation::focus(self.sftp_rename_input_id.clone());
                }
                if action == SftpContextAction::Delete {
                    let is_dir = match pane {
                        SftpPane::Local => self
                            .sftp_state_for_tab(self.active_tab)
                            .and_then(|state| {
                                state
                                    .local_entries
                                    .iter()
                                    .find(|entry| entry.name == name)
                                    .map(|entry| entry.is_dir)
                            })
                            .unwrap_or(false),
                        SftpPane::Remote => self
                            .sftp_state_for_tab(self.active_tab)
                            .and_then(|state| {
                                state
                                    .remote_entries
                                    .iter()
                                    .find(|entry| entry.name == name)
                                    .map(|entry| entry.is_dir)
                            })
                            .unwrap_or(false),
                    };
                    if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                        state.delete_target =
                            Some(crate::ui::state::SftpPendingAction { pane, name, is_dir });
                    }
                }
            }
            Message::SftpTransferCancel(id) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    if let Some(transfer) = state
                        .transfers
                        .iter_mut()
                        .find(|transfer| transfer.id == id)
                    {
                        transfer.cancel_flag.store(true, Ordering::SeqCst);
                        transfer.pause_flag.store(false, Ordering::SeqCst);
                        transfer.pause_notify.notify_waiters();
                        if matches!(
                            transfer.status,
                            SftpTransferStatus::Queued | SftpTransferStatus::Uploading
                        ) {
                            transfer.status = SftpTransferStatus::Canceled;
                        }
                    }
                    if let Some(task) = schedule_transfer_tasks(self, self.active_tab) {
                        return task;
                    }
                }
            }
            Message::SftpTransferPause(id) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    if let Some(transfer) = state
                        .transfers
                        .iter_mut()
                        .find(|transfer| transfer.id == id)
                    {
                        if transfer.status == SftpTransferStatus::Uploading {
                            transfer.pause_flag.store(true, Ordering::SeqCst);
                            transfer.status = SftpTransferStatus::Paused;
                        }
                    }
                    if let Some(task) = schedule_transfer_tasks(self, self.active_tab) {
                        return task;
                    }
                }
            }
            Message::SftpTransferResume(id) => {
                let max_concurrent = self.sftp_max_concurrent;
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    let active = state
                        .transfers
                        .iter()
                        .filter(|transfer| transfer.status == SftpTransferStatus::Uploading)
                        .count();
                    if active < max_concurrent {
                        if let Some(transfer) = state
                            .transfers
                            .iter_mut()
                            .find(|transfer| transfer.id == id)
                        {
                            if transfer.status == SftpTransferStatus::Paused {
                                transfer.pause_flag.store(false, Ordering::SeqCst);
                                transfer.pause_notify.notify_waiters();
                                transfer.status = SftpTransferStatus::Uploading;
                            }
                        }
                    }
                }
            }
            Message::SftpTransferRetry(id) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    if let Some(transfer) = state
                        .transfers
                        .iter_mut()
                        .find(|transfer| transfer.id == id)
                    {
                        transfer.cancel_flag.store(false, Ordering::SeqCst);
                        transfer.status = SftpTransferStatus::Queued;
                        transfer.bytes_sent = 0;
                        transfer.bytes_total = 0;
                        transfer.started_at = None;
                        transfer.last_update = None;
                        transfer.last_bytes_sent = 0;
                        transfer.last_rate_bps = None;
                        transfer.cancel_flag.store(false, Ordering::SeqCst);
                        transfer.pause_flag.store(false, Ordering::SeqCst);
                    }
                    if let Some(task) = schedule_transfer_tasks(self, self.active_tab) {
                        return task;
                    }
                }
            }
            Message::SftpTransferClearDone => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.transfers.retain(|transfer| {
                        !matches!(
                            transfer.status,
                            SftpTransferStatus::Completed
                                | SftpTransferStatus::Failed(_)
                                | SftpTransferStatus::Canceled
                        )
                    });
                }
            }
            Message::SftpRenameStart(pane, name, is_dir) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.rename_target = Some(crate::ui::state::SftpPendingAction {
                        pane,
                        name: name.clone(),
                        is_dir,
                    });
                    state.rename_value = name;
                }
                return iced::widget::operation::focus(self.sftp_rename_input_id.clone());
            }
            Message::SftpRenameInput(value) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.rename_value = value;
                }
            }
            Message::SftpRenameCancel => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.rename_target = None;
                    state.rename_value.clear();
                }
            }
            Message::SftpRenameConfirm => {
                if let Some(task) = start_rename(self) {
                    return task;
                }
            }
            Message::SftpRenameFinished(tab_index, result) => {
                if let Some(state) = self.sftp_state_for_tab_mut(tab_index) {
                    let target = state.rename_target.clone();
                    state.rename_target = None;
                    state.rename_value.clear();
                    match result {
                        Ok(()) => {
                            if let Some(target) = target {
                                return match target.pane {
                                    SftpPane::Local => Task::done(Message::SftpLocalPathChanged(
                                        state.local_path.clone(),
                                    )),
                                    SftpPane::Remote => {
                                        if let Some(task) = start_remote_list(self, tab_index) {
                                            task
                                        } else {
                                            Task::none()
                                        }
                                    }
                                };
                            }
                        }
                        Err(err) => {
                            state.remote_error = Some(err);
                        }
                    }
                }
            }
            Message::SftpDeleteStart(pane, name, is_dir) => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.delete_target =
                        Some(crate::ui::state::SftpPendingAction { pane, name, is_dir });
                }
            }
            Message::SftpDeleteCancel => {
                if let Some(state) = self.sftp_state_for_tab_mut(self.active_tab) {
                    state.delete_target = None;
                }
            }
            Message::SftpDeleteConfirm => {
                if let Some(task) = start_delete(self) {
                    return task;
                }
            }
            Message::SftpDeleteFinished(tab_index, result) => {
                if let Some(state) = self.sftp_state_for_tab_mut(tab_index) {
                    let target = state.delete_target.clone();
                    state.delete_target = None;
                    match result {
                        Ok(()) => {
                            if let Some(target) = target {
                                return match target.pane {
                                    SftpPane::Local => Task::done(Message::SftpLocalPathChanged(
                                        state.local_path.clone(),
                                    )),
                                    SftpPane::Remote => {
                                        if let Some(task) = start_remote_list(self, tab_index) {
                                            task
                                        } else {
                                            Task::none()
                                        }
                                    }
                                };
                            }
                        }
                        Err(err) => {
                            state.remote_error = Some(err);
                        }
                    }
                }
            }
            Message::SftpTransferUpdate(update) => {
                let status = update.status.clone();
                let mut should_refresh = false;
                let mut error_message: Option<String> = None;
                if let Some(state) = self.sftp_state_for_tab_mut(update.tab_index) {
                    if let Some(transfer) = state
                        .transfers
                        .iter_mut()
                        .find(|transfer| transfer.id == update.id)
                    {
                        transfer.bytes_sent = update.bytes_sent;
                        transfer.bytes_total = update.bytes_total;
                        let now = std::time::Instant::now();
                        if transfer.started_at.is_none() {
                            transfer.started_at = Some(now);
                        }
                        if let Some(last_update) = transfer.last_update {
                            let elapsed = now.duration_since(last_update);
                            if elapsed.as_millis() >= 200 {
                                let delta_bytes =
                                    update.bytes_sent.saturating_sub(transfer.last_bytes_sent);
                                let rate = (delta_bytes as f64 / elapsed.as_secs_f64()) as u64;
                                transfer.last_rate_bps = Some(rate);
                                transfer.last_update = Some(now);
                                transfer.last_bytes_sent = update.bytes_sent;
                            }
                        } else {
                            transfer.last_update = Some(now);
                            transfer.last_bytes_sent = update.bytes_sent;
                        }
                        if let Some(status_value) = status.clone() {
                            transfer.status = status_value;
                        }
                        if matches!(
                            status,
                            Some(
                                SftpTransferStatus::Completed
                                    | SftpTransferStatus::Canceled
                                    | SftpTransferStatus::Paused
                            )
                        ) && transfer.direction == SftpTransferDirection::Upload
                            && update.tab_index == self.active_tab
                            && self.sftp_panel_open
                        {
                            should_refresh = true;
                        }
                        if let Some(SftpTransferStatus::Failed(error)) = status.clone() {
                            error_message = Some(error);
                        }
                    }
                }

                if let Some(message) = error_message {
                    if let Some(state) = self.sftp_state_for_tab_mut(update.tab_index) {
                        state.remote_error = Some(message);
                    }
                }

                let mut tasks = Vec::new();
                if should_refresh {
                    if let Some(task) = start_remote_list(self, self.active_tab) {
                        tasks.push(task);
                    }
                }
                if matches!(
                    status,
                    Some(
                        SftpTransferStatus::Completed
                            | SftpTransferStatus::Failed(_)
                            | SftpTransferStatus::Canceled
                            | SftpTransferStatus::Paused
                    )
                ) {
                    if let Some(task) = schedule_transfer_tasks(self, update.tab_index) {
                        tasks.push(task);
                    }
                }
                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
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
            | Message::SessionKeyIdChanged(_)
            | Message::SessionKeyPassphraseChanged(_)
            | Message::SessionSearchChanged(_)
            | Message::ToggleSavedKeyMenu
            | Message::CloseSavedKeyMenu
            | Message::SessionDialogTabSelected(_)
            | Message::OpenPortForwarding(_)
            | Message::ClosePortForwarding
            | Message::PortForwardLocalPortChanged(_)
            | Message::PortForwardLocalHostChanged(_)
            | Message::PortForwardRemoteHostChanged(_)
            | Message::PortForwardRemotePortChanged(_)
            | Message::AddPortForward
            | Message::TogglePortForward(_)
            | Message::DeletePortForward(_)
            | Message::TestConnection
            | Message::TestConnectionResult(_)
            | Message::ToggleSessionMenu(_)
            | Message::CloseSessionMenu => {
                return sessions::handle(self, message);
            }
            Message::SessionConnected(result, tab_index) => match result {
                Ok((session, rx)) => {
                    if let Some(tab) = self.tabs.get_mut(tab_index) {
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

                        let forward_task = tab
                            .sftp_key
                            .clone()
                            .map(|session_id| sessions::apply_port_forwards(self, &session_id));

                        // Start reading loop
                        let rx_clone = rx.clone();
                        let read_task = Task::perform(
                            async move {
                                let mut guard = rx_clone.lock().await;
                                match guard.recv().await {
                                    Some(data) => {
                                        tracing::debug!(
                                            "recv loop got {} bytes for tab {}",
                                            data.len(),
                                            tab_index
                                        );
                                        (tab_index, data)
                                    }
                                    None => {
                                        tracing::debug!("recv loop closed for tab {}", tab_index);
                                        (tab_index, vec![])
                                    }
                                }
                            },
                            |(idx, data)| Message::TerminalDataReceived(idx, data),
                        );
                        let mut tasks = vec![open_shell_task, read_task];
                        if let Some(task) = forward_task {
                            tasks.push(task);
                        }
                        return Task::batch(tasks);
                    }
                }
                Err(e) => {
                    // Record the error with timestamp
                    self.last_error = Some((e.clone(), std::time::Instant::now()));

                    if let Some(tab) = self.tabs.get_mut(tab_index) {
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
                                                        tracing::warn!("ssh write terminal response failed: {}", e);
                                                        break;
                                                    },
                                                    Err(_) => {
                                                        tracing::warn!("ssh write terminal response timeout - connection might be dead");
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
                            let reserved_width = 0.0;
                            let h_padding = 24.0;
                            let v_padding = 72.0;

                            let term_w = (width as f32 - reserved_width - h_padding).max(0.0);
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
                if let Some(task) =
                    terminal::handle(self, Message::TerminalDataReceived(tab_index, data))
                {
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

    let dir =
        std::fs::read_dir(&target).map_err(|e| format!("Failed to read {}: {}", target, e))?;

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

fn join_local_path(base: &str, name: &str) -> String {
    let expanded = expand_tilde(base);
    let base_path = if expanded.trim().is_empty() {
        expand_tilde("~")
    } else {
        expanded
    };

    std::path::Path::new(&base_path)
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn join_remote_path(base: &str, name: &str) -> String {
    let trimmed = base.trim();
    if trimmed.is_empty() || trimmed == "~" {
        format!("./{}", name)
    } else if trimmed.ends_with('/') {
        format!("{}{}", trimmed, name)
    } else {
        format!("{}/{}", trimmed, name)
    }
}

fn start_remote_list(app: &mut App, tab_index: usize) -> Option<Task<Message>> {
    if tab_index == 0 || tab_index >= app.tabs.len() {
        if let Some(state) = app.sftp_state_for_tab_mut(tab_index) {
            state.remote_entries.clear();
            state.remote_error = Some("No active SSH session".to_string());
            state.remote_loading = false;
        }
        return None;
    }

    let tab = app.tabs.get(tab_index)?;
    let session = match &tab.session {
        Some(session) => session.clone(),
        None => {
            if let Some(state) = app.sftp_state_for_tab_mut(tab_index) {
                state.remote_entries.clear();
                state.remote_error = Some("No active SSH session".to_string());
                state.remote_loading = false;
            }
            return None;
        }
    };

    let sftp_session = tab.sftp_session.clone();
    let path = app
        .sftp_state_for_tab(tab_index)
        .map(|state| normalize_remote_path(&state.remote_path))
        .unwrap_or_else(|| ".".to_string());
    if let Some(state) = app.sftp_state_for_tab_mut(tab_index) {
        state.remote_loading = true;
        state.remote_error = None;
    }
    Some(Task::perform(
        async move { load_remote_entries(session, sftp_session, path).await },
        move |result| Message::SftpRemoteLoaded(tab_index, result),
    ))
}

async fn load_remote_entries(
    session: crate::core::session::Session,
    sftp_session: Arc<Mutex<Option<russh_sftp::client::SftpSession>>>,
    path: String,
) -> Result<(Vec<SftpEntry>, Option<String>), String> {
    use chrono::TimeZone;

    let (dir_entries, resolved_path) = {
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
        let sftp = guard
            .as_ref()
            .ok_or_else(|| "SFTP not available".to_string())?;
        let resolved = if path == "." || path.starts_with("./") {
            sftp.canonicalize(".").await.ok()
        } else {
            None
        };
        let entries = sftp
            .read_dir(path)
            .await
            .map_err(|e| format!("Failed to read remote dir: {}", e))?;
        (entries, resolved)
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

    Ok((entries, resolved_path))
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

fn start_upload(app: &mut App, name: String) -> Option<Task<Message>> {
    let tab_index = app.active_tab;
    if tab_index == 0 || tab_index >= app.tabs.len() {
        if let Some(state) = app.sftp_state_for_tab_mut(tab_index) {
            state.remote_error = Some("No active SSH session".to_string());
        }
        return None;
    }

    let state = app.sftp_state_for_tab_mut(tab_index)?;
    let is_dir = state
        .local_entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.is_dir)
        .unwrap_or(false);

    if is_dir {
        state.remote_error = Some("Directory upload not supported yet".to_string());
        return None;
    }

    let local_path = join_local_path(&state.local_path, &name);
    let remote_path = join_remote_path(&state.remote_path, &name);
    let transfer_id = uuid::Uuid::new_v4();

    state.transfers.push(SftpTransfer {
        id: transfer_id,
        tab_index,
        name: name.clone(),
        direction: SftpTransferDirection::Upload,
        status: SftpTransferStatus::Queued,
        bytes_sent: 0,
        bytes_total: 0,
        local_path: local_path.clone(),
        remote_path: remote_path.clone(),
        started_at: None,
        last_update: None,
        last_bytes_sent: 0,
        last_rate_bps: None,
        cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        pause_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        pause_notify: std::sync::Arc::new(tokio::sync::Notify::new()),
    });
    state.remote_error = None;

    schedule_transfer_tasks(app, tab_index)
}

async fn upload_local_file(
    session: crate::core::session::Session,
    sftp_session: Arc<Mutex<Option<russh_sftp::client::SftpSession>>>,
    local_path: String,
    remote_path: String,
    transfer_id: uuid::Uuid,
    tab_index: usize,
    tx: tokio::sync::mpsc::UnboundedSender<SftpTransferUpdate>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pause_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pause_notify: std::sync::Arc<tokio::sync::Notify>,
) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let send_status = |status| {
        let _ = tx.send(SftpTransferUpdate {
            id: transfer_id,
            tab_index,
            bytes_sent: 0,
            bytes_total: 0,
            status: Some(status),
        });
    };

    let metadata = tokio::fs::metadata(&local_path).await.map_err(|e| {
        let msg = format!("Failed to stat local file: {}", e);
        send_status(SftpTransferStatus::Failed(msg.clone()));
        msg
    })?;
    if metadata.is_dir() {
        let msg = "Directory upload not supported yet".to_string();
        send_status(SftpTransferStatus::Failed(msg.clone()));
        return Err(msg);
    }

    let mut local_file = tokio::fs::File::open(&local_path).await.map_err(|e| {
        let msg = format!("Failed to open local file: {}", e);
        send_status(SftpTransferStatus::Failed(msg.clone()));
        msg
    })?;

    let total = metadata.len();
    let _ = tx.send(SftpTransferUpdate {
        id: transfer_id,
        tab_index,
        bytes_sent: 0,
        bytes_total: total,
        status: Some(SftpTransferStatus::Uploading),
    });

    let mut remote_file = {
        let mut guard = sftp_session.lock().await;
        if guard.is_none() {
            let ssh = match session.backend.as_ref() {
                crate::core::backend::SessionBackend::Ssh { session, .. } => session.clone(),
                _ => return Err("No SSH session".to_string()),
            };
            let mut ssh_guard = ssh.lock().await;
            let created = ssh_guard.open_sftp().await.map_err(|e| {
                let msg = format!("SFTP init failed: {}", e);
                send_status(SftpTransferStatus::Failed(msg.clone()));
                msg
            })?;
            *guard = Some(created);
        }
        let sftp = guard
            .as_ref()
            .ok_or_else(|| "SFTP not available".to_string())?;
        sftp.create(remote_path).await.map_err(|e| {
            let msg = format!("Failed to open remote file: {}", e);
            send_status(SftpTransferStatus::Failed(msg.clone()));
            msg
        })?
    };

    let mut buffer = vec![0u8; 64 * 1024];
    let mut sent: u64 = 0;
    loop {
        while pause_flag.load(Ordering::SeqCst) {
            let _ = tx.send(SftpTransferUpdate {
                id: transfer_id,
                tab_index,
                bytes_sent: sent,
                bytes_total: total,
                status: Some(SftpTransferStatus::Paused),
            });
            pause_notify.notified().await;
        }
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = tx.send(SftpTransferUpdate {
                id: transfer_id,
                tab_index,
                bytes_sent: sent,
                bytes_total: total,
                status: Some(SftpTransferStatus::Canceled),
            });
            return Ok(());
        }
        let read = local_file.read(&mut buffer).await.map_err(|e| {
            let msg = format!("Upload failed: {}", e);
            send_status(SftpTransferStatus::Failed(msg.clone()));
            msg
        })?;
        if read == 0 {
            break;
        }
        remote_file.write_all(&buffer[..read]).await.map_err(|e| {
            let msg = format!("Upload failed: {}", e);
            send_status(SftpTransferStatus::Failed(msg.clone()));
            msg
        })?;
        sent = sent.saturating_add(read as u64);
        let _ = tx.send(SftpTransferUpdate {
            id: transfer_id,
            tab_index,
            bytes_sent: sent,
            bytes_total: total,
            status: None,
        });
    }
    let _ = remote_file.sync_all().await;
    let _ = remote_file.shutdown().await;

    let _ = tx.send(SftpTransferUpdate {
        id: transfer_id,
        tab_index,
        bytes_sent: sent,
        bytes_total: total,
        status: Some(SftpTransferStatus::Completed),
    });

    Ok(())
}

fn start_download(app: &mut App, name: String) -> Option<Task<Message>> {
    let tab_index = app.active_tab;
    if tab_index == 0 || tab_index >= app.tabs.len() {
        if let Some(state) = app.sftp_state_for_tab_mut(tab_index) {
            state.remote_error = Some("No active SSH session".to_string());
        }
        return None;
    }

    let state = app.sftp_state_for_tab_mut(tab_index)?;
    let is_dir = state
        .remote_entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.is_dir)
        .unwrap_or(false);

    if is_dir {
        state.remote_error = Some("Directory download not supported yet".to_string());
        return None;
    }

    let local_path = join_local_path(&state.local_path, &name);
    let remote_path = join_remote_path(&state.remote_path, &name);
    let transfer_id = uuid::Uuid::new_v4();

    state.transfers.push(SftpTransfer {
        id: transfer_id,
        tab_index,
        name: name.clone(),
        direction: SftpTransferDirection::Download,
        status: SftpTransferStatus::Queued,
        bytes_sent: 0,
        bytes_total: 0,
        local_path: local_path.clone(),
        remote_path: remote_path.clone(),
        started_at: None,
        last_update: None,
        last_bytes_sent: 0,
        last_rate_bps: None,
        cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        pause_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        pause_notify: std::sync::Arc::new(tokio::sync::Notify::new()),
    });
    state.remote_error = None;

    schedule_transfer_tasks(app, tab_index)
}

fn start_rename(app: &mut App) -> Option<Task<Message>> {
    let tab_index = app.active_tab;
    let (target, new_name, local_path, remote_path) = {
        let state = app.sftp_state_for_tab_mut(tab_index)?;
        let target = state.rename_target.clone()?;
        let new_name = state.rename_value.trim().to_string();
        if new_name.is_empty() || new_name == target.name {
            state.rename_target = None;
            state.rename_value.clear();
            return None;
        }
        (
            target,
            new_name,
            state.local_path.clone(),
            state.remote_path.clone(),
        )
    };
    match target.pane {
        SftpPane::Local => {
            let old_path = join_local_path(&local_path, &target.name);
            let new_path = join_local_path(&local_path, &new_name);
            Some(Task::perform(
                async move {
                    tokio::fs::rename(old_path, new_path)
                        .await
                        .map_err(|e| format!("Rename failed: {}", e))
                },
                move |result| Message::SftpRenameFinished(tab_index, result),
            ))
        }
        SftpPane::Remote => {
            let tab = app.tabs.get(tab_index)?;
            let session = match &tab.session {
                Some(session) => session.clone(),
                None => return None,
            };
            let sftp_session = tab.sftp_session.clone();
            let old_path = join_remote_path(&remote_path, &target.name);
            let new_path = join_remote_path(&remote_path, &new_name);
            Some(Task::perform(
                async move {
                    let mut guard = sftp_session.lock().await;
                    if guard.is_none() {
                        let ssh = match session.backend.as_ref() {
                            crate::core::backend::SessionBackend::Ssh { session, .. } => {
                                session.clone()
                            }
                            _ => return Err("No SSH session".to_string()),
                        };
                        let mut ssh_guard = ssh.lock().await;
                        let created = ssh_guard
                            .open_sftp()
                            .await
                            .map_err(|e| format!("SFTP init failed: {}", e))?;
                        *guard = Some(created);
                    }
                    let sftp = guard
                        .as_ref()
                        .ok_or_else(|| "SFTP not available".to_string())?;
                    sftp.rename(old_path, new_path)
                        .await
                        .map_err(|e| format!("Rename failed: {}", e))
                },
                move |result| Message::SftpRenameFinished(tab_index, result),
            ))
        }
    }
}

fn start_delete(app: &mut App) -> Option<Task<Message>> {
    let tab_index = app.active_tab;
    let (target, local_path, remote_path) = {
        let state = app.sftp_state_for_tab_mut(tab_index)?;
        let target = state.delete_target.clone()?;
        (target, state.local_path.clone(), state.remote_path.clone())
    };
    match target.pane {
        SftpPane::Local => {
            let path = join_local_path(&local_path, &target.name);
            Some(Task::perform(
                async move {
                    if target.is_dir {
                        tokio::fs::remove_dir_all(path)
                            .await
                            .map_err(|e| format!("Delete failed: {}", e))
                    } else {
                        tokio::fs::remove_file(path)
                            .await
                            .map_err(|e| format!("Delete failed: {}", e))
                    }
                },
                move |result| Message::SftpDeleteFinished(tab_index, result),
            ))
        }
        SftpPane::Remote => {
            let tab = app.tabs.get(tab_index)?;
            let session = match &tab.session {
                Some(session) => session.clone(),
                None => return None,
            };
            let sftp_session = tab.sftp_session.clone();
            let path = join_remote_path(&remote_path, &target.name);
            Some(Task::perform(
                async move {
                    let mut guard = sftp_session.lock().await;
                    if guard.is_none() {
                        let ssh = match session.backend.as_ref() {
                            crate::core::backend::SessionBackend::Ssh { session, .. } => {
                                session.clone()
                            }
                            _ => return Err("No SSH session".to_string()),
                        };
                        let mut ssh_guard = ssh.lock().await;
                        let created = ssh_guard
                            .open_sftp()
                            .await
                            .map_err(|e| format!("SFTP init failed: {}", e))?;
                        *guard = Some(created);
                    }
                    let sftp = guard
                        .as_ref()
                        .ok_or_else(|| "SFTP not available".to_string())?;
                    if target.is_dir {
                        sftp.remove_dir(path)
                            .await
                            .map_err(|e| format!("Delete failed: {}", e))
                    } else {
                        sftp.remove_file(path)
                            .await
                            .map_err(|e| format!("Delete failed: {}", e))
                    }
                },
                move |result| Message::SftpDeleteFinished(tab_index, result),
            ))
        }
    }
}

fn schedule_transfer_tasks(app: &mut App, tab_index: usize) -> Option<Task<Message>> {
    let max_concurrent = app.sftp_max_concurrent.max(1);
    let tx = app.sftp_transfer_tx.clone();
    let mut tasks = Vec::new();

    loop {
        let (transfer, transfer_index) = {
            let state = app.sftp_state_for_tab_mut(tab_index)?;
            let active = state
                .transfers
                .iter()
                .filter(|transfer| transfer.status == SftpTransferStatus::Uploading)
                .count();
            if active >= max_concurrent {
                break;
            }
            let Some(index) = state
                .transfers
                .iter()
                .position(|transfer| transfer.status == SftpTransferStatus::Queued)
            else {
                break;
            };
            let transfer = state.transfers[index].clone();
            state.transfers[index].status = SftpTransferStatus::Uploading;
            (transfer, index)
        };

        let tab = match app.tabs.get(transfer.tab_index) {
            Some(tab) => tab,
            None => {
                if let Some(state) = app.sftp_state_for_tab_mut(tab_index) {
                    if let Some(entry) = state.transfers.get_mut(transfer_index) {
                        entry.status =
                            SftpTransferStatus::Failed("Invalid session tab".to_string());
                    }
                }
                continue;
            }
        };
        let session = match &tab.session {
            Some(session) => session.clone(),
            None => {
                if let Some(state) = app.sftp_state_for_tab_mut(tab_index) {
                    if let Some(entry) = state.transfers.get_mut(transfer_index) {
                        entry.status =
                            SftpTransferStatus::Failed("No active SSH session".to_string());
                    }
                }
                continue;
            }
        };

        let sftp_session = tab.sftp_session.clone();
        let tx = tx.clone();
        tasks.push(Task::perform(
            async move { run_transfer(session, sftp_session, transfer, tx).await },
            |_| Message::Ignore,
        ));
    }

    if tasks.is_empty() {
        None
    } else {
        Some(Task::batch(tasks))
    }
}

async fn download_remote_file(
    session: crate::core::session::Session,
    sftp_session: Arc<Mutex<Option<russh_sftp::client::SftpSession>>>,
    remote_path: String,
    local_path: String,
    transfer_id: uuid::Uuid,
    tab_index: usize,
    tx: tokio::sync::mpsc::UnboundedSender<SftpTransferUpdate>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pause_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pause_notify: std::sync::Arc<tokio::sync::Notify>,
) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let send_status = |status| {
        let _ = tx.send(SftpTransferUpdate {
            id: transfer_id,
            tab_index,
            bytes_sent: 0,
            bytes_total: 0,
            status: Some(status),
        });
    };

    let mut remote_file = {
        let mut guard = sftp_session.lock().await;
        if guard.is_none() {
            let ssh = match session.backend.as_ref() {
                crate::core::backend::SessionBackend::Ssh { session, .. } => session.clone(),
                _ => return Err("No SSH session".to_string()),
            };
            let mut ssh_guard = ssh.lock().await;
            let created = ssh_guard.open_sftp().await.map_err(|e| {
                let msg = format!("SFTP init failed: {}", e);
                send_status(SftpTransferStatus::Failed(msg.clone()));
                msg
            })?;
            *guard = Some(created);
        }
        let sftp = guard
            .as_ref()
            .ok_or_else(|| "SFTP not available".to_string())?;
        sftp.open(&remote_path).await.map_err(|e| {
            let msg = format!("Failed to open remote file: {}", e);
            send_status(SftpTransferStatus::Failed(msg.clone()));
            msg
        })?
    };

    let metadata = remote_file.metadata().await.map_err(|e| {
        let msg = format!("Failed to stat remote file: {}", e);
        send_status(SftpTransferStatus::Failed(msg.clone()));
        msg
    })?;

    if metadata.is_dir() {
        let msg = "Directory download not supported yet".to_string();
        send_status(SftpTransferStatus::Failed(msg.clone()));
        return Err(msg);
    }

    let total = metadata.size.unwrap_or(0);
    let _ = tx.send(SftpTransferUpdate {
        id: transfer_id,
        tab_index,
        bytes_sent: 0,
        bytes_total: total,
        status: Some(SftpTransferStatus::Uploading), // Reusing 'Uploading' state for running
    });

    let mut local_file = tokio::fs::File::create(&local_path).await.map_err(|e| {
        let msg = format!("Failed to create local file: {}", e);
        send_status(SftpTransferStatus::Failed(msg.clone()));
        msg
    })?;

    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
    let mut sent: u64 = 0;

    loop {
        while pause_flag.load(Ordering::SeqCst) {
            let _ = tx.send(SftpTransferUpdate {
                id: transfer_id,
                tab_index,
                bytes_sent: sent,
                bytes_total: total,
                status: Some(SftpTransferStatus::Paused),
            });
            pause_notify.notified().await;
        }
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = tx.send(SftpTransferUpdate {
                id: transfer_id,
                tab_index,
                bytes_sent: sent,
                bytes_total: total,
                status: Some(SftpTransferStatus::Canceled),
            });
            return Ok(());
        }

        let read = remote_file.read(&mut buffer).await.map_err(|e| {
            let msg = format!("Download failed: {}", e);
            send_status(SftpTransferStatus::Failed(msg.clone()));
            msg
        })?;

        if read == 0 {
            break;
        }

        local_file.write_all(&buffer[..read]).await.map_err(|e| {
            let msg = format!("Download failed: {}", e);
            send_status(SftpTransferStatus::Failed(msg.clone()));
            msg
        })?;

        sent = sent.saturating_add(read as u64);
        let _ = tx.send(SftpTransferUpdate {
            id: transfer_id,
            tab_index,
            bytes_sent: sent,
            bytes_total: total,
            status: None,
        });
    }

    let _ = local_file.sync_all().await;

    let _ = tx.send(SftpTransferUpdate {
        id: transfer_id,
        tab_index,
        bytes_sent: sent,
        bytes_total: total,
        status: Some(SftpTransferStatus::Completed),
    });

    Ok(())
}

fn handle_local_click(app: &mut App, name: String, is_dir: bool) -> Task<Message> {
    let Some(state) = app.sftp_state_for_tab_mut(app.active_tab) else {
        return Task::none();
    };
    if state.rename_target.is_some() {
        state.rename_target = None;
        state.rename_value.clear();
    }
    let now = Instant::now();
    let is_double = state
        .local_last_click
        .as_ref()
        .map(|(last_name, last_time)| {
            last_name == &name && now.duration_since(*last_time) < Duration::from_millis(400)
        })
        .unwrap_or(false);

    state.local_selected = Some(name.clone());
    state.local_last_click = Some((name.clone(), now));
    state.context_menu = None;

    if is_double && is_dir {
        let new_path = join_local_path(&state.local_path, &name);
        state.local_path = new_path;
        state.local_selected = None;
        state.local_last_click = None;
        let result = load_local_entries(&state.local_path);
        match result {
            Ok(entries) => {
                state.local_entries = entries;
                state.local_error = None;
            }
            Err(err) => {
                state.local_entries.clear();
                state.local_error = Some(err);
            }
        }
    }
    Task::none() // Return Task::none() or result of load
}

fn handle_remote_click(app: &mut App, name: String, is_dir: bool) -> Task<Message> {
    let Some(state) = app.sftp_state_for_tab_mut(app.active_tab) else {
        return Task::none();
    };
    if state.rename_target.is_some() {
        state.rename_target = None;
        state.rename_value.clear();
    }
    let now = Instant::now();
    let is_double = state
        .remote_last_click
        .as_ref()
        .map(|(last_name, last_time)| {
            last_name == &name && now.duration_since(*last_time) < Duration::from_millis(400)
        })
        .unwrap_or(false);

    state.remote_selected = Some(name.clone());
    state.remote_last_click = Some((name.clone(), now));
    state.context_menu = None;

    if is_double && is_dir {
        state.remote_path = join_remote_path(&state.remote_path, &name);
        state.remote_selected = None;
        state.remote_last_click = None;
        if let Some(task) = start_remote_list(app, app.active_tab) {
            return task;
        }
    }
    Task::none()
}

async fn run_transfer(
    session: crate::core::session::Session,
    sftp_session: Arc<Mutex<Option<russh_sftp::client::SftpSession>>>,
    transfer: SftpTransfer,
    tx: tokio::sync::mpsc::UnboundedSender<SftpTransferUpdate>,
) -> Result<(), String> {
    match transfer.direction {
        SftpTransferDirection::Upload => {
            upload_local_file(
                session,
                sftp_session,
                transfer.local_path,
                transfer.remote_path,
                transfer.id,
                transfer.tab_index,
                tx,
                transfer.cancel_flag,
                transfer.pause_flag,
                transfer.pause_notify,
            )
            .await
        }
        SftpTransferDirection::Download => {
            download_remote_file(
                session,
                sftp_session,
                transfer.remote_path,
                transfer.local_path,
                transfer.id,
                transfer.tab_index,
                tx,
                transfer.cancel_flag,
                transfer.pause_flag,
                transfer.pause_notify,
            )
            .await
        }
    }
}
