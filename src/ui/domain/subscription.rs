use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ui::message::Message;
use crate::ui::state::SessionState;
use crate::ui::App;

impl App {
    pub(in crate::ui) fn subscription(&self) -> iced::Subscription<Message> {
        use iced::event;

        let mut subs = Vec::new();

        // Add Tick subscription for render throttling (approx 60 FPS check rate)
        subs.push(iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick));

        if let Some(main_window) = self.main_window {
            let events = event::listen_with(|event, _status, id| Some((id, event)))
                .with(main_window)
                .filter_map(|(target, (id, event))| {
                    if id == target {
                        Some(Message::RuntimeEvent(event, id))
                    } else {
                        None
                    }
                });
            subs.push(events);
        }

        subs.push(iced::window::close_events().map(Message::WindowClosed));

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
                                Some(damage) => Some((Message::TerminalDamaged(idx, damage), rx)),
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
