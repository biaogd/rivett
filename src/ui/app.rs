use iced::{Settings, Task, Theme};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::message::{ActiveView, Message, SessionDialogTab};
use super::state::{ConnectionTestStatus, SessionTab, SftpPane, SftpState, SftpTransferUpdate};
use crate::core::SessionManager;
use crate::platform::PlatformServices;
use crate::session::config::PortForwardDirection;
use crate::session::{SessionConfig, SessionStorage};
use crate::settings::ThemeMode;
use crate::settings::{AppSettings, SettingsStorage};
use crate::ui::style as ui_style;
use std::collections::HashMap;

#[derive(Debug)]
pub struct App {
    #[allow(dead_code)]
    pub(in crate::ui) sessions: SessionManager,
    #[allow(dead_code)]
    pub(in crate::ui) platform: PlatformServices,
    pub(in crate::ui) tabs: Vec<SessionTab>,
    pub(in crate::ui) active_tab: usize,
    pub(in crate::ui) main_window: Option<iced::window::Id>,
    pub(in crate::ui) settings_process: Option<std::process::Child>,
    // Session management
    pub(in crate::ui) active_view: ActiveView,
    pub(in crate::ui) saved_sessions: Vec<SessionConfig>,
    pub(in crate::ui) session_storage: SessionStorage,
    pub(in crate::ui) settings_storage: SettingsStorage,
    pub(in crate::ui) app_settings: AppSettings,
    pub(in crate::ui) terminal_font_size: f32,
    pub(in crate::ui) use_gpu_renderer: bool,
    pub(in crate::ui) editing_session: Option<SessionConfig>,
    // Form state
    pub(in crate::ui) form_name: String,
    pub(in crate::ui) form_host: String,
    pub(in crate::ui) form_port: String,
    pub(in crate::ui) form_username: String,
    pub(in crate::ui) form_password: String,
    pub(in crate::ui) form_key_id: String,
    pub(in crate::ui) form_key_passphrase: String,
    pub(in crate::ui) auth_method_password: bool,
    pub(in crate::ui) validation_error: Option<String>,
    pub(in crate::ui) session_search_query: String,
    pub(in crate::ui) show_password: bool,
    pub(in crate::ui) connection_test_status: ConnectionTestStatus,
    pub(in crate::ui) saved_key_menu_open: bool,
    pub(in crate::ui) session_dialog_tab: SessionDialogTab,
    pub(in crate::ui) port_forward_session_id: Option<String>,
    pub(in crate::ui) port_forward_local_host: String,
    pub(in crate::ui) port_forward_local_port: String,
    pub(in crate::ui) port_forward_remote_host: String,
    pub(in crate::ui) port_forward_remote_port: String,
    pub(in crate::ui) port_forward_direction: PortForwardDirection,
    pub(in crate::ui) port_forward_error: Option<String>,
    pub(in crate::ui) port_forward_statuses:
        HashMap<String, HashMap<String, crate::ui::state::PortForwardStatus>>,
    pub(in crate::ui) window_width: u32,
    pub(in crate::ui) window_height: u32,
    pub(in crate::ui) last_error: Option<(String, std::time::Instant)>, // (error message, timestamp)
    // Quick Connect
    pub(in crate::ui) show_quick_connect: bool,
    pub(in crate::ui) quick_connect_query: String,
    pub(in crate::ui) session_menu_open: Option<String>,
    pub(in crate::ui) ime_buffer: String,
    pub(in crate::ui) ime_input_id: iced::widget::Id,
    pub(in crate::ui) ime_focused: bool,
    pub(in crate::ui) last_ime_focus_check: std::time::Instant,
    pub(in crate::ui) ime_preedit: String,
    pub(in crate::ui) ime_ignore_next_input: bool,
    pub(in crate::ui) pending_resize: Option<(usize, usize, std::time::Instant)>,
    pub(in crate::ui) last_terminal_tab: usize,
    pub(in crate::ui) sftp_panel_open: bool,
    pub(in crate::ui) sftp_panel_width: f32,
    pub(in crate::ui) sftp_panel_initialized: bool,
    pub(in crate::ui) port_forward_panel_open: bool,
    pub(in crate::ui) port_forward_panel_width: f32,
    pub(in crate::ui) port_forward_panel_initialized: bool,
    pub(in crate::ui) port_forward_dragging: bool,
    pub(in crate::ui) sftp_dragging: bool, // Window resizing
    pub(in crate::ui) sftp_file_dragging: Option<(SftpPane, String)>,
    pub(in crate::ui) sftp_drag_position: Option<iced::Point>,
    pub(in crate::ui) sftp_hovered_file: Option<(SftpPane, String)>,
    pub(in crate::ui) sftp_transfer_tx: tokio::sync::mpsc::UnboundedSender<SftpTransferUpdate>,
    pub(in crate::ui) sftp_transfer_rx:
        Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<SftpTransferUpdate>>>,
    pub(in crate::ui) sftp_max_concurrent: usize,
    pub(in crate::ui) sftp_rename_input_id: iced::widget::Id,
    pub(in crate::ui) sftp_states: HashMap<String, SftpState>,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let storage = SessionStorage::new();
        let saved_sessions = storage.load_sessions().unwrap_or_else(|e| {
            eprintln!("Failed to load sessions: {}", e);
            Vec::new()
        });
        let settings_storage = SettingsStorage::new();
        let app_settings = settings_storage.load_settings().unwrap_or_default();
        ui_style::set_dark_mode(matches!(app_settings.theme, ThemeMode::Dark));
        let use_gpu_renderer = app_settings.use_gpu_renderer;
        let mut sessions_tab = SessionTab::new("Sessions");
        sessions_tab.sftp_key = Some("session-manager".to_string());

