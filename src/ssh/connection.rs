use russh::keys::PublicKey;
use russh::{ChannelId, client};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct SshClient {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    shell_channel: Arc<Mutex<Option<ChannelId>>>,
    remote_forwards: RemoteForwardMap,
}

#[derive(Clone)]
pub(super) struct RemoteForwardTarget {
    pub local_host: String,
    pub local_port: u16,
}

pub(super) type RemoteForwardMap = Arc<Mutex<HashMap<String, RemoteForwardTarget>>>;

pub(super) fn remote_forward_key(address: &str, port: u32) -> String {
    format!("{}:{}", address.trim(), port)
}

impl SshClient {
    pub fn new(
        tx: mpsc::UnboundedSender<Vec<u8>>,
        shell_channel: Arc<Mutex<Option<ChannelId>>>,
        remote_forwards: RemoteForwardMap,
    ) -> Self {
        Self {
            tx,
            shell_channel,
            remote_forwards,
        }
    }
}

impl client::Handler for SshClient {
    type Error = anyhow::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> impl std::future::Future<Output = Result<bool, Self::Error>> + Send {
        async {
            // For now, accept all keys. In a real app, we should verify against known_hosts.
            Ok(true)
        }
    }

    fn channel_open_confirmation(
        &mut self,
        id: ChannelId,
        max_packet_size: u32,
        window_size: u32,
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            tracing::info!(
                "ssh channel {:?} open (window={}, max_packet={})",
                id,
                window_size,
                max_packet_size
            );
            Ok(())
        }
    }

    fn adjust_window(&mut self, channel: ChannelId, window: u32) -> u32 {
        tracing::debug!("ssh window adjust {:?} -> {}", channel, window);
        window
    }

    fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let tx = self.tx.clone();
        let shell_channel = self.shell_channel.clone();
        let data = data.to_vec();
        async move {
            if let Ok(guard) = shell_channel.lock() {
                if let Some(active) = *guard {
                    if channel != active {
                        return Ok(());
                    }
                }
            }
            use std::sync::Mutex;
            use std::sync::OnceLock;
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::time::Instant;

            static RX_BYTES: AtomicUsize = AtomicUsize::new(0);
            static LAST_LOG: OnceLock<Mutex<Instant>> = OnceLock::new();

            RX_BYTES.fetch_add(data.len(), Ordering::Relaxed);
            let last_log = LAST_LOG.get_or_init(|| Mutex::new(Instant::now()));
            let mut last = last_log.lock().unwrap();
            if last.elapsed().as_secs() >= 1 {
                let bytes = RX_BYTES.swap(0, Ordering::Relaxed);
                tracing::info!("ssh rx {} bytes/s (channel {:?})", bytes, channel);
                *last = Instant::now();
            }
            if let Err(e) = tx.send(data) {
                eprintln!("Failed to send SSH data to UI: {}", e);
            }
            Ok(())
        }
    }

    fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            tracing::info!("ssh channel {:?} closed by server", channel);
            Ok(())
        }
    }

    fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            tracing::info!("ssh channel {:?} sent EOF", channel);
            Ok(())
        }
    }

    fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<client::Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let remote_forwards = self.remote_forwards.clone();
        let bind_key = remote_forward_key(connected_address, connected_port);
        let origin = format!("{}:{}", originator_address, originator_port);
        async move {
            let target = remote_forwards
                .lock()
                .ok()
                .and_then(|map| map.get(&bind_key).cloned());
            let Some(target) = target else {
                tracing::warn!(
                    "remote forward {} missing target (origin {})",
                    bind_key,
                    origin
                );
                let _ = channel.close().await;
                return Ok(());
            };

            tokio::spawn(async move {
                let target_addr = format!("{}:{}", target.local_host, target.local_port);
                let mut stream = match TcpStream::connect(&target_addr).await {
                    Ok(stream) => stream,
                    Err(err) => {
                        tracing::warn!(
                            "remote forward {} connect to {} failed: {}",
                            bind_key,
                            target_addr,
                            err
                        );
                        let _ = channel.close().await;
                        return;
                    }
                };

                let mut channel_stream = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut channel_stream, &mut stream).await;
            });

            Ok(())
        }
    }

    fn disconnected(
        &mut self,
        reason: client::DisconnectReason<Self::Error>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            tracing::info!("ssh disconnected: {:?}", reason);
            match reason {
                client::DisconnectReason::ReceivedDisconnect(_) => Ok(()),
                client::DisconnectReason::Error(e) => Err(e),
            }
        }
    }
}
