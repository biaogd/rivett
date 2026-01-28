use std::process::Command;

use crate::ui::App;

impl App {
    pub(in crate::ui) fn reload_settings(&mut self) {
        let loaded = self.settings_storage.load_settings().unwrap_or_default();
        if loaded != self.app_settings {
            self.app_settings = loaded.clone();
            self.terminal_font_size = loaded.terminal_font_size;
            self.use_gpu_renderer = loaded.use_gpu_renderer;
            crate::ui::style::set_dark_mode(matches!(
                self.app_settings.theme,
                crate::settings::ThemeMode::Dark
            ));
            for tab in &mut self.tabs {
                tab.mark_full_damage();
            }
        }
    }

    pub(in crate::ui) fn open_settings_window(&mut self) {
        if let Some(child) = &mut self.settings_process {
            if let Ok(None) = child.try_wait() {
                return;
            }
        }

        let parent_pid = std::process::id().to_string();
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => {
                eprintln!("Failed to locate current executable: {}", e);
                return;
            }
        };

        if self.try_open_settings_bundle(&exe, &parent_pid) {
            self.settings_process = None;
            return;
        }

        match Command::new(exe)
            .arg("--settings")
            .arg("--parent-pid")
            .arg(parent_pid)
            .spawn()
        {
            Ok(child) => {
                self.settings_process = Some(child);
            }
            Err(e) => {
                eprintln!("Failed to open settings window: {}", e);
            }
        }
    }

    fn try_open_settings_bundle(&self, exe: &std::path::Path, parent_pid: &str) -> bool {
        #[cfg(target_os = "macos")]
        {
            if let Some(app_dir) = exe
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                let helper_app = app_dir.with_file_name("SSH GUI Settings.app");
                if helper_app.exists() {
                    let status = Command::new("open")
                        .arg("-a")
                        .arg(helper_app)
                        .arg("--args")
                        .arg("--settings")
                        .arg("--parent-pid")
                        .arg(parent_pid)
                        .status();
                    return status.map(|s| s.success()).unwrap_or(false);
                }
            }
            false
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = exe;
            false
        }
    }
}
