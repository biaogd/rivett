use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const KEYRING_SERVICE: &str = "ssh-gui";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SshKeyEntry {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub path: String,
    pub key_type: String,
    pub fingerprint: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub terminal_font_size: f32,
    #[serde(default)]
    pub use_gpu_renderer: bool,
    #[serde(default)]
    pub ssh_keys: Vec<SshKeyEntry>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            terminal_font_size: 12.0,
            use_gpu_renderer: true,
            ssh_keys: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SettingsFile {
    version: String,
    settings: AppSettings,
}

#[derive(Debug)]
pub struct SettingsStorage {
    file_path: PathBuf,
}

impl SettingsStorage {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home.join(".ssh-gui");

        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }

        Self {
            file_path: config_dir.join("settings.json"),
        }
    }

    pub fn load_settings(&self) -> Result<AppSettings, String> {
        if !self.file_path.exists() {
            return Ok(AppSettings::default());
        }

        let contents = fs::read_to_string(&self.file_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;

        let file: SettingsFile = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse settings file: {}", e))?;

        let mut settings = file.settings;
        let mut needs_save = false;
        for key in &mut settings.ssh_keys {
            if key.id.trim().is_empty() {
                key.id = Uuid::new_v4().to_string();
                needs_save = true;
            }
        }

        if needs_save {
            let _ = self.save_settings(&settings);
        }

        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let file = SettingsFile {
            version: "1.0".to_string(),
            settings: settings.clone(),
        };

        let contents = serde_json::to_string_pretty(&file)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&self.file_path, contents)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        Ok(())
    }
}

#[derive(Clone, Copy)]
enum KeySecretKind {
    PrivateKey,
}

fn key_secret_key(key_id: &str, kind: KeySecretKind) -> String {
    match kind {
        KeySecretKind::PrivateKey => format!("ssh-key:{}:private", key_id),
    }
}

pub fn store_key_secret(key_id: &str, secret: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &key_secret_key(key_id, KeySecretKind::PrivateKey))
        .map_err(|e| e.to_string())?;
    entry.set_password(secret).map_err(|e| e.to_string())
}

pub fn load_key_secret(key_id: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &key_secret_key(key_id, KeySecretKind::PrivateKey)).ok()?;
    entry.get_password().ok()
}

pub fn delete_key_secret(key_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &key_secret_key(key_id, KeySecretKind::PrivateKey))
        .map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
