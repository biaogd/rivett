use crate::settings::{AppSettings, SettingsStorage};
use crate::ui::style as ui_style;
use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input,
};
use iced::{Alignment, Element, Length, Settings, Subscription, Theme};
use std::fs;
use std::path::Path;

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
    Keys,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddKeyMode {
    File,
    Paste,
}

#[derive(Debug)]
struct SettingsApp {
    activation_set: bool,
    storage: SettingsStorage,
    settings: AppSettings,
    tab: SettingsTab,
    parent_pid: Option<u32>,
    font_size_input: String,
    editing_key: Option<usize>,
    editing_key_name: String,
    key_status: Option<String>,
    adding_key: bool,
    adding_key_mode: AddKeyMode,
    adding_key_name: String,
    adding_key_path: String,
    adding_key_type: String,
    adding_key_paste: text_editor::Content,
}

#[derive(Debug, Clone)]
enum Message {
    Init,
    SelectTab(SettingsTab),
    FontSizeDecrease,
    FontSizeIncrease,
    FontSizeInputChanged(String),
    FontSizeInputSubmit,
    AddExistingKey,
    AddKeyNameChanged(String),
    AddKeyPathChanged(String),
    AddKeyTypeChanged(String),
    AddKeyPasteAction(text_editor::Action),
    SetAddKeyMode(AddKeyMode),
    AddKeyPickFile,
    AddKeySave,
    AddKeyCancel,
    RefreshKeys,
    EditKeyStart(usize),
    EditKeyNameChanged(String),
    EditKeySave,
    EditKeyCancel,
    DeleteKey(usize),
    TestKey(usize),
    SetDefaultKey(usize),
    Tick,
}

