mod core;
mod platform;
mod session;
mod ssh;
mod terminal;
mod ui;

fn main() -> iced::Result {
    ui::App::run(iced::Settings::default())
}
