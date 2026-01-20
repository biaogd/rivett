use anyhow::Result;
use russh::{ChannelId, client};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::connection::SshClient;

use std::fmt;

pub struct SshSession {
    #[allow(dead_code)]
    session: client::Handle<SshClient>,
    active_channel: Option<russh::Channel<client::Msg>>,
}

impl fmt::Debug for SshSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SshSession")
    }
}

impl SshSession {
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(Self, mpsc::UnboundedReceiver<Vec<u8>>)> {
        let config = client::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(30)),
            ..Default::default()
        };
        let config = Arc::new(config);

        // Create the channel for received data
        let (tx, rx) = mpsc::unbounded_channel();

        // Create the handler
        let sh = SshClient::new(tx);

        let addr = format!("{}:{}", host, port);
        // Connect
        let mut session = client::connect(config, addr, sh).await?;

        // Authenticate (Password)
        // TODO: Support agent auth or key auth
        if !password.is_empty() {
            let auth_res = session.authenticate_password(username, password).await?;
            if !auth_res {
                return Err(anyhow::anyhow!("Authentication failed"));
            }
        } else {
            let auth_res = session.authenticate_none(username).await?;
            if !auth_res {
                return Err(anyhow::anyhow!(
                    "Authentication failed (no password provided and none auth failed)"
                ));
            }
        }

        Ok((
            Self {
                session,
                active_channel: None,
            },
            rx,
        ))
    }

    #[allow(dead_code)]
    pub async fn call_password_auth(&mut self, username: &str, password: &str) -> Result<bool> {
        let result = self
            .session
            .authenticate_password(username, password)
            .await?;
        Ok(result)
    }

    pub async fn open_shell(&mut self) -> Result<ChannelId> {
        let channel = self.session.channel_open_session().await?;
        channel
            .request_pty(true, "xterm-256color", 80, 24, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;
        let id = channel.id();
        self.active_channel = Some(channel);
        Ok(id)
    }

    pub async fn write_data(&mut self, channel_id: ChannelId, data: &[u8]) -> Result<()> {
        let data = russh::CryptoVec::from_slice(data);
        match self.session.data(channel_id, data).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!(
                "Failed to write data to SSH channel (buffer returned)"
            )),
        }
    }

    pub async fn resize(&mut self, _channel_id: ChannelId, cols: u32, rows: u32) -> Result<()> {
        if let Some(channel) = self.active_channel.as_mut() {
            channel.window_change(cols, rows, 0, 0).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active channel to resize"))
        }
    }
}