impl SettingsApp {
    fn new() -> (Self, iced::Task<Message>) {
        let storage = SettingsStorage::new();
        let settings = storage.load_settings().unwrap_or_default();
        let font_size_input = format!("{}", settings.terminal_font_size.round() as i32);
        let parent_pid = read_parent_pid();
        let app = Self {
            activation_set: false,
            storage,
            settings,
            tab: SettingsTab::Terminal,
            parent_pid,
            font_size_input,
            editing_key: None,
            editing_key_name: String::new(),
            key_status: None,
            adding_key: false,
            adding_key_mode: AddKeyMode::File,
            adding_key_name: String::new(),
            adding_key_path: String::new(),
            adding_key_type: String::new(),
            adding_key_paste: text_editor::Content::new(),
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
                self.sync_font_size_input();
            }
            Message::FontSizeIncrease => {
                let next = (self.settings.terminal_font_size + 1.0).min(24.0);
                self.update_font_size(next);
                self.sync_font_size_input();
            }
            Message::FontSizeInputChanged(value) => {
                self.font_size_input = value;
                if let Ok(parsed) = self.font_size_input.trim().parse::<f32>() {
                    if (8.0..=24.0).contains(&parsed) {
                        self.update_font_size(parsed.round());
                        self.sync_font_size_input();
                    }
                }
            }
            Message::FontSizeInputSubmit => {
                if let Ok(parsed) = self.font_size_input.trim().parse::<f32>() {
                    let clamped = parsed.clamp(8.0, 24.0).round();
                    self.update_font_size(clamped);
                    self.sync_font_size_input();
                } else {
                    self.sync_font_size_input();
                }
            }
            Message::Tick => {
                if let Some(pid) = self.parent_pid {
                    if !is_parent_alive(pid) {
                        return iced::exit();
                    }
                }
            }
            Message::AddExistingKey => {
                self.adding_key = true;
                self.adding_key_mode = AddKeyMode::File;
                self.key_status = None;
            }
            Message::SetAddKeyMode(mode) => {
                self.adding_key_mode = mode;
            }
            Message::AddKeyNameChanged(value) => {
                self.adding_key_name = value;
            }
            Message::AddKeyPathChanged(value) => {
                self.adding_key_path = value;
            }
            Message::AddKeyTypeChanged(value) => {
                self.adding_key_type = value;
            }
            Message::AddKeyPasteAction(action) => {
                self.adding_key_paste.perform(action);
            }
            Message::AddKeyPickFile => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    let path_string = path.display().to_string();
                    self.adding_key_path = path_string.clone();
                    if self.adding_key_name.trim().is_empty() {
                        self.adding_key_name =
                            normalize_key_name(&self.adding_key_name, &path_string);
                    }
                    match fs::read_to_string(&path_string) {
                        Ok(contents) => match parse_key_metadata(&contents) {
                            Ok((key_type, fingerprint)) => {
                                self.adding_key_type = key_type;
                                self.key_status =
                                    Some(format!("Fingerprint: {}", short_fingerprint(&fingerprint)));
                            }
                            Err(err) => {
                                self.key_status = Some(err);
                            }
                        },
                        Err(err) => {
                            self.key_status = Some(format!("Failed to read key: {}", err));
                        }
                    }
                }
            }
            Message::AddKeySave => {
                let name = normalize_key_name(&self.adding_key_name, &self.adding_key_path);
                let path = self.adding_key_path.trim().to_string();
                let pasted = self.adding_key_paste.text();
                let has_paste = !pasted.trim().is_empty();
                let can_save = match self.adding_key_mode {
                    AddKeyMode::File => !path.is_empty(),
                    AddKeyMode::Paste => has_paste,
                };
                if can_save && !name.is_empty() {
                    let (key_type, fingerprint, parse_status) = match self.adding_key_mode {
                        AddKeyMode::File => match fs::read_to_string(&path) {
                            Ok(contents) => match parse_key_metadata(&contents) {
                                Ok((key_type, fingerprint)) => (key_type, fingerprint, None),
                                Err(err) => (
                                    normalize_key_type(&self.adding_key_type),
                                    String::new(),
                                    Some(err),
                                ),
                            },
                            Err(err) => (
                                normalize_key_type(&self.adding_key_type),
                                String::new(),
                                Some(format!("Failed to read key: {}", err)),
                            ),
                        },
                        AddKeyMode::Paste => match parse_key_metadata(&pasted) {
                            Ok((key_type, fingerprint)) => (key_type, fingerprint, None),
                            Err(err) => (
                                normalize_key_type(&self.adding_key_type),
                                String::new(),
                                Some(err),
                            ),
                        },
                    };
                    let is_default = self.settings.ssh_keys.is_empty();
                    let stored_path = match self.adding_key_mode {
                        AddKeyMode::File => path,
                        AddKeyMode::Paste => "<pasted>".to_string(),
                    };
                    self.settings.ssh_keys.push(crate::settings::SshKeyEntry {
                        name: name.clone(),
                        path: stored_path,
                        key_type,
                        fingerprint,
                        is_default,
                        last_used: None,
                    });
                    self.persist_settings();
                    self.key_status = Some(match (self.adding_key_mode, parse_status) {
                        (_, Some(err)) => err,
                        (AddKeyMode::File, None) => format!("Added key \"{}\".", name),
                        (AddKeyMode::Paste, None) => {
                            format!("Added key \"{}\" (pasted content not stored yet).", name)
                        }
                    });
                    self.adding_key = false;
                    self.adding_key_name.clear();
                    self.adding_key_path.clear();
                    self.adding_key_type.clear();
                    self.adding_key_paste = text_editor::Content::new();
                }
            }
            Message::AddKeyCancel => {
                self.adding_key = false;
                self.adding_key_name.clear();
                self.adding_key_path.clear();
                self.adding_key_type.clear();
                self.adding_key_paste = text_editor::Content::new();
            }
            Message::RefreshKeys => {}
            Message::EditKeyStart(index) => {
                if let Some(entry) = self.settings.ssh_keys.get(index) {
                    self.editing_key = Some(index);
                    self.editing_key_name = entry.name.clone();
                }
            }
            Message::EditKeyNameChanged(value) => {
                self.editing_key_name = value;
            }
            Message::EditKeySave => {
                if let Some(index) = self.editing_key.take() {
                    let trimmed = self.editing_key_name.trim();
                    if !trimmed.is_empty() {
                        if let Some(entry) = self.settings.ssh_keys.get_mut(index) {
                            entry.name = trimmed.to_string();
                        }
                        self.persist_settings();
                    }
                }
            }
            Message::EditKeyCancel => {
                self.editing_key = None;
                self.editing_key_name.clear();
            }
            Message::DeleteKey(index) => {
                if index < self.settings.ssh_keys.len() {
                    let was_default = self.settings.ssh_keys[index].is_default;
                    self.settings.ssh_keys.remove(index);
                    if was_default {
                        if let Some(first) = self.settings.ssh_keys.first_mut() {
                            first.is_default = true;
                        }
                    }
                    self.persist_settings();
                }
            }
            Message::TestKey(index) => {
                if let Some(entry) = self.settings.ssh_keys.get_mut(index) {
                    entry.last_used = Some(current_timestamp());
                    self.key_status = Some(format!("Tested key \"{}\".", entry.name));
                    self.persist_settings();
                }
            }
            Message::SetDefaultKey(index) => {
                if index < self.settings.ssh_keys.len() {
                    for (i, entry) in self.settings.ssh_keys.iter_mut().enumerate() {
                        entry.is_default = i == index;
                    }
                    self.persist_settings();
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
            container("").height(4.0),
            tab_button("Keys", self.tab == SettingsTab::Keys, SettingsTab::Keys),
        ]
        .spacing(0);

        let content = match self.tab {
            SettingsTab::General => {
                column![text("No settings yet.")
                    .size(13)
                    .style(ui_style::muted_text),]
                .spacing(6)
            }
            SettingsTab::Terminal => {
                let header = column![
                    text("Terminal").size(14),
                    text("Adjust font and display settings for the terminal.")
                        .size(13)
                        .style(ui_style::muted_text),
                ]
                .spacing(4);

                let font_row = row![
                    text("Font Size")
                        .size(13),
                    container("").width(Length::Fill),
                    text_input("", &self.font_size_input)
                        .on_input(Message::FontSizeInputChanged)
                        .on_submit(Message::FontSizeInputSubmit)
                        .padding([4, 6])
                        .size(13)
                        .style(ui_style::dialog_input)
                        .width(Length::Fixed(40.0)),
                    column![
                        button(text("▲").size(10))
                            .padding([0, 6])
                            .style(ui_style::icon_button)
                            .on_press(Message::FontSizeIncrease),
                        button(text("▼").size(10))
                            .padding([0, 6])
                            .style(ui_style::icon_button)
                            .on_press(Message::FontSizeDecrease),
                    ]
                    .spacing(1),
                ]
                .align_y(Alignment::Center);

                let panel = container(
                    column![
                        container(font_row).padding([8, 10]),
                    ]
                    .spacing(6),
                )
                .style(ui_style::panel);

                column![header, panel].spacing(16)
            }
            SettingsTab::Keys => {
                let header = column![
                    text("SSH Keys").size(14),
                    text("Manage your SSH private keys for authentication.")
                        .size(13)
                        .style(ui_style::muted_text),
                ]
                .spacing(4);

                let status_line = self
                    .key_status
                    .as_ref()
                    .map(|status| text(status).size(13).style(ui_style::muted_text));

                let list_header = row![
                    text("Name")
                        .size(12)
                        .style(ui_style::muted_text)
                        .width(Length::FillPortion(4)),
                    text("Type")
                        .size(12)
                        .style(ui_style::muted_text)
                        .width(Length::FillPortion(2)),
                    text("Fingerprint")
                        .size(12)
                        .style(ui_style::muted_text)
                        .width(Length::FillPortion(3)),
                    text("Default")
                        .size(12)
                        .style(ui_style::muted_text)
                        .width(Length::Fixed(70.0)),
                    text("Actions")
                        .size(12)
                        .style(ui_style::muted_text)
                        .width(Length::Fixed(120.0)),
                ]
                .align_y(Alignment::Center);

                let list_rows = if self.settings.ssh_keys.is_empty() {
                    column![
                        text("No SSH keys added yet.")
                            .size(13)
                            .style(ui_style::muted_text),
                        text("Add a key to enable key-based authentication.")
                            .size(13)
                            .style(ui_style::muted_text),
                    ]
                    .spacing(4)
                } else {
                    let mut rows = column![];
                    for (index, entry) in self.settings.ssh_keys.iter().enumerate() {
                        let fingerprint = short_fingerprint(&entry.fingerprint);
                        let default_cell: Element<'_, Message> = if entry.is_default {
                            text("Default")
                                .size(13)
                                .style(ui_style::muted_text)
                                .into()
                        } else {
                            button(text("Set").size(12))
                                .padding([2, 4])
                                .style(ui_style::action_button)
                                .on_press(Message::SetDefaultKey(index))
                                .into()
                        };
                        let name_cell: Element<'_, Message> =
                            if self.editing_key == Some(index) {
                                text_input("Name", &self.editing_key_name)
                                    .on_input(Message::EditKeyNameChanged)
                                    .padding([2, 6])
                                    .size(13)
                                    .style(ui_style::dialog_input)
                                    .into()
                            } else {
                                text(&entry.name).size(13).into()
                            };
                        let actions: Element<'_, Message> = if self.editing_key == Some(index) {
                            row![
                                button(text("Save").size(12))
                                    .padding([2, 4])
                                    .style(ui_style::action_button)
                                    .on_press(Message::EditKeySave),
                                button(text("Cancel").size(12))
                                    .padding([2, 4])
                                    .style(ui_style::action_button)
                                    .on_press(Message::EditKeyCancel),
                            ]
                            .spacing(6)
                            .into()
                        } else {
                            row![
                                button(text("Edit").size(12))
                                    .padding([2, 4])
                                    .style(ui_style::action_button)
                                    .on_press(Message::EditKeyStart(index)),
                                button(text("Delete").size(12))
                                    .padding([2, 4])
                                    .style(ui_style::action_button_destructive)
                                    .on_press(Message::DeleteKey(index)),
                                button(text("Test").size(12))
                                    .padding([2, 4])
                                    .style(ui_style::action_button)
                                    .on_press(Message::TestKey(index)),
                            ]
                            .spacing(6)
                            .into()
                        };
                        rows = rows.push(
                            row![
                                container(name_cell).width(Length::FillPortion(4)),
                                text(&entry.key_type)
                                    .size(12)
                                    .style(ui_style::muted_text)
                                    .width(Length::FillPortion(2)),
                                text(fingerprint)
                                    .size(12)
                                    .style(ui_style::muted_text)
                                    .width(Length::FillPortion(3)),
                                container(default_cell).width(Length::Fixed(70.0)),
                                container(actions).width(Length::Fixed(120.0)),
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        );
                    }
                    rows.spacing(4)
                };

                let list = container(
                    column![
                        container(list_header)
                            .padding([6, 10])
                            .style(ui_style::table_header),
                        container("")
                            .height(1.0)
                            .width(Length::Fill)
                            .style(ui_style::divider),
                        container(
                            scrollable(list_rows)
                                .height(Length::Fill)
                                .style(ui_style::scrollable_style)
                                .direction(ui_style::thin_scrollbar()),
                        )
                        .padding([6, 6]),
                    ]
                    .spacing(6)
                    .height(Length::Fill),
                )
                .style(ui_style::panel)
                .height(Length::Fill);

                let actions = row![
                    button(text("+ Add Existing Key").size(12))
                        .padding([4, 10])
                        .style(ui_style::secondary_button_style)
                        .on_press(Message::AddExistingKey),
                    button(text("Refresh").size(12))
                        .padding([4, 10])
                        .style(ui_style::secondary_button_style)
                        .on_press(Message::RefreshKeys),
                ]
                .spacing(10)
                .align_y(Alignment::Center);

                let mut content = column![header, list, actions].spacing(16);
                if let Some(line) = status_line {
                    content = content.push(line);
                }
                content.height(Length::Fill)
            }
        };

        let sidebar = container(sidebar)
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .padding(12)
            .style(ui_style::dropdown_menu);

        let content = container(content)
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill);

        let layout = row![sidebar, content].spacing(0);

        let base: Element<'_, Message> = container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ui_style::app_background)
            .into();

        let overlay: Element<'_, Message> = if self.adding_key {
            let mode_row = row![
                button(text("From File").size(13))
                    .padding([4, 10])
                    .style(ui_style::menu_button(self.adding_key_mode == AddKeyMode::File))
                    .on_press(Message::SetAddKeyMode(AddKeyMode::File)),
                button(text("Paste Key").size(13))
                    .padding([4, 10])
                    .style(ui_style::menu_button(self.adding_key_mode == AddKeyMode::Paste))
                    .on_press(Message::SetAddKeyMode(AddKeyMode::Paste)),
            ]
            .spacing(6);

            let file_row = row![
                text("Path").size(13).style(ui_style::muted_text),
                container("").width(Length::Fill),
                text_input("~/.ssh/id_ed25519", &self.adding_key_path)
                    .on_input(Message::AddKeyPathChanged)
                    .padding([4, 8])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fixed(220.0)),
                button(text("Choose...").size(13))
                    .padding([4, 10])
                    .style(ui_style::secondary_button_style)
                    .on_press(Message::AddKeyPickFile),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let paste_editor = text_editor(&self.adding_key_paste)
                .placeholder("Paste private key here")
                .on_action(Message::AddKeyPasteAction)
                .height(Length::Fixed(120.0));

            let source_row: Element<'_, Message> = match self.adding_key_mode {
                AddKeyMode::File => file_row.into(),
                AddKeyMode::Paste => paste_editor.into(),
            };

            let form = column![
                text("Add SSH Key").size(14),
                mode_row,
                row![
                    text("Name").size(13).style(ui_style::muted_text),
                    container("").width(Length::Fill),
                    text_input("Key name", &self.adding_key_name)
                        .on_input(Message::AddKeyNameChanged)
                        .padding([4, 8])
                        .size(13)
                        .style(ui_style::dialog_input)
                        .width(Length::Fixed(260.0)),
                ]
                .align_y(Alignment::Center),
                row![
                    text("Type").size(13).style(ui_style::muted_text),
                    container("").width(Length::Fill),
                    text_input("ed25519 / rsa", &self.adding_key_type)
                        .on_input(Message::AddKeyTypeChanged)
                        .padding([4, 8])
                        .size(13)
                        .style(ui_style::dialog_input)
                        .width(Length::Fixed(260.0)),
                ]
                .align_y(Alignment::Center),
                source_row,
                row![
                    container("").width(Length::Fill),
                    button(text("Cancel").size(13))
                        .padding([4, 10])
                        .style(ui_style::secondary_button_style)
                        .on_press(Message::AddKeyCancel),
                    button(text("Add").size(13))
                        .padding([4, 10])
                        .style(ui_style::secondary_button_style)
                        .on_press(Message::AddKeySave),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(10);

            let dialog = container(form)
                .padding(16)
                .width(Length::Fixed(520.0))
                .style(ui_style::dialog_container);

            iced::widget::mouse_area(
                container(dialog)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(ui_style::modal_backdrop_container)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .on_press(Message::AddKeyCancel)
            .into()
        } else {
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        iced::widget::stack![base, overlay].into()
    }

    fn update_font_size(&mut self, size: f32) {
        if (self.settings.terminal_font_size - size).abs() < f32::EPSILON {
            return;
        }
        self.settings.terminal_font_size = size;
        self.persist_settings();
    }

    fn sync_font_size_input(&mut self) {
        self.font_size_input = format!("{}", self.settings.terminal_font_size.round() as i32);
    }

    fn persist_settings(&self) {
        if let Err(e) = self.storage.save_settings(&self.settings) {
            eprintln!("Failed to save settings: {}", e);
        }
    }
}

fn short_fingerprint(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() <= 14 {
        return trimmed.to_string();
    }
    let head = &trimmed[..12];
    format!("{}...", head)
}

fn current_timestamp() -> String {
    let now = chrono::Local::now();
    now.format("%Y-%m-%d %H:%M").to_string()
}

fn normalize_key_name(name: &str, path: &str) -> String {
    let trimmed = name.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    let path_trimmed = path.trim();
    if path_trimmed.is_empty() {
        return String::new();
    }
    Path::new(path_trimmed)
        .file_name()
        .and_then(|os| os.to_str())
        .unwrap_or(path_trimmed)
        .to_string()
}

fn normalize_key_type(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_key_metadata(secret: &str) -> Result<(String, String), String> {
    let key = russh_keys::decode_secret_key(secret, None)
        .map_err(|err| format!("Failed to parse key: {}", err))?;
    let key_type = display_key_type(key.algorithm().as_str());
    let fingerprint = key.fingerprint(russh_keys::HashAlg::Sha256).to_string();
    Ok((key_type, fingerprint))
}

fn display_key_type(algorithm: &str) -> String {
    match algorithm {
        "ssh-ed25519" => "ED25519".to_string(),
        "ssh-rsa" => "RSA".to_string(),
        "ecdsa-sha2-nistp256" => "ECDSA P-256".to_string(),
        "ecdsa-sha2-nistp384" => "ECDSA P-384".to_string(),
        "ecdsa-sha2-nistp521" => "ECDSA P-521".to_string(),
        "sk-ecdsa-sha2-nistp256@openssh.com" => "ECDSA-SK".to_string(),
        "sk-ssh-ed25519@openssh.com" => "ED25519-SK".to_string(),
        other => other.to_string(),
    }
}

pub fn run() -> iced::Result {
    set_accessory_activation_policy();
    iced::application(SettingsApp::new, SettingsApp::update, SettingsApp::view)
        .title(|_: &SettingsApp| "Settings".to_string())
        .theme(|_: &SettingsApp| Theme::Light)
        .settings(Settings::default())
        .window_size((720.0, 420.0))
        .subscription(SettingsApp::subscription)
        .run()
}

fn tab_button(label: &str, active: bool, tab: SettingsTab) -> iced::Element<'_, Message> {
    button(text(label).size(13))
        .padding([8, 12])
        .width(Length::Fill)
        .style(ui_style::menu_button(active))
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
