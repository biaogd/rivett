use crate::settings::{AppSettings, SettingsStorage};
use crate::ui::style as ui_style;
use iced::widget::{column, container, text};
use iced::widget::{button, row};
use iced::{Alignment, Element, Length, Settings, Theme};

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
}

#[derive(Debug, Clone)]
enum Message {
    Init,
    SelectTab(SettingsTab),
    FontSizeDecrease,
    FontSizeIncrease,
}

impl SettingsApp {
    fn new() -> (Self, iced::Task<Message>) {
        let storage = SettingsStorage::new();
        let settings = storage.load_settings().unwrap_or_default();
        let app = Self {
            activation_set: false,
            storage,
            settings,
            tab: SettingsTab::Terminal,
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
            Message::Init => {}
        }
        iced::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let tab_bar = row![
            tab_button("General", self.tab == SettingsTab::General, SettingsTab::General),
            tab_button("Terminal", self.tab == SettingsTab::Terminal, SettingsTab::Terminal),
        ]
        .spacing(8);

        let content = match self.tab {
            SettingsTab::General => column![text("General").size(14).style(ui_style::header_text)]
                .spacing(8),
            SettingsTab::Terminal => {
                let size = self.settings.terminal_font_size.round() as i32;
                column![
                    text("Terminal").size(14).style(ui_style::header_text),
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

        let content = column![text("Settings").size(16).style(ui_style::header_text), tab_bar, content]
            .spacing(12);

        container(content)
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::panel)
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
        .run()
}

fn tab_button(label: &str, active: bool, tab: SettingsTab) -> iced::Element<'_, Message> {
    let style = if active {
        ui_style::sidebar_button_active
    } else {
        ui_style::sidebar_button_inactive
    };

    button(text(label).size(12))
        .padding([6, 12])
        .style(style)
        .on_press(Message::SelectTab(tab))
        .into()
}
