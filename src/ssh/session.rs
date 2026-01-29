use anyhow::Result;
use dirs::home_dir;
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg, decode_secret_key, load_secret_key};
use russh::{ChannelId, client};
use russh_sftp::client::SftpSession;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::connection::{RemoteForwardMap, RemoteForwardTarget, SshClient, remote_forward_key};
use crate::session::config::{AuthMethod, PortForwardDirection, PortForwardRule};

use std::fmt;

pub struct SshSession {
    #[allow(dead_code)]
    session: Arc<AsyncMutex<client::Handle<SshClient>>>,
    active_channel: Option<russh::ChannelWriteHalf<client::Msg>>,
    shell_channel: Arc<StdMutex<Option<ChannelId>>>,
    port_forwards: HashMap<String, PortForwardHandle>,
    remote_forwards: RemoteForwardMap,
}

const CONNECT_TIMEOUT_SECS: u64 = 10;
const KEEPALIVE_INTERVAL_SECS: u64 = 30;
const KEEPALIVE_MAX: usize = 3;

impl fmt::Debug for SshSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SshSession")
    }
}

struct PortForwardHandle {
    kind: PortForwardKind,
}

enum PortForwardKind {
    Local {
        cancel: oneshot::Sender<()>,
        _task: JoinHandle<()>,
    },
    Dynamic {
        cancel: oneshot::Sender<()>,
        _task: JoinHandle<()>,
    },
    Remote {
        address: String,
        port: u32,
    },
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
        let remote_forwards: RemoteForwardMap = Arc::new(StdMutex::new(HashMap::new()));
        let sh = SshClient::new(tx, shell_channel.clone(), remote_forwards.clone());

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
                        session.best_supported_rsa_hash().await?.flatten()
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
                    session: Arc::new(AsyncMutex::new(session)),
                    active_channel: None,
                    shell_channel,
                    port_forwards: HashMap::new(),
                    remote_forwards,
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
        let mut session = self.session.lock().await;
        let result = session.authenticate_password(username, password).await?;
        Ok(result.success())
    }

    pub async fn open_shell(&mut self) -> Result<ChannelId> {
        let session = self.session.lock().await;
        let channel = session.channel_open_session().await?;
        channel
            .request_pty(true, "xterm-256color", 80, 24, 0, 0, &[])
            .await?;
        channel.request_shell(true).await?;
        let id = channel.id();
        let (mut read_half, write_half) = channel.split();
        tokio::spawn(async move { while let Some(_msg) = read_half.wait().await {} });
        self.active_channel = Some(write_half);
        if let Ok(mut guard) = self.shell_channel.lock() {
            *guard = Some(id);
        }
        Ok(id)
    }

    pub async fn open_sftp(&mut self) -> Result<SftpSession> {
        let session = self.session.lock().await;
        let channel = session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;
        Ok(sftp)
    }

    pub async fn write_data(&mut self, channel_id: ChannelId, data: &[u8]) -> Result<()> {
        let data = russh::CryptoVec::from_slice(data);
        tracing::debug!("write {} bytes on channel {:?}", data.len(), channel_id);
        let session = self.session.lock().await;
        match session.data(channel_id, data).await {
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

    pub async fn sync_port_forwards(
        &mut self,
        rules: &[PortForwardRule],
    ) -> std::collections::HashMap<String, Result<(), String>> {
        let mut results = std::collections::HashMap::new();
        let mut enabled = std::collections::HashSet::new();

        for rule in rules {
            if rule.enabled {
                enabled.insert(rule.id.clone());
                if !self.port_forwards.contains_key(&rule.id) {
                    tracing::info!(
                        "port forward {} starting {:?} {}:{} -> {}:{}",
                        rule.id,
                        rule.direction,
                        rule.local_host,
                        rule.local_port,
                        rule.remote_host,
                        rule.remote_port
                    );
                    let result = match rule.direction {
                        PortForwardDirection::Local => self.start_local_forward(rule).await,
                        PortForwardDirection::Remote => self.start_remote_forward(rule).await,
                        PortForwardDirection::Dynamic => self.start_dynamic_forward(rule).await,
                    };
                    match result {
                        Ok(_) => {
                            results.insert(rule.id.clone(), Ok(()));
                        }
                        Err(err) => {
                            tracing::warn!("port forward {} failed: {}", rule.id, err);
                            results.insert(rule.id.clone(), Err(err.to_string()));
                        }
                    }
                } else {
                    tracing::info!(
                        "port forward {} already active {:?} {}:{} -> {}:{}",
                        rule.id,
                        rule.direction,
                        rule.local_host,
                        rule.local_port,
                        rule.remote_host,
                        rule.remote_port
                    );
                    results.insert(rule.id.clone(), Ok(()));
                }
            } else {
                tracing::info!(
                    "port forward {} disabled {:?} {}:{} -> {}:{}",
                    rule.id,
                    rule.direction,
                    rule.local_host,
                    rule.local_port,
                    rule.remote_host,
                    rule.remote_port
                );
                results.insert(rule.id.clone(), Ok(()));
            }
        }

        let existing: Vec<String> = self.port_forwards.keys().cloned().collect();
        for id in existing {
            if !enabled.contains(&id) {
                self.stop_port_forward(&id).await;
            }
        }

        results
    }

    async fn start_local_forward(&mut self, rule: &PortForwardRule) -> Result<()> {
        if self.port_forwards.contains_key(&rule.id) {
            return Ok(());
        }

        let local_host = if rule.local_host.trim().is_empty() {
            "127.0.0.1"
        } else {
            rule.local_host.trim()
        };
        let bind_addr: std::net::SocketAddr =
            match format!("{}:{}", local_host, rule.local_port).parse() {
                Ok(addr) => addr,
                Err(err) => {
                    tracing::warn!(
                        "port forward {} invalid bind {}:{}: {}",
                        rule.id,
                        local_host,
                        rule.local_port,
                        err
                    );
                    return Err(err.into());
                }
            };
        let listener = match TcpListener::bind(bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::warn!(
                    "port forward {} bind {} failed: {}",
                    rule.id,
                    bind_addr,
                    err
                );
                return Err(err.into());
            }
        };
        let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
        let session = self.session.clone();
        let remote_host = rule.remote_host.clone();
        let remote_port = rule.remote_port;
        let rule_id = rule.id.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        tracing::info!("port forward {} stopped", rule_id);
                        break;
                    }
                    accept = listener.accept() => {
                        let (mut stream, origin) = match accept {
                            Ok(result) => result,
                            Err(err) => {
                                tracing::warn!("port forward accept error: {}", err);
                                continue;
                            }
                        };

                        let session = session.clone();
                        let remote_host = remote_host.clone();
                        tokio::spawn(async move {
                            let handle = session.lock().await;
                            let channel: russh::Channel<client::Msg> = match handle
                                .channel_open_direct_tcpip(
                                    remote_host,
                                    remote_port.into(),
                                    origin.ip().to_string(),
                                    origin.port().into(),
                                )
                                .await
                            {
                                Ok(channel) => channel,
                                Err(err) => {
                                    tracing::warn!("port forward open channel failed: {}", err);
                                    return;
                                }
                            };
                            drop(handle);

                            let mut channel_stream = channel.into_stream();
                            let _ = tokio::io::copy_bidirectional(&mut stream, &mut channel_stream)
                                .await;
                        });
                    }
                }
            }
        });

        self.port_forwards.insert(
            rule.id.clone(),
            PortForwardHandle {
                kind: PortForwardKind::Local {
                    cancel: cancel_tx,
                    _task: task,
                },
            },
        );

        tracing::info!(
            "port forward {} listening on {} -> {}:{}",
            rule.id,
            bind_addr,
            rule.remote_host,
            rule.remote_port
        );

        Ok(())
    }

    async fn start_dynamic_forward(&mut self, rule: &PortForwardRule) -> Result<()> {
        if self.port_forwards.contains_key(&rule.id) {
            return Ok(());
        }

        let local_host = if rule.local_host.trim().is_empty() {
            "127.0.0.1"
        } else {
            rule.local_host.trim()
        };
        let bind_addr: std::net::SocketAddr =
            match format!("{}:{}", local_host, rule.local_port).parse() {
                Ok(addr) => addr,
                Err(err) => {
                    tracing::warn!(
                        "dynamic forward {} invalid bind {}:{}: {}",
                        rule.id,
                        local_host,
                        rule.local_port,
                        err
                    );
                    return Err(err.into());
                }
            };
        let listener = match TcpListener::bind(bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::warn!(
                    "dynamic forward {} bind {} failed: {}",
                    rule.id,
                    bind_addr,
                    err
                );
                return Err(err.into());
            }
        };
        let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
        let session = self.session.clone();
        let rule_id = rule.id.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        tracing::info!("dynamic forward {} stopped", rule_id);
                        break;
                    }
                    accept = listener.accept() => {
                        let (mut stream, origin) = match accept {
                            Ok(result) => result,
                            Err(err) => {
                                tracing::warn!("dynamic forward accept error: {}", err);
                                continue;
                            }
                        };

                        let session = session.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_socks5(&mut stream, origin, session).await {
                                tracing::warn!("dynamic forward socks error: {}", err);
                            }
                        });
                    }
                }
            }
        });

        self.port_forwards.insert(
            rule.id.clone(),
            PortForwardHandle {
                kind: PortForwardKind::Dynamic {
                    cancel: cancel_tx,
                    _task: task,
                },
            },
        );

        tracing::info!("dynamic forward {} listening on {}", rule.id, bind_addr);

        Ok(())
    }

    async fn start_remote_forward(&mut self, rule: &PortForwardRule) -> Result<()> {
        if self.port_forwards.contains_key(&rule.id) {
            return Ok(());
        }

        let local_host = if rule.local_host.trim().is_empty() {
            "127.0.0.1"
        } else {
            rule.local_host.trim()
        };
        let remote_host = if rule.remote_host.trim().is_empty() {
            "127.0.0.1"
        } else {
            rule.remote_host.trim()
        };

        let mut handle = self.session.lock().await;
        let bound_port = handle
            .tcpip_forward(remote_host, rule.remote_port.into())
            .await?;
        drop(handle);

        let key = remote_forward_key(remote_host, bound_port);
        if let Ok(mut map) = self.remote_forwards.lock() {
            map.insert(
                key,
                RemoteForwardTarget {
                    local_host: local_host.to_string(),
                    local_port: rule.local_port,
                },
            );
        }

        self.port_forwards.insert(
            rule.id.clone(),
            PortForwardHandle {
                kind: PortForwardKind::Remote {
                    address: remote_host.to_string(),
                    port: bound_port,
                },
            },
        );

        tracing::info!(
            "port forward {} remote listening on {}:{} -> {}:{}",
            rule.id,
            remote_host,
            bound_port,
            local_host,
            rule.local_port
        );

        Ok(())
    }

    async fn stop_port_forward(&mut self, rule_id: &str) {
        if let Some(handle) = self.port_forwards.remove(rule_id) {
            match handle.kind {
                PortForwardKind::Local { cancel, .. } => {
                    let _ = cancel.send(());
                }
                PortForwardKind::Dynamic { cancel, .. } => {
                    let _ = cancel.send(());
                }
                PortForwardKind::Remote { address, port } => {
                    let key = remote_forward_key(&address, port);
                    if let Ok(mut map) = self.remote_forwards.lock() {
                        map.remove(&key);
                    }
                    let session = self.session.lock().await;
                    if let Err(err) = session.cancel_tcpip_forward(address, port).await {
                        tracing::warn!("remote forward cancel failed: {}", err);
                    }
                }
            }
        }
    }
}

