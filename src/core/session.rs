use crate::core::backend::SessionBackend;
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Session {
    pub backend: Arc<SessionBackend>,
    // We can add more common session state here (e.g. title, status)
}

impl Session {
    pub fn new(backend: SessionBackend) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    pub async fn write(&self, data: &[u8]) -> Result<()> {
        self.backend.write(data).await
    }

    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.backend.resize(cols, rows).await
    }
}
