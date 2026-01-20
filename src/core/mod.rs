#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

#[derive(Debug, Default)]
pub struct SessionManager {
    next_id: u64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn create_session(&mut self) -> SessionId {
        let id = SessionId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}
