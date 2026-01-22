use iced::{Settings, Task, Theme};

use crate::core::SessionManager;
use crate::platform::PlatformServices;
use crate::settings::{AppSettings, SettingsStorage};
use crate::session::{SessionConfig, SessionStorage};
use super::message::{ActiveView, Message};
use super::state::SessionTab;

#[derive(Debug)]
pub struct App {
    #[allow(dead_code)]
    pub(in crate::ui) sessions: SessionManager,
    #[allow(dead_code)]
    pub(in crate::ui) platform: PlatformServices,
    pub(in crate::ui) tabs: Vec<SessionTab>,
    pub(in crate::ui) active_tab: usize,
    pub(in crate::ui) show_menu: bool,
    pub(in crate::ui) main_window: Option<iced::window::Id>,
    pub(in crate::ui) settings_process: Option<std::process::Child>,
    // Session management
    pub(in crate::ui) active_view: ActiveView,
    pub(in crate::ui) saved_sessions: Vec<SessionConfig>,
    pub(in crate::ui) session_storage: SessionStorage,
    pub(in crate::ui) settings_storage: SettingsStorage,
    pub(in crate::ui) app_settings: AppSettings,
    pub(in crate::ui) terminal_font_size: f32,
    pub(in crate::ui) editing_session: Option<SessionConfig>,
    // Form state
    pub(in crate::ui) form_name: String,
    pub(in crate::ui) form_host: String,
    pub(in crate::ui) form_port: String,
    pub(in crate::ui) form_username: String,
    pub(in crate::ui) form_password: String,
    pub(in crate::ui) auth_method_password: bool,
    pub(in crate::ui) validation_error: Option<String>,
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
        let sessions_tab = SessionTab::new("Sessions");

        let (main_window, open_task) = iced::window::open(iced::window::Settings::default());

        (
            Self {
                sessions: SessionManager::new(),
                platform: PlatformServices::new(),
                tabs: vec![sessions_tab],
                active_tab: 0,
                show_menu: true,
                main_window: Some(main_window),
                settings_process: None,
                active_view: ActiveView::SessionManager,
                saved_sessions,
                session_storage: storage,
                settings_storage,
                terminal_font_size: app_settings.terminal_font_size,
                app_settings,
                editing_session: None,
                // Form defaults
                form_name: String::new(),
                form_host: String::new(),
                form_port: "22".to_string(),
                form_username: String::new(),
                form_password: String::new(),
                auth_method_password: true,
                validation_error: None,
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
            },
            open_task.map(Message::WindowOpened), // Open the main window
        )
    }

    pub fn title(&self, _window: iced::window::Id) -> String {
        if self.active_tab == 0 {
            "SSH GUI - Sessions".to_string()
        } else {
            format!("SSH GUI - {}", self.tabs[self.active_tab].title)
        }
    }


    pub fn run(settings: Settings) -> iced::Result {
        iced::daemon(App::new, App::update, App::view)
            .title(App::title)
            .theme(|_: &App, _| Theme::Light)
            .subscription(App::subscription)
            .settings(settings)
            .run()
    }

    // Old subscription removed

    // Add separate timer subscription method if needed, or combine:
}
