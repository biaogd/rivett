use crate::session::config::SessionConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "rivett";

#[derive(Debug, Serialize, Deserialize)]
struct SessionsFile {
    version: String,
    sessions: Vec<SessionConfig>,
}

#[derive(Debug)]
pub struct SessionStorage {
    file_path: PathBuf,
}

impl SessionStorage {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home.join(".rivett");

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }

        Self {
            file_path: config_dir.join("sessions.json"),
        }
    }

    pub fn load_sessions(&self) -> Result<Vec<SessionConfig>, String> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&self.file_path)
            .map_err(|e| format!("Failed to read sessions file: {}", e))?;

        let file: SessionsFile = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse sessions file: {}", e))?;

        let mut sessions = file.sessions;
        for session in &mut sessions {
            session.password = load_secret(&session.id, SecretKind::Password);
            session.key_passphrase = load_secret(&session.id, SecretKind::KeyPassphrase);
        }

        Ok(sessions)
    }

    pub fn save_sessions(&self, sessions: &[SessionConfig]) -> Result<(), String> {
        for session in sessions {
            if let Some(password) = session.password.as_deref() {
                if let Err(err) = store_secret(&session.id, SecretKind::Password, password) {
                    tracing::warn!("Failed to store password in keyring: {}", err);
                }
            } else if let Err(err) = delete_secret(&session.id, SecretKind::Password) {
                tracing::warn!("Failed to delete password from keyring: {}", err);
            }

            if let Some(passphrase) = session.key_passphrase.as_deref() {
                if let Err(err) = store_secret(&session.id, SecretKind::KeyPassphrase, passphrase) {
                    tracing::warn!("Failed to store key passphrase in keyring: {}", err);
                }
            } else if let Err(err) = delete_secret(&session.id, SecretKind::KeyPassphrase) {
                tracing::warn!("Failed to delete key passphrase from keyring: {}", err);
            }
        }

        let sanitized: Vec<_> = sessions
            .iter()
            .cloned()
            .map(|mut session| {
                session.password = None;
                session.key_passphrase = None;
                session
            })
            .collect();
        let file = SessionsFile {
            version: "1.0".to_string(),
            sessions: sanitized,
        };

        let contents = serde_json::to_string_pretty(&file)
            .map_err(|e| format!("Failed to serialize sessions: {}", e))?;

        fs::write(&self.file_path, contents)
            .map_err(|e| format!("Failed to write sessions file: {}", e))?;

        Ok(())
    }
    pub fn save_session(
        &self,
        config: SessionConfig,
        existing: &mut Vec<SessionConfig>,
    ) -> Result<(), String> {
        if let Some(session) = existing.iter_mut().find(|s| s.id == config.id) {
            *session = config;
        } else {
            existing.push(config);
        }
        self.save_sessions(existing)
    }

    pub fn delete_session(
        &self,
        id: &str,
        existing: &mut Vec<SessionConfig>,
    ) -> Result<(), String> {
        existing.retain(|s| s.id != id);
        if let Err(err) = delete_secret(id, SecretKind::Password) {
            tracing::warn!("Failed to delete password from keyring: {}", err);
        }
        if let Err(err) = delete_secret(id, SecretKind::KeyPassphrase) {
            tracing::warn!("Failed to delete key passphrase from keyring: {}", err);
        }
        self.save_sessions(existing)
    }
}

#[derive(Clone, Copy)]
enum SecretKind {
    Password,
    KeyPassphrase,
}

fn secret_key(session_id: &str, kind: SecretKind) -> String {
    match kind {
        SecretKind::Password => format!("session:{}:password", session_id),
        SecretKind::KeyPassphrase => format!("session:{}:key_passphrase", session_id),
    }
}

fn store_secret(session_id: &str, kind: SecretKind, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &secret_key(session_id, kind))
        .map_err(|e| e.to_string())?;
    entry.set_password(value).map_err(|e| e.to_string())
}

fn load_secret(session_id: &str, kind: SecretKind) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &secret_key(session_id, kind)).ok()?;
    entry.get_password().ok()
}

fn delete_secret(session_id: &str, kind: SecretKind) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &secret_key(session_id, kind))
        .map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}
