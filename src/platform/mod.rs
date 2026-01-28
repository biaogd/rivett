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

pub fn terminal_fallback_family() -> &'static str {
    use std::sync::OnceLock;

    static FALLBACK: OnceLock<String> = OnceLock::new();
    FALLBACK.get_or_init(|| detect_terminal_fallback()).as_str()
}

fn detect_terminal_fallback() -> String {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    #[cfg(target_os = "macos")]
    let candidates = [
        "Sarasa Mono SC",
        "Noto Sans Mono CJK SC",
        "Noto Sans Mono CJK",
        "Noto Sans Mono",
        "PingFang SC",
        "Heiti SC",
    ];

    #[cfg(target_os = "windows")]
    let candidates = [
        "Sarasa Mono SC",
        "Noto Sans Mono CJK SC",
        "Microsoft YaHei UI",
        "Microsoft YaHei",
        "SimHei",
    ];

    #[cfg(target_os = "linux")]
    let candidates = [
        "Sarasa Mono SC",
        "Noto Sans Mono CJK SC",
        "Noto Sans Mono CJK",
        "Noto Sans Mono",
        "WenQuanYi Micro Hei",
        "DejaVu Sans Mono",
    ];

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let candidates = ["sans-serif"];

    for name in candidates {
        if has_family(&db, name) {
            return name.to_string();
        }
    }

    #[cfg(target_os = "macos")]
    {
        "PingFang SC".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        "Noto Sans Mono CJK SC".to_string()
    }
    #[cfg(target_os = "windows")]
    {
        "Microsoft YaHei".to_string()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "sans-serif".to_string()
    }
}

fn has_family(db: &fontdb::Database, name: &str) -> bool {
    db.faces().any(|face| {
        face.families
            .iter()
            .any(|(family, _)| family.eq_ignore_ascii_case(name))
    })
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
