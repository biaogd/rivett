#[cfg(target_os = "macos")]
mod macos_menu;

#[derive(Debug, Default)]
pub struct PlatformServices;

impl PlatformServices {
    pub fn new() -> Self {
        Self
    }
}

pub fn setup_macos_menu() {
    #[cfg(target_os = "macos")]
    macos_menu::setup();
}

pub fn maybe_setup_macos_menu() {
    #[cfg(target_os = "macos")]
    macos_menu::maybe_install();
}

pub fn take_settings_request() -> bool {
    #[cfg(target_os = "macos")]
    {
        return macos_menu::take_settings_request();
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn default_terminal_font_family() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Menlo"
    }
    #[cfg(target_os = "linux")]
    {
        "DejaVu Sans Mono"
    }
    #[cfg(target_os = "windows")]
    {
        "Consolas"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "monospace"
    }
}
