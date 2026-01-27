use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default, skip_serializing)]
    pub password: Option<String>,
    #[serde(default, skip_serializing)]
    pub key_passphrase: Option<String>,
    pub auth_method: AuthMethod,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_connected: Option<DateTime<Utc>>,
    #[serde(default)]
    pub port_forwards: Vec<PortForwardRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    Password,
    PrivateKey {
        path: String,
        #[serde(default)]
        key_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortForwardRule {
    pub id: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    #[serde(default)]
    pub enabled: bool,
}

impl SessionConfig {
    pub fn new(name: String, host: String, port: u16, username: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            host,
            port,
            username,
            password: None,
            key_passphrase: None,
            auth_method: AuthMethod::PrivateKey {
                path: String::from("~/.ssh/id_rsa"),
                key_id: None,
            },
            color: None,
            created_at: Utc::now(),
            last_connected: None,
            port_forwards: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }
}
