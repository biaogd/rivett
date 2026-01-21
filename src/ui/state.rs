use crate::core::session::Session;
use crate::terminal::TerminalEmulator;
use iced::widget::canvas::Cache;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Connecting(std::time::Instant), // Instant for animation start time
    Connected,
    Disconnected,
    Failed(String),
}

#[derive(Debug)]
pub struct SessionTab {
    pub title: String,
    pub cache: Cache,
    pub state: SessionState,
    pub spinner_cache: Cache, // Cache for spinner drawing
    // Session (abstracted)
    pub session: Option<Session>,
    // Temporary storage for SSH handle before shell is opened
    pub ssh_handle: Option<Arc<Mutex<crate::ssh::SshSession>>>,
    pub rx: Option<Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>>,
    pub emulator: TerminalEmulator,
}

impl Clone for SessionTab {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            cache: iced::widget::canvas::Cache::new(),
            state: self.state.clone(),
            spinner_cache: iced::widget::canvas::Cache::new(),
            session: self.session.clone(),
            ssh_handle: self.ssh_handle.clone(),
            rx: self.rx.clone(),
            emulator: self.emulator.clone(),
        }
    }
}

impl SessionTab {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            cache: Cache::default(),
            state: SessionState::Connecting(std::time::Instant::now()),
            spinner_cache: Cache::default(),
            session: None,
            ssh_handle: None,
            rx: None,
            emulator: TerminalEmulator::new(),
        }
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
