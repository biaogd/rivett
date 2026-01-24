use crate::core::session::Session;
use crate::terminal::{TerminalDamage, TerminalEmulator};
use iced::widget::canvas::Cache;
use iced::Point;
use std::sync::Arc;
use std::sync::mpsc;
use tokio::sync::Mutex;
use russh_sftp::client::SftpSession;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Connecting(std::time::Instant), // Instant for animation start time
    Connected,
    Disconnected,
    Failed(String),
}

pub struct SessionTab {
    pub title: String,
    pub chrome_cache: Cache,
    pub line_caches: Vec<Cache>,
    pub state: SessionState,
    pub spinner_cache: Cache, // Cache for spinner drawing
    // Session (abstracted)
    pub session: Option<Session>,
    // Temporary storage for SSH handle before shell is opened
    pub ssh_handle: Option<Arc<Mutex<crate::ssh::SshSession>>>,
    pub rx: Option<Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>>,
    pub emulator: TerminalEmulator,
    pub parser_tx: Option<mpsc::Sender<Vec<u8>>>,
    pub damage_rx: Option<Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<TerminalDamage>>>>,
    pub is_dirty: bool,
    pub last_data_received: std::time::Instant,
    pub last_redraw_time: std::time::Instant,
    pub pending_damage_full: bool,
    pub pending_damage_lines: Vec<usize>,
    pub sftp_session: Arc<Mutex<Option<SftpSession>>>,
}

impl std::fmt::Debug for SessionTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTab")
            .field("title", &self.title)
            .field("state", &self.state)
            .field("is_dirty", &self.is_dirty)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionTestStatus {
    Idle,
    Testing,
    Success,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct SftpEntry {
    pub name: String,
    pub size: Option<u64>,
    pub modified: Option<chrono::DateTime<chrono::Local>>,
    pub is_dir: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SftpTransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SftpTransferStatus {
    Queued,
    Uploading,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct SftpTransfer {
    pub id: uuid::Uuid,
    pub tab_index: usize,
    pub name: String,
    pub direction: SftpTransferDirection,
    pub status: SftpTransferStatus,
    pub bytes_sent: u64,
    pub bytes_total: u64,
    pub local_path: String,
    pub remote_path: String,
    pub started_at: Option<std::time::Instant>,
    pub last_update: Option<std::time::Instant>,
    pub last_bytes_sent: u64,
    pub last_rate_bps: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SftpTransferUpdate {
    pub id: uuid::Uuid,
    pub tab_index: usize,
    pub bytes_sent: u64,
    pub bytes_total: u64,
    pub status: Option<SftpTransferStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SftpPane {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub struct SftpContextMenu {
    pub pane: SftpPane,
    pub name: String,
    pub position: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SftpContextAction {
    Upload,
    Download,
    Rename,
    Delete,
}

impl Clone for SessionTab {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            chrome_cache: iced::widget::canvas::Cache::new(),
            line_caches: Vec::new(),
            state: self.state.clone(),
            spinner_cache: iced::widget::canvas::Cache::new(),
            session: self.session.clone(),
            ssh_handle: self.ssh_handle.clone(),
            rx: self.rx.clone(),
            emulator: self.emulator.clone(),
            parser_tx: None,
            damage_rx: None,
            is_dirty: self.is_dirty,
            last_data_received: self.last_data_received,
            last_redraw_time: self.last_redraw_time,
            pending_damage_full: self.pending_damage_full,
            pending_damage_lines: self.pending_damage_lines.clone(),
            sftp_session: self.sftp_session.clone(),
        }
    }
}

impl SessionTab {
    pub fn new(title: &str) -> Self {
        let emulator = TerminalEmulator::new();
        let screen_lines = emulator.get_scroll_state().2;
        let (parser_tx, parser_rx) = mpsc::channel::<Vec<u8>>();
        let (damage_tx, damage_rx) = tokio::sync::mpsc::unbounded_channel::<TerminalDamage>();
        let mut line_caches = Vec::with_capacity(screen_lines);
        for _ in 0..screen_lines {
            line_caches.push(Cache::default());
        }

        let mut emulator_clone = emulator.clone();
        std::thread::spawn(move || {
            while let Ok(mut data) = parser_rx.recv() {
                let mut drain_count = 0;
                while drain_count < 100 {
                    match parser_rx.try_recv() {
                        Ok(chunk) => {
                            data.extend(chunk);
                            drain_count += 1;
                        }
                        Err(_) => break,
                    }
                }

                emulator_clone.process_input(&data);
                let damage = emulator_clone.take_damage();
                if damage_tx.send(damage).is_err() {
                    break;
                }
            }
        });

        Self {
            title: title.to_string(),
            chrome_cache: Cache::default(),
            line_caches,
            state: SessionState::Connecting(std::time::Instant::now()),
            spinner_cache: Cache::default(),
            session: None,
            ssh_handle: None,
            rx: None,
            emulator,
            parser_tx: Some(parser_tx),
            damage_rx: Some(Arc::new(Mutex::new(damage_rx))),
            is_dirty: false,
            last_data_received: std::time::Instant::now(),
            last_redraw_time: std::time::Instant::now(),
            pending_damage_full: true,
            pending_damage_lines: Vec::new(),
            sftp_session: Arc::new(Mutex::new(None)),
        }
    }

    pub fn ensure_line_caches(&mut self, rows: usize) {
        if self.line_caches.len() != rows {
            let mut line_caches = Vec::with_capacity(rows);
            for _ in 0..rows {
                line_caches.push(Cache::default());
            }
            self.line_caches = line_caches;
            self.pending_damage_full = true;
        }
    }

    pub fn mark_full_damage(&mut self) {
        self.pending_damage_full = true;
        self.pending_damage_lines.clear();
        self.is_dirty = true;
        self.last_data_received = std::time::Instant::now();
    }

    pub fn add_damage_lines(&mut self, lines: &[usize]) {
        if lines.is_empty() {
            return;
        }
        self.pending_damage_lines.extend_from_slice(lines);
        self.is_dirty = true;
        self.last_data_received = std::time::Instant::now();
    }
}

// Simple Spinner definition
pub(crate) struct Spinner {
    start: std::time::Instant,
}

impl Spinner {
    pub(crate) fn new(start: std::time::Instant) -> Self {
        Self { start }
    }
}

impl<Message> iced::widget::canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());

        let center = frame.center();
        let radius = bounds.width.min(bounds.height) / 2.0;
        let time = self.start.elapsed().as_secs_f32();

        // Warning: Path::arc is not a direct method, use Path::circle for shadow
        let shadow = iced::widget::canvas::Path::circle(center, radius - 4.0);
        frame.stroke(
            &shadow,
            iced::widget::canvas::Stroke::default()
                .with_color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.1))
                .with_width(4.0),
        );

        let start_angle = time * 5.0;
        let end_angle = start_angle + 1.5; // quarter circle arc

        let arc = iced::widget::canvas::Path::new(|b| {
            b.arc(iced::widget::canvas::path::Arc {
                center,
                radius: radius - 4.0,
                start_angle: iced::Radians(start_angle),
                end_angle: iced::Radians(end_angle),
            });
        });

        frame.stroke(
            &arc,
            iced::widget::canvas::Stroke::default()
                .with_color(iced::Color::from_rgb(0.2, 0.4, 0.8))
                .with_width(4.0),
        );

        vec![frame.into_geometry()]
    }
}
