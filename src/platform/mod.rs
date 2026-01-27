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

pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(url);
        cmd
    };

    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(url);
        cmd
    };

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", "", url]);
        cmd
    };

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let mut cmd = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(url);
        cmd
    };

    cmd.status()
        .map_err(|e| e.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err("Failed to open URL".to_string())
            }
        })
}