        let (main_window, open_task) = iced::window::open(iced::window::Settings::default());

        let (sftp_transfer_tx, sftp_transfer_rx) =
            tokio::sync::mpsc::unbounded_channel::<SftpTransferUpdate>();

        let mut sftp_states = HashMap::new();
        sftp_states.insert("session-manager".to_string(), SftpState::new());

        (
            Self {
                sessions: SessionManager::new(),
                platform: PlatformServices::new(),
                tabs: vec![sessions_tab],
                active_tab: 0,
                main_window: Some(main_window),
                settings_process: None,
                active_view: ActiveView::SessionManager,
                saved_sessions,
                session_storage: storage,
                settings_storage,
                terminal_font_size: app_settings.terminal_font_size,
                app_settings,
                use_gpu_renderer,
                editing_session: None,
                // Form defaults
                form_name: String::new(),
                form_host: String::new(),
                form_port: "22".to_string(),
                form_username: String::new(),
                form_password: String::new(),
                form_key_id: String::new(),
                form_key_passphrase: String::new(),
                auth_method_password: true,
                validation_error: None,
                session_search_query: String::new(),
                show_password: false,
                connection_test_status: ConnectionTestStatus::Idle,
                saved_key_menu_open: false,
                session_dialog_tab: SessionDialogTab::General,
                port_forward_session_id: None,
                port_forward_local_host: "127.0.0.1".to_string(),
                port_forward_local_port: String::new(),
                port_forward_remote_host: String::new(),
                port_forward_remote_port: String::new(),
                port_forward_direction: PortForwardDirection::Local,
                port_forward_error: None,
                port_forward_statuses: HashMap::new(),
                window_width: 1024, // Default assumption
                window_height: 768,
                last_error: None,
                show_quick_connect: false,
                quick_connect_query: String::new(),
                session_menu_open: None,
                ime_buffer: String::new(),
                ime_input_id: iced::widget::Id::new("terminal-ime-input"),
                ime_focused: false,
                last_ime_focus_check: std::time::Instant::now(),
                ime_preedit: String::new(),
                ime_ignore_next_input: false,
                pending_resize: None,
                last_terminal_tab: 0,
                sftp_panel_open: false,
                sftp_panel_width: 520.0,
                sftp_panel_initialized: false,
                port_forward_panel_open: false,
                port_forward_panel_width: 420.0,
                port_forward_panel_initialized: false,
                port_forward_dragging: false,
                sftp_dragging: false,
                sftp_file_dragging: None,
                sftp_drag_position: None,
                sftp_hovered_file: None,
                sftp_transfer_tx,
                sftp_transfer_rx: Arc::new(Mutex::new(sftp_transfer_rx)),
                sftp_max_concurrent: 2,
                sftp_rename_input_id: iced::widget::Id::new("sftp-rename-input"),
                sftp_states,
            },
            open_task.map(Message::WindowOpened), // Open the main window
        )
    }

    pub fn title(&self, _window: iced::window::Id) -> String {
        if self.active_tab == 0 {
            "Rivett - Sessions".to_string()
        } else {
            format!("Rivett - {}", self.tabs[self.active_tab].title)
        }
    }

    pub fn run(settings: Settings) -> iced::Result {
        iced::daemon(App::new, App::update, App::view)
            .title(App::title)
            .theme(|app: &App, _| match app.app_settings.theme {
                ThemeMode::Dark => Theme::Dark,
                ThemeMode::Light => Theme::Light,
            })
            .subscription(App::subscription)
            .settings(settings)
            .run()
    }

    // Old subscription removed

    // Add separate timer subscription method if needed, or combine:

    pub(in crate::ui) fn sftp_key_for_tab(&self, tab_index: usize) -> Option<&str> {
        self.tabs
            .get(tab_index)
            .and_then(|tab| tab.sftp_key.as_deref())
    }

    pub(in crate::ui) fn sftp_state_for_tab(&self, tab_index: usize) -> Option<&SftpState> {
        let key = self.sftp_key_for_tab(tab_index)?;
        self.sftp_states.get(key)
    }

    pub(in crate::ui) fn sftp_state_for_tab_mut(
        &mut self,
        tab_index: usize,
    ) -> Option<&mut SftpState> {
        let key = self.sftp_key_for_tab(tab_index)?.to_string();
        Some(self.sftp_states.entry(key).or_insert_with(SftpState::new))
    }
}
