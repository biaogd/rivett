use russh::{ChannelId, client};
use russh::keys::PublicKey;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct SshClient {
    tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl SshClient {
    pub fn new(tx: mpsc::UnboundedSender<Vec<u8>>) -> Self {
        Self { tx }
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

    fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let tx = self.tx.clone();
        let data = data.to_vec();
        async move {
            println!("DEBUG: Received {} bytes on channel {:?}", data.len(), channel);
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
            println!("DEBUG: Channel {:?} closed by server", channel);
            Ok(())
        }
    }

    fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut client::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            println!("DEBUG: Channel {:?} sent EOF", channel);
            Ok(())
        }
    }
}
