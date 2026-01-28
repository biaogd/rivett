mod core;
mod platform;
mod session;
mod settings;
mod settings_app;
mod ssh;
mod terminal;
mod ui;

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter(filter)
        .init();
}

fn main() -> iced::Result {
    init_tracing();
    tracing::info!("iced renderer: {}", std::any::type_name::<iced::Renderer>());
    let is_settings = std::env::args().any(|arg| arg == "--settings");
    if is_settings {
        return settings_app::run();
    }

    platform::setup_macos_menu();
    ui::App::run(iced::Settings::default())
}
