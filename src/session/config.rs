use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub auth_method: AuthMethod,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_connected: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    Password,
    PrivateKey { path: String },
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
            auth_method: AuthMethod::PrivateKey {
                path: String::from("~/.ssh/id_rsa"),
            },
            color: None,
            created_at: Utc::now(),
            last_connected: None,
        }
    }

    #[allow(dead_code)]
    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }
}