async fn handle_socks5(
    stream: &mut tokio::net::TcpStream,
    origin: std::net::SocketAddr,
    session: Arc<AsyncMutex<client::Handle<SshClient>>>,
) -> Result<()> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;
    if header[0] != 0x05 {
        return Err(anyhow::anyhow!("Unsupported SOCKS version"));
    }
    let nmethods = header[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).await?;
    if !methods.iter().any(|m| *m == 0x00) {
        let _ = stream.write_all(&[0x05, 0xFF]).await;
        return Err(anyhow::anyhow!("No supported auth methods"));
    }
    stream.write_all(&[0x05, 0x00]).await?;

    let mut req = [0u8; 4];
    stream.read_exact(&mut req).await?;
    if req[0] != 0x05 {
        return Err(anyhow::anyhow!("Invalid SOCKS request version"));
    }
    if req[1] != 0x01 {
        let _ = stream
            .write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await;
        return Err(anyhow::anyhow!("Unsupported SOCKS command"));
    }

    let addr = match req[3] {
        0x01 => {
            let mut ip = [0u8; 4];
            stream.read_exact(&mut ip).await?;
            std::net::Ipv4Addr::from(ip).to_string()
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut host = vec![0u8; len[0] as usize];
            stream.read_exact(&mut host).await?;
            String::from_utf8_lossy(&host).to_string()
        }
        0x04 => {
            let mut ip = [0u8; 16];
            stream.read_exact(&mut ip).await?;
            std::net::Ipv6Addr::from(ip).to_string()
        }
        _ => {
            let _ = stream
                .write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await;
            return Err(anyhow::anyhow!("Unsupported SOCKS address type"));
        }
    };

    let mut port_bytes = [0u8; 2];
    stream.read_exact(&mut port_bytes).await?;
    let port = u16::from_be_bytes(port_bytes) as u32;

    let origin_addr = origin.ip().to_string();
    let origin_port = origin.port() as u32;
    let channel = {
        let handle = session.lock().await;
        handle
            .channel_open_direct_tcpip(addr, port, origin_addr, origin_port)
            .await?
    };

    stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;

    let mut channel_stream = channel.into_stream();
    let _ = tokio::io::copy_bidirectional(&mut channel_stream, stream).await;
    Ok(())
}
