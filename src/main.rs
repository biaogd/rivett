mod core;
mod platform;
mod settings;
mod settings_app;
mod session;
mod ssh;
mod terminal;
mod ui;

fn main() -> iced::Result {
    let is_settings = std::env::args().any(|arg| arg == "--settings");
    if is_settings {
        return settings_app::run();
    }

    platform::setup_macos_menu();
    ui::App::run(iced::Settings::default())
}
