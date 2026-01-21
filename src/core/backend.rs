use anyhow::Result;
use portable_pty::MasterPty;
use std::io::Write;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex as AsyncMutex;

// #[derive(Debug)] // Removed because we implement manual Debug
pub enum SessionBackend {
    Ssh {
        session: Arc<AsyncMutex<crate::ssh::SshSession>>,
        channel_id: russh::ChannelId,
    },
    Local {
        master: Arc<StdMutex<Box<dyn MasterPty + Send>>>,
    },
}

impl std::fmt::Debug for SessionBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ssh {
                session: _,
                channel_id,
            } => f
                .debug_struct("Ssh")
                .field("session", &"<hidden>")
                .field("channel_id", channel_id)
                .finish(),
            Self::Local { .. } => f.debug_struct("Local").finish(),
        }
    }
}

impl SessionBackend {
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        match self {
            SessionBackend::Ssh {
                session,
                channel_id,
            } => {
                let mut session = session.lock().await;
                session.write_data(*channel_id, data).await?;
                Ok(())
            }
            SessionBackend::Local { master } => {
                let master = master.lock().unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::io::FromRawFd;
                    // generic-pty MasterPty::as_raw_fd returns Option<RawFd> on recent versions?
                    if let Some(fd) = master.as_raw_fd() {
                        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
                        let res = file.write_all(data);
                        std::mem::forget(file); // Prevent closing the FD
                        res?;
                    } else {
                        return Err(anyhow::anyhow!("Local PTY failed to provide raw FD"));
                    }
                }
                #[cfg(not(unix))]
                {
                    // Fallback or error for Windows (if dyn MasterPty doesn't impl Write)
                    // Try coercion if possible, else error
                    return Err(anyhow::anyhow!(
                        "Local PTY write not implemented for this platform logic"
                    ));
                }
                Ok(())
            }
        }
    }

    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        match self {
            SessionBackend::Ssh {
                session,
                channel_id,
            } => {
                let mut session = session.lock().await;
                session
                    .resize(*channel_id, cols as u32, rows as u32)
                    .await?;
                Ok(())
            }
            SessionBackend::Local { master } => {
                let master = master.lock().unwrap();
                master.resize(portable_pty::PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })?;
                Ok(())
            }
        }
    }
}
