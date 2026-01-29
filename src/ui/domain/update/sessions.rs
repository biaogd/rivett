use iced::Task;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::session::SessionConfig;
use crate::session::config::{PortForwardDirection, PortForwardRule};
use crate::ui::App;
use crate::ui::message::{ActiveView, Message, SessionDialogTab};
use crate::ui::state::{ConnectionTestStatus, PortForwardStatus, SessionTab, SftpState};
use uuid::Uuid;

pub(in crate::ui) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::CreateNewSession => {
            app.editing_session = Some(SessionConfig::new(
                String::new(),
                String::new(),
                22,
                String::new(),
            ));
            app.session_dialog_tab = SessionDialogTab::General;
            app.form_name.clear();
            app.form_host.clear();
            app.form_port = String::from("22");
            app.form_username.clear();
            app.form_password.clear();
            app.form_key_id = app
                .app_settings
                .ssh_keys
                .iter()
                .find(|key| key.is_default)
                .or_else(|| app.app_settings.ssh_keys.first())
                .map(|key| key.id.clone())
                .unwrap_or_default();
            app.form_key_passphrase.clear();
            app.auth_method_password = false;
            app.show_password = false;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            app.saved_key_menu_open = false;
            app.port_forward_session_id = app
                .editing_session
                .as_ref()
                .map(|session| session.id.clone());
            app.port_forward_local_host = "127.0.0.1".to_string();
            app.port_forward_local_port.clear();
            app.port_forward_remote_host.clear();
            app.port_forward_remote_port.clear();
            app.port_forward_direction = PortForwardDirection::Local;
            app.port_forward_error = None;
            Task::none()
        }
        Message::EditSession(id) => {
            app.session_menu_open = None;
            if let Some(session) = app.saved_sessions.iter().find(|s| s.id == id).cloned() {
                start_edit_session(app, session, SessionDialogTab::General);
            }
            Task::none()
        }
        Message::DeleteSession(id) => {
            app.session_menu_open = None;
            if let Err(e) = app
                .session_storage
                .delete_session(&id, &mut app.saved_sessions)
            {
                eprintln!("Failed to delete session: {}", e);
            }
            Task::none()
        }
        Message::ConnectToSession(id) => {
            app.session_menu_open = None;
            if let Some(session) = app.saved_sessions.iter().find(|s| s.id == id) {
                let name = session.name.clone();
                let host = session.host.clone();
                let port = session.port;
                let username = session.username.clone();
                let password = session.password.clone();
                let auth_method = session.auth_method.clone();
                let key_passphrase = session.key_passphrase.clone();
                println!("Connecting to {}:{} with user '{}'", host, port, username);

                app.tabs.push(SessionTab::new(&name));
                let new_tab_index = app.tabs.len() - 1;
                if let Some(tab) = app.tabs.get_mut(new_tab_index) {
                    tab.sftp_key = Some(id.clone());
                }
                app.sftp_states
                    .entry(id.clone())
                    .or_insert_with(SftpState::new);
                app.active_tab = new_tab_index;
                app.active_view = ActiveView::Terminal;
                app.last_terminal_tab = app.active_tab;
                let tab_index = app.active_tab;

                let connect_task = Task::perform(
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
                            Ok((session, rx)) => {
                                Ok((Arc::new(Mutex::new(session)), Arc::new(Mutex::new(rx))))
                            }
                            Err(e) => Err(e.to_string()),
                        }
                    },
                    move |result| Message::SessionConnected(result, tab_index),
                );
                return Task::batch(vec![connect_task, app.focus_terminal_ime()]);
            }
            Task::none()
        }
        Message::SaveSession => {
            if let Some(ref mut session) = app.editing_session {
                if app.form_name.trim().is_empty() {
                    app.validation_error = Some("Session name is required".to_string());
                    return Task::none();
                }

                if app.form_host.trim().is_empty() {
                    app.validation_error = Some("Host is required".to_string());
                    return Task::none();
                }

                if app.form_username.trim().is_empty() {
                    app.validation_error = Some("Username is required".to_string());
                    return Task::none();
                }

                let port = match app.form_port.parse::<u16>() {
                    Ok(p) if p > 0 => p,
                    _ => {
                        app.validation_error =
                            Some("Port must be a number between 1 and 65535".to_string());
                        return Task::none();
                    }
                };

                if app.auth_method_password && app.form_password.trim().is_empty() {
                    app.validation_error =
                        Some("Password is required for password authentication".to_string());
                    return Task::none();
                }

                if !app.auth_method_password && app.form_key_id.trim().is_empty() {
                    app.validation_error = Some("Private key is required".to_string());
                    return Task::none();
                }

                session.name = app.form_name.clone();
                session.host = app.form_host.clone();
                session.port = port;
                session.username = app.form_username.clone();

                if app.auth_method_password {
                    session.auth_method = crate::session::config::AuthMethod::Password;
                    session.password = Some(app.form_password.clone());
                    session.key_passphrase = None;
                } else {
                    let key_id = app.form_key_id.trim().to_string();
                    let key_path = app
                        .app_settings
                        .ssh_keys
                        .iter()
                        .find(|key| key.id == key_id)
                        .map(|key| key.path.clone())
                        .unwrap_or_default();
                    session.auth_method = crate::session::config::AuthMethod::PrivateKey {
                        path: key_path,
                        key_id: if key_id.is_empty() {
                            None
                        } else {
                            Some(key_id)
                        },
                    };
                    session.password = None;
                    session.key_passphrase = if app.form_key_passphrase.trim().is_empty() {
                        None
                    } else {
                        Some(app.form_key_passphrase.clone())
                    };
                }

                if let Err(e) = app
                    .session_storage
                    .save_session(session.clone(), &mut app.saved_sessions)
                {
                    app.validation_error = Some(format!("Failed to save: {}", e));
                    return Task::none();
                }

                app.editing_session = None;
                app.validation_error = None;
                app.saved_key_menu_open = false;
                app.port_forward_session_id = None;
                app.port_forward_local_host = "127.0.0.1".to_string();
                app.port_forward_local_port.clear();
                app.port_forward_remote_host.clear();
                app.port_forward_remote_port.clear();
                app.port_forward_direction = PortForwardDirection::Local;
                app.port_forward_error = None;
            }
            Task::none()
        }
        Message::CancelSessionEdit => {
            app.editing_session = None;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            app.saved_key_menu_open = false;
            app.port_forward_session_id = None;
            app.port_forward_local_host = "127.0.0.1".to_string();
            app.port_forward_local_port.clear();
            app.port_forward_remote_host.clear();
            app.port_forward_remote_port.clear();
            app.port_forward_direction = PortForwardDirection::Local;
            app.port_forward_error = None;
            Task::none()
        }
        Message::CloseSessionManager => {
            if app.last_terminal_tab > 0 && app.last_terminal_tab < app.tabs.len() {
                app.active_tab = app.last_terminal_tab;
                app.active_view = ActiveView::Terminal;
                Task::batch(vec![app.focus_terminal_ime()])
            } else {
                app.active_tab = 0;
                app.active_view = ActiveView::SessionManager;
                Task::none()
            }
        }
        Message::ToggleAuthMethod => {
            app.auth_method_password = !app.auth_method_password;
            app.validation_error = None;
            app.show_password = false;
            app.connection_test_status = ConnectionTestStatus::Idle;
            app.saved_key_menu_open = false;
            Task::none()
        }
        Message::SessionDialogTabSelected(tab) => {
            app.session_dialog_tab = tab;
            app.saved_key_menu_open = false;
            Task::none()
        }
        Message::ClearValidationError => {
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            Task::none()
        }
        Message::SessionNameChanged(value) => {
            app.form_name = value;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            Task::none()
        }
        Message::SessionHostChanged(value) => {
            app.form_host = value;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            Task::none()
        }
        Message::SessionPortChanged(value) => {
            if value.chars().all(|c| c.is_numeric()) {
                app.form_port = value;
                app.validation_error = None;
                app.connection_test_status = ConnectionTestStatus::Idle;
            }
            Task::none()
        }
        Message::SessionUsernameChanged(value) => {
            app.form_username = value;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            Task::none()
        }
        Message::SessionPasswordChanged(value) => {
            app.form_password = value;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            Task::none()
        }
        Message::TogglePasswordVisibility => {
            app.show_password = !app.show_password;
            Task::none()
        }
        Message::SessionKeyIdChanged(value) => {
            app.form_key_id = value;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            app.saved_key_menu_open = false;
            Task::none()
        }
        Message::SessionKeyPassphraseChanged(value) => {
            app.form_key_passphrase = value;
            app.validation_error = None;
            app.connection_test_status = ConnectionTestStatus::Idle;
            Task::none()
        }
        Message::TestConnection => {
            let host = app.form_host.trim().to_string();
            if host.is_empty() {
                app.connection_test_status =
                    ConnectionTestStatus::Failed("Host is required".to_string());
                return Task::none();
            }
            let username = app.form_username.trim().to_string();
            if username.is_empty() {
                app.connection_test_status =
                    ConnectionTestStatus::Failed("Username is required".to_string());
                return Task::none();
            }
            let port = match app.form_port.trim().parse::<u16>() {
                Ok(p) if p > 0 => p,
                _ => {
                    app.connection_test_status =
                        ConnectionTestStatus::Failed("Port must be 1-65535".to_string());
                    return Task::none();
                }
            };

            let auth_method = if app.auth_method_password {
                crate::session::config::AuthMethod::Password
            } else {
                let key_id = app.form_key_id.trim().to_string();
                if key_id.is_empty() {
                    app.connection_test_status =
                        ConnectionTestStatus::Failed("Private key is required".to_string());
                    return Task::none();
                }
                let key_path = app
                    .app_settings
                    .ssh_keys
                    .iter()
                    .find(|key| key.id == key_id)
                    .map(|key| key.path.clone())
                    .unwrap_or_default();
                crate::session::config::AuthMethod::PrivateKey {
                    path: key_path,
                    key_id: Some(key_id),
                }
            };

            let password = if app.auth_method_password {
                let pass = app.form_password.clone();
                if pass.trim().is_empty() {
                    app.connection_test_status =
                        ConnectionTestStatus::Failed("Password is required".to_string());
                    return Task::none();
                }
                Some(pass)
            } else {
                None
            };

            let key_passphrase = if app.auth_method_password {
                None
            } else if app.form_key_passphrase.trim().is_empty() {
                None
            } else {
                Some(app.form_key_passphrase.clone())
            };

            app.connection_test_status = ConnectionTestStatus::Testing;

            Task::perform(
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
                        Ok(_) => Ok(()),
                        Err(err) => Err(err.to_string()),
                    }
                },
                Message::TestConnectionResult,
            )
        }
        Message::TestConnectionResult(result) => {
            match result {
                Ok(_) => app.connection_test_status = ConnectionTestStatus::Success,
                Err(err) => app.connection_test_status = ConnectionTestStatus::Failed(err),
            }
            Task::none()
        }
        Message::SessionSearchChanged(value) => {
            app.session_search_query = value;
            Task::none()
        }
        Message::ToggleSavedKeyMenu => {
            app.saved_key_menu_open = !app.saved_key_menu_open;
            Task::none()
        }
        Message::CloseSavedKeyMenu => {
            app.saved_key_menu_open = false;
            Task::none()
        }
        Message::ToggleSessionMenu(id) => {
            app.session_menu_open = if app.session_menu_open.as_deref() == Some(&id) {
                None
            } else {
                Some(id)
            };
            Task::none()
        }
        Message::CloseSessionMenu => {
            app.session_menu_open = None;
            Task::none()
        }
        Message::OpenPortForwarding(id) => {
            app.session_menu_open = None;
            if let Some(session) = app.saved_sessions.iter().find(|s| s.id == id).cloned() {
                start_edit_session(app, session, SessionDialogTab::PortForwarding);
            }
            Task::none()
        }
        Message::ClosePortForwarding => {
            app.port_forward_session_id = None;
            app.port_forward_error = None;
            Task::none()
        }
        Message::PortForwardLocalPortChanged(value) => {
            app.port_forward_local_port = value;
            app.port_forward_error = None;
            Task::none()
        }
        Message::PortForwardLocalHostChanged(value) => {
            app.port_forward_local_host = value;
            app.port_forward_error = None;
            Task::none()
        }
        Message::PortForwardRemoteHostChanged(value) => {
            app.port_forward_remote_host = value;
            app.port_forward_error = None;
            Task::none()
        }
        Message::PortForwardRemotePortChanged(value) => {
            app.port_forward_remote_port = value;
            app.port_forward_error = None;
            Task::none()
        }
        Message::PortForwardDirectionChanged(direction) => {
            app.port_forward_direction = direction;
            app.port_forward_error = None;
            Task::none()
        }
        Message::AddPortForward => {
            let session_id = match app.port_forward_session_id.clone() {
                Some(id) => id,
                None => return Task::none(),
            };

            let local_host_input = app.port_forward_local_host.trim().to_string();
            let remote_host_input = app.port_forward_remote_host.trim().to_string();
            let direction = app.port_forward_direction.clone();

            let local_port = match app.port_forward_local_port.trim().parse::<u16>() {
                Ok(port) if port > 0 => port,
                _ => {
                    app.port_forward_error =
                        Some("Local port must be a number between 1 and 65535".to_string());
                    return Task::none();
                }
            };

            let remote_port = if direction == PortForwardDirection::Dynamic {
                0
            } else {
                match app.port_forward_remote_port.trim().parse::<u16>() {
                    Ok(port) if port > 0 => port,
                    _ => {
                        app.port_forward_error =
                            Some("Remote port must be a number between 1 and 65535".to_string());
                        return Task::none();
                    }
                }
            };

            let (local_host, remote_host) = match direction {
                PortForwardDirection::Local => {
                    if remote_host_input.is_empty() {
                        app.port_forward_error = Some("Remote host is required".to_string());
                        return Task::none();
                    }
                    let local_host = if local_host_input.is_empty() {
                        "127.0.0.1".to_string()
                    } else {
                        local_host_input
                    };
                    (local_host, remote_host_input)
                }
                PortForwardDirection::Remote => {
                    if local_host_input.is_empty() {
                        app.port_forward_error = Some("Local host is required".to_string());
                        return Task::none();
                    }
                    let remote_host = if remote_host_input.is_empty() {
                        "127.0.0.1".to_string()
                    } else {
                        remote_host_input
                    };
                    (local_host_input, remote_host)
                }
                PortForwardDirection::Dynamic => {
                    let local_host = if local_host_input.is_empty() {
                        "127.0.0.1".to_string()
                    } else {
                        local_host_input
                    };
                    (local_host, String::new())
                }
            };

            if let Some(session) = app
                .editing_session
                .as_mut()
                .filter(|session| session.id == session_id)
            {
                session.port_forwards.push(PortForwardRule {
                    id: Uuid::new_v4().to_string(),
                    direction,
                    local_host,
                    local_port,
                    remote_host,
                    remote_port,
                    enabled: true,
                });
            } else if let Some(session) = app
                .saved_sessions
                .iter_mut()
                .find(|session| session.id == session_id)
            {
                session.port_forwards.push(PortForwardRule {
                    id: Uuid::new_v4().to_string(),
                    direction,
                    local_host,
                    local_port,
                    remote_host,
                    remote_port,
                    enabled: true,
                });
                if let Err(err) = app
                    .session_storage
                    .save_session(session.clone(), &mut app.saved_sessions)
                {
                    app.port_forward_error = Some(format!("Failed to save: {}", err));
                    return Task::none();
                }
            }

            app.port_forward_local_port.clear();
            app.port_forward_local_host = "127.0.0.1".to_string();
            app.port_forward_remote_host.clear();
            app.port_forward_remote_port.clear();
            app.port_forward_direction = PortForwardDirection::Local;
            app.port_forward_error = None;
            Task::none()
        }
        Message::TogglePortForward(rule_id) => {
            let session_id = match app.port_forward_session_id.clone() {
                Some(id) => id,
                None => return Task::none(),
            };

            if let Some(session) = app
                .editing_session
                .as_mut()
                .filter(|session| session.id == session_id)
            {
                if let Some(rule) = session.port_forwards.iter_mut().find(|r| r.id == rule_id) {
                    rule.enabled = !rule.enabled;
                }
            } else if let Some(session) = app
                .saved_sessions
                .iter_mut()
                .find(|session| session.id == session_id)
            {
                if let Some(rule) = session.port_forwards.iter_mut().find(|r| r.id == rule_id) {
                    rule.enabled = !rule.enabled;
                    if let Err(err) = app
                        .session_storage
                        .save_session(session.clone(), &mut app.saved_sessions)
                    {
                        app.port_forward_error = Some(format!("Failed to save: {}", err));
                    }
                }
            }
            Task::none()
        }
        Message::DeletePortForward(rule_id) => {
            let session_id = match app.port_forward_session_id.clone() {
                Some(id) => id,
                None => return Task::none(),
            };

            if let Some(session) = app
                .editing_session
                .as_mut()
                .filter(|session| session.id == session_id)
            {
                session.port_forwards.retain(|rule| rule.id != rule_id);
            } else if let Some(session) = app
                .saved_sessions
                .iter_mut()
                .find(|session| session.id == session_id)
            {
                session.port_forwards.retain(|rule| rule.id != rule_id);
                if let Err(err) = app
                    .session_storage
                    .save_session(session.clone(), &mut app.saved_sessions)
                {
                    app.port_forward_error = Some(format!("Failed to save: {}", err));
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

fn start_edit_session(app: &mut App, session: SessionConfig, tab: SessionDialogTab) {
    app.form_name = session.name.clone();
    app.form_host = session.host.clone();
    app.form_port = session.port.to_string();
    app.form_username = session.username.clone();
    if let Some(pass) = &session.password {
        app.form_password = pass.clone();
        app.auth_method_password = true;
    } else {
        app.form_password.clear();
        app.auth_method_password = false;
    }
    if let crate::session::config::AuthMethod::Password = session.auth_method {
        app.auth_method_password = true;
    }
    if let crate::session::config::AuthMethod::PrivateKey {
        ref path,
        ref key_id,
    } = session.auth_method
    {
        if let Some(id) = key_id.as_ref() {
            app.form_key_id = id.clone();
        } else {
            app.form_key_id = app
                .app_settings
                .ssh_keys
                .iter()
                .find(|key| key.path == *path)
                .map(|key| key.id.clone())
                .unwrap_or_default();
        }
        app.auth_method_password = false;
    }
    app.form_key_passphrase = session.key_passphrase.clone().unwrap_or_default();
    app.show_password = false;
    app.editing_session = Some(session);
    app.validation_error = None;
    app.connection_test_status = ConnectionTestStatus::Idle;
    app.saved_key_menu_open = false;
    app.session_dialog_tab = tab;
    app.port_forward_session_id = app
        .editing_session
        .as_ref()
        .map(|editing| editing.id.clone());
    app.port_forward_local_host = "127.0.0.1".to_string();
    app.port_forward_local_port.clear();
    app.port_forward_remote_host.clear();
    app.port_forward_remote_port.clear();
    app.port_forward_direction = PortForwardDirection::Local;
    app.port_forward_error = None;
}

pub(in crate::ui) fn apply_port_forwards(app: &App, session_id: &str) -> Task<Message> {
    let mut rules = match app
        .saved_sessions
        .iter()
        .find(|session| session.id == session_id)
    {
        Some(session) => session.port_forwards.clone(),
        None => return Task::none(),
    };
    for rule in &mut rules {
        rule.enabled = true;
    }
    tracing::info!(
        "port forward apply {} rules for session {}",
        rules.len(),
        session_id
    );

    let mut tasks = Vec::new();
    for tab in &app.tabs {
        if tab.sftp_key.as_deref() == Some(session_id) {
            if let Some(session) = &tab.ssh_handle {
                let session = session.clone();
                let rules = rules.clone();
                let session_id = session_id.to_string();
                tasks.push(Task::perform(
                    async move {
                        let mut guard = session.lock().await;
                        let results = guard.sync_port_forwards(&rules).await;
                        let statuses = rules
                            .into_iter()
                            .map(|rule| {
                                let status = if let Some(Err(err)) = results.get(&rule.id) {
                                    PortForwardStatus::Error(err.clone())
                                } else {
                                    PortForwardStatus::Active
                                };
                                (rule.id, status)
                            })
                            .collect::<Vec<_>>();
                        (session_id, statuses)
                    },
                    |(session_id, statuses)| {
                        Message::PortForwardStatusUpdated(session_id, statuses)
                    },
                ));
            }
        }
    }

    if tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(tasks)
    }
}
