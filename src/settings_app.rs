use crate::ui::style as ui_style;
use iced::widget::{column, container, text};
use iced::{Element, Length, Settings, Theme};

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

#[derive(Debug, Default)]
struct SettingsApp {
    activation_set: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Init,
}

impl SettingsApp {
    fn new() -> (Self, iced::Task<Message>) {
        (
            Self {
                activation_set: false,
            },
            iced::Task::done(Message::Init),
        )
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        if matches!(message, Message::Init) && !self.activation_set {
            set_accessory_activation_policy();
            self.activation_set = true;
        }
        iced::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let content = column![
            text("Settings").size(16).style(ui_style::header_text),
            text("Settings UI is coming soon.")
                .size(12)
                .style(ui_style::muted_text),
        ]
        .spacing(8);

        container(content)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::panel)
            .into()
    }
}

pub fn run() -> iced::Result {
    set_accessory_activation_policy();
    iced::application(SettingsApp::new, SettingsApp::update, SettingsApp::view)
        .title(|_: &SettingsApp| "Settings".to_string())
        .theme(|_: &SettingsApp| Theme::Light)
        .settings(Settings::default())
        .run()
}
