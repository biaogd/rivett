use async_trait::async_trait;
use russh::{ChannelId, client};
use russh_keys::key;
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

#[async_trait]
impl client::Handler for SshClient {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // For now, accept all keys. In a real app, we should verify against known_hosts.
        // TODO: Implement known_hosts verification
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        println!(
            "DEBUG: Received {} bytes on channel {:?}",
            data.len(),
            channel
        );
        // Forward data to the receiver
        if let Err(e) = self.tx.send(data.to_vec()) {
            eprintln!("Failed to send SSH data to UI: {}", e);
        }
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        println!("DEBUG: Channel {:?} closed by server", channel);
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        println!("DEBUG: Channel {:?} sent EOF", channel);
        Ok(())
    }
}
