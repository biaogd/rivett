use crate::session::config::SessionConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
        let config_dir = home.join(".ssh-gui");

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

        Ok(file.sessions)
    }

    pub fn save_sessions(&self, sessions: &[SessionConfig]) -> Result<(), String> {
        let file = SessionsFile {
            version: "1.0".to_string(),
            sessions: sessions.to_vec(),
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
        self.save_sessions(existing)
    }
}
