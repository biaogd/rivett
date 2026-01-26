use russh::{ChannelId, client};
use russh::keys::PublicKey;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct SshClient {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    shell_channel: Arc<Mutex<Option<ChannelId>>>,
}

impl SshClient {
    pub fn new(
        tx: mpsc::UnboundedSender<Vec<u8>>,
        shell_channel: Arc<Mutex<Option<ChannelId>>>,
    ) -> Self {
        Self { tx, shell_channel }
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

    fn adjust_window(
        &mut self,
        channel: ChannelId,
        window: u32,
    ) -> u32 {
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
