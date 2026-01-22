use iced::Task;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::session::SessionConfig;
use crate::ui::message::{ActiveView, Message};
use crate::ui::state::SessionTab;
use crate::ui::App;

pub(in crate::ui) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::CreateNewSession => {
            app.editing_session = Some(SessionConfig::new(
                String::new(),
                String::new(),
                22,
                String::new(),
            ));
            app.form_name.clear();
            app.form_host.clear();
            app.form_port = String::from("22");
            app.form_username.clear();
            app.form_password.clear();
            app.auth_method_password = false;
            app.validation_error = None;
            Task::none()
        }
        Message::EditSession(id) => {
            app.session_menu_open = None;
            if let Some(session) = app.saved_sessions.iter().find(|s| s.id == id).cloned() {
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
                app.editing_session = Some(session);
                app.validation_error = None;
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
                let password = session.password.clone().unwrap_or_default();
                println!(
                    "Connecting to {}:{} with user '{}' and password '{}'",
                    host, port, username, password
                );

                app.tabs.push(SessionTab::new(&name));
                app.active_tab = app.tabs.len() - 1;
                app.active_view = ActiveView::Terminal;
                let tab_index = app.active_tab;

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

                session.name = app.form_name.clone();
                session.host = app.form_host.clone();
                session.port = port;
                session.username = app.form_username.clone();

                if app.auth_method_password {
                    session.auth_method = crate::session::config::AuthMethod::Password;
                    session.password = Some(app.form_password.clone());
                } else {
                    session.auth_method = crate::session::config::AuthMethod::PrivateKey {
                        path: "~/.ssh/id_rsa".to_string(),
                    };
                    session.password = None;
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
            }
            Task::none()
        }
        Message::CancelSessionEdit => {
            app.editing_session = None;
            app.validation_error = None;
            Task::none()
        }
        Message::CloseSessionManager => {
            app.active_view = ActiveView::Terminal;
            Task::batch(vec![app.focus_terminal_ime()])
        }
        Message::ToggleAuthMethod => {
            app.auth_method_password = !app.auth_method_password;
            app.validation_error = None;
            Task::none()
        }
        Message::ClearValidationError => {
            app.validation_error = None;
            Task::none()
        }
        Message::SessionNameChanged(value) => {
            app.form_name = value;
            app.validation_error = None;
            Task::none()
        }
        Message::SessionHostChanged(value) => {
            app.form_host = value;
            app.validation_error = None;
            Task::none()
        }
        Message::SessionPortChanged(value) => {
            if value.chars().all(|c| c.is_numeric()) {
                app.form_port = value;
                app.validation_error = None;
            }
            Task::none()
        }
        Message::SessionUsernameChanged(value) => {
            app.form_username = value;
            app.validation_error = None;
            Task::none()
        }
        Message::SessionPasswordChanged(value) => {
            app.form_password = value;
            app.validation_error = None;
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
        _ => Task::none(),
    }
}
