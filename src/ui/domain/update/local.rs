use iced::Task;
use std::sync::Arc;
use tokio::sync::Mutex;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use uuid::Uuid;

use crate::ui::App;
use crate::ui::message::{ActiveView, Message};
use crate::ui::state::{SessionState, SessionTab, SftpState};

pub(in crate::ui) fn create_local_tab(app: &mut App) -> Task<Message> {
    let mut commands = Vec::new();

    app.show_quick_connect = false;
    let system = native_pty_system();
    let size = PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    };

    match system.openpty(size) {
        Ok(pair) => {
            let mut cmd = CommandBuilder::new("zsh");
            cmd.env("TERM", "xterm-256color");
            cmd.env("LANG", "en_US.UTF-8");
            cmd.env("LC_ALL", "en_US.UTF-8");

            match pair.slave.spawn_command(cmd) {
                Ok(_) => {
                    println!("Local: process spawned");
                    let master = pair.master;
                    let mut reader = master.try_clone_reader().unwrap();

                    let backend = crate::core::backend::SessionBackend::Local {
                        master: Arc::new(std::sync::Mutex::new(master)),
                    };
                    let session = crate::core::session::Session::new(backend);

                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

                    std::thread::spawn(move || {
                        println!("Local: reader thread started");
                        let mut buf = [0u8; 1024];
                        loop {
                            match reader.read(&mut buf) {
                                Ok(n) if n > 0 => {
                                    if let Err(e) = tx.send(buf[..n].to_vec()) {
                                        println!("Local: failed to send to channel: {}", e);
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
                    let sftp_key = format!("local:{}", Uuid::new_v4());
                    tab.sftp_key = Some(sftp_key.clone());
                    app.sftp_states
                        .entry(sftp_key)
                        .or_insert_with(SftpState::new);
                    tab.state = SessionState::Connected;
                    tab.session = Some(session.clone());
                    tab.rx = Some(Arc::new(Mutex::new(rx)));

                    if let Some(mut output_rx) = tab.emulator.take_output_receiver() {
                        let session_clone = session.clone();
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

                    app.tabs.push(tab);
                    let tab_index = app.tabs.len() - 1;
                    app.active_tab = tab_index;
                    app.active_view = ActiveView::Terminal;
                    app.last_terminal_tab = tab_index;
                    commands.push(app.focus_terminal_ime());

                    if let Some(tab) = app.tabs.get_mut(tab_index) {
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

                            let width = app.window_width;
                            let height = app.window_height;
                            if width > 0 && height > 0 {
                                let reserved_width = 0.0;
                                let h_padding = 24.0;
                                let v_padding = 80.0;

                                let term_w = (width as f32 - reserved_width - h_padding).max(0.0);
                                let term_h = (height as f32 - v_padding).max(0.0);

                                let cols = (term_w / app.cell_width()) as usize;
                                let rows = (term_h / app.cell_height()) as usize;

                                commands.push(Task::done(Message::TerminalResize(cols, rows)));
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

    Task::batch(commands)
}
