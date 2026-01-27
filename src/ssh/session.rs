use anyhow::Result;
use dirs::home_dir;
use russh::{ChannelId, client};
use russh::keys::{decode_secret_key, load_secret_key, PrivateKey, PrivateKeyWithHashAlg};
use russh_sftp::client::SftpSession;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::mpsc;

use super::connection::SshClient;
use crate::session::config::AuthMethod;

use std::fmt;

pub struct SshSession {
    #[allow(dead_code)]
    session: client::Handle<SshClient>,
    active_channel: Option<russh::ChannelWriteHalf<client::Msg>>,
    shell_channel: Arc<StdMutex<Option<ChannelId>>>,
}

const CONNECT_TIMEOUT_SECS: u64 = 10;
const KEEPALIVE_INTERVAL_SECS: u64 = 30;
const KEEPALIVE_MAX: usize = 3;

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
        auth_method: AuthMethod,
        password: Option<String>,
        key_passphrase: Option<String>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<Vec<u8>>)> {
        tracing::info!("ssh connect start {}@{}:{}", username, host, port);
        let config = client::Config {
            inactivity_timeout: None,
            keepalive_interval: Some(std::time::Duration::from_secs(KEEPALIVE_INTERVAL_SECS)),
            keepalive_max: KEEPALIVE_MAX,
            ..Default::default()
        };
        let config = Arc::new(config);

        // Create the channel for received data
        let (tx, rx) = mpsc::unbounded_channel();

        // Create the handler
        let shell_channel = Arc::new(StdMutex::new(None));
        let sh = SshClient::new(tx, shell_channel.clone());

        let addr = format!("{}:{}", host, port);
        let timeout = std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS);
        let connect_result = tokio::time::timeout(timeout, async move {
            let mut session = client::connect(config, addr, sh).await?;

            match auth_method {
                AuthMethod::Password => {
                    let password = password.unwrap_or_default();
                    if password.trim().is_empty() {
                        return Err(anyhow::anyhow!("Password required for authentication"));
                    }
                let auth_res = session.authenticate_password(username, password).await?;
                if !auth_res.success() {
                    return Err(anyhow::anyhow!("Authentication failed"));
                }
                tracing::info!("ssh auth success (password)");
            }
            AuthMethod::PrivateKey { path, key_id } => {
                let mut key_source: Option<String> = None;
                if let Some(id) = key_id.as_deref() {
                    key_source = crate::settings::load_key_secret(id);
                }

                let key: PrivateKey = if let Some(secret) = key_source.as_deref() {
                    decode_secret_key(secret, key_passphrase.as_deref())?
                } else if !path.trim().is_empty() {
                    let expanded = Self::expand_tilde(&path);
                    load_secret_key(&expanded, key_passphrase.as_deref())?
                } else {
                    return Err(anyhow::anyhow!("Private key content is missing"));
                };
                let hash_alg = if key.algorithm().is_rsa() {
                    session
                        .best_supported_rsa_hash()
                        .await?
                        .flatten()
                } else {
                    None
                };
                let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg);
                let auth_res = session
                    .authenticate_publickey(username, key_with_alg)
                    .await?;
                if !auth_res.success() {
                    return Err(anyhow::anyhow!("Authentication failed"));
                }
                tracing::info!("ssh auth success (public key)");
            }
        }

        Ok((
            Self {
                session,
                active_channel: None,
                shell_channel,
            },
            rx,
        ))
        })
        .await;

        match connect_result {
            Ok(result) => {
                if result.is_ok() {
                    tracing::info!("ssh connect ok {}@{}:{}", username, host, port);
                }
                result
            }
            Err(_) => Err(anyhow::anyhow!(
                "Connection timeout ({}s)",
                CONNECT_TIMEOUT_SECS
            )),
        }
    }

    fn expand_home(path: &str) -> Option<String> {
        if !path.starts_with("~/") {
            return None;
        }
        let home = home_dir()?;
        let rest = path.trim_start_matches("~/");
        Some(home.join(rest).to_string_lossy().to_string())
    }

    fn expand_tilde(path: &str) -> String {
        Self::expand_home(path).unwrap_or_else(|| path.to_string())
    }

    #[allow(dead_code)]
    pub async fn call_password_auth(&mut self, username: &str, password: &str) -> Result<bool> {
        let result = self
            .session
            .authenticate_password(username, password)
            .await?;
        Ok(result.success())
    }

    pub async fn open_shell(&mut self) -> Result<ChannelId> {
        let channel = self.session.channel_open_session().await?;
        channel
            .request_pty(true, "xterm-256color", 80, 24, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;
        let id = channel.id();
        let (mut read_half, write_half) = channel.split();
        tokio::spawn(async move {
            while let Some(_msg) = read_half.wait().await {}
        });
        self.active_channel = Some(write_half);
        if let Ok(mut guard) = self.shell_channel.lock() {
            *guard = Some(id);
        }
        Ok(id)
    }

    pub async fn open_sftp(&mut self) -> Result<SftpSession> {
        let channel = self.session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;
        Ok(sftp)
    }

    pub async fn write_data(&mut self, channel_id: ChannelId, data: &[u8]) -> Result<()> {
        let data = russh::CryptoVec::from_slice(data);
        tracing::debug!("write {} bytes on channel {:?}", data.len(), channel_id);
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
