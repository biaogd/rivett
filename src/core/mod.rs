pub mod backend;
pub mod session;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct SessionId(u64);

#[derive(Debug, Default)]
pub struct SessionManager {
    #[allow(dead_code)]
    next_id: u64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    #[allow(dead_code)]
    pub fn create_session(&mut self) -> SessionId {
        let id = SessionId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}
