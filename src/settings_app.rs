use crate::settings::{AppSettings, SettingsStorage};
use crate::ui::style as ui_style;
use iced::widget::{column, container, text};
use iced::widget::{button, row};
use iced::{Alignment, Element, Length, Settings, Subscription, Theme};

#[cfg(target_os = "macos")]
fn set_accessory_activation_policy() {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::MainThreadMarker;

    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    }
}

#[cfg(not(target_os = "macos"))]
fn set_accessory_activation_policy() {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    General,
    Terminal,
}

#[derive(Debug)]
struct SettingsApp {
    activation_set: bool,
    storage: SettingsStorage,
    settings: AppSettings,
    tab: SettingsTab,
    parent_pid: Option<u32>,
}

#[derive(Debug, Clone)]
enum Message {
    Init,
    SelectTab(SettingsTab),
    FontSizeDecrease,
    FontSizeIncrease,
    Tick,
}

impl SettingsApp {
    fn new() -> (Self, iced::Task<Message>) {
        let storage = SettingsStorage::new();
        let settings = storage.load_settings().unwrap_or_default();
        let parent_pid = read_parent_pid();
        let app = Self {
            activation_set: false,
            storage,
            settings,
            tab: SettingsTab::Terminal,
            parent_pid,
        };
        (app, iced::Task::done(Message::Init))
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        if matches!(message, Message::Init) && !self.activation_set {
            set_accessory_activation_policy();
            self.activation_set = true;
        }

        match message {
            Message::SelectTab(tab) => {
                self.tab = tab;
            }
            Message::FontSizeDecrease => {
                let next = (self.settings.terminal_font_size - 1.0).max(8.0);
                self.update_font_size(next);
            }
            Message::FontSizeIncrease => {
                let next = (self.settings.terminal_font_size + 1.0).min(24.0);
                self.update_font_size(next);
            }
            Message::Tick => {
                if let Some(pid) = self.parent_pid {
                    if !is_parent_alive(pid) {
                        return iced::exit();
                    }
                }
            }
            Message::Init => {}
        }
        iced::Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.parent_pid.is_some() {
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = column![
            container("").height(10.0),
            tab_button("General", self.tab == SettingsTab::General, SettingsTab::General),
            container("").height(4.0),
            tab_button("Terminal", self.tab == SettingsTab::Terminal, SettingsTab::Terminal),
        ]
        .spacing(0);

        let content = match self.tab {
            SettingsTab::General => {
                column![text("No settings yet.")
                    .size(12)
                    .style(ui_style::muted_text),]
                .spacing(6)
            }
            SettingsTab::Terminal => {
                let size = self.settings.terminal_font_size.round() as i32;
                column![
                    row![
                        text("Font Size")
                            .size(12)
                            .style(ui_style::muted_text),
                        container("").width(Length::Fill),
                        button(text("-").size(14))
                            .padding([4, 10])
                            .style(ui_style::secondary_button_style)
                            .on_press(Message::FontSizeDecrease),
                        text(format!("{}", size))
                            .size(12)
                            .style(ui_style::header_text),
                        button(text("+").size(14))
                            .padding([4, 10])
                            .style(ui_style::secondary_button_style)
                            .on_press(Message::FontSizeIncrease),
                    ]
                    .align_y(Alignment::Center),
                ]
                .spacing(12)
            }
        };

        let sidebar = container(sidebar)
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .padding(12)
            .style(ui_style::dropdown_menu);

        let content = container(content)
            .width(Length::Fill)
            .height(Length::Fill);

        let layout = row![sidebar, content].spacing(0);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::app_background)
            .into()
    }

    fn update_font_size(&mut self, size: f32) {
        if (self.settings.terminal_font_size - size).abs() < f32::EPSILON {
            return;
        }
        self.settings.terminal_font_size = size;
        if let Err(e) = self.storage.save_settings(&self.settings) {
            eprintln!("Failed to save settings: {}", e);
        }
    }
}

pub fn run() -> iced::Result {
    set_accessory_activation_policy();
    iced::application(SettingsApp::new, SettingsApp::update, SettingsApp::view)
        .title(|_: &SettingsApp| "Settings".to_string())
        .theme(|_: &SettingsApp| Theme::Light)
        .settings(Settings::default())
        .window_size((520.0, 360.0))
        .subscription(SettingsApp::subscription)
        .run()
}

fn tab_button(label: &str, active: bool, tab: SettingsTab) -> iced::Element<'_, Message> {
    let style = if active {
        ui_style::sidebar_button_active
    } else {
        ui_style::sidebar_button_inactive
    };

    button(text(label).size(12))
        .padding([8, 12])
        .width(Length::Fill)
        .style(style)
        .on_press(Message::SelectTab(tab))
        .into()
}

fn read_parent_pid() -> Option<u32> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--parent-pid" {
            return args.next()?.parse::<u32>().ok();
        }
        if let Some(value) = arg.strip_prefix("--parent-pid=") {
            return value.parse::<u32>().ok();
        }
    }
    None
}

#[cfg(unix)]
fn is_parent_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as i32, 0) };
    if result == 0 {
        return true;
    }
    let err = std::io::Error::last_os_error();
    err.raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
fn is_parent_alive(_pid: u32) -> bool {
    true
}
