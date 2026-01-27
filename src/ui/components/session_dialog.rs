use crate::session::SessionConfig;
use crate::settings::SshKeyEntry;
use crate::ui::message::SessionDialogTab;
use crate::ui::Message;
use crate::ui::style as ui_style;
use crate::ui::state::ConnectionTestStatus;
use iced::widget::{button, column, container, mouse_area, row, stack, text, text_input, Space};
use iced::{Alignment, Element, Length};

pub fn render<'a>(
    editing_session: Option<&'a SessionConfig>,
    saved_sessions: &'a [SessionConfig],
    saved_keys: &'a [SshKeyEntry],
    form_name: &'a str,
    form_host: &'a str,
    form_port: &'a str,
    form_username: &'a str,
    form_password: &'a str,
    form_key_id: &'a str,
    _form_key_passphrase: &'a str,
    auth_method_password: bool,
    show_password: bool,
    connection_test_status: &'a ConnectionTestStatus,
    saved_key_menu_open: bool,
    validation_error: Option<&'a String>,
    session_dialog_tab: SessionDialogTab,
    port_forward_local_host: &'a str,
    port_forward_local_port: &'a str,
    port_forward_remote_host: &'a str,
    port_forward_remote_port: &'a str,
    port_forward_error: Option<&'a String>,
) -> Element<'a, Message> {
    let is_new = editing_session
        .map(|s| !saved_sessions.iter().any(|saved| saved.id == s.id))
        .unwrap_or(true);

    let title = if is_new {
        "New Session"
    } else {
        "Edit Session"
    };
    let subtitle = if is_new {
        "Configure a new SSH connection to your remote server."
    } else {
        "Update your SSH connection configuration."
    };

    // Header with title and close button
    let header = row![
        column![
            text(title).size(18).style(ui_style::header_text),
            text(subtitle).size(12).style(ui_style::muted_text),
        ]
        .spacing(3),
        container("").width(Length::Fill),
        button(text("✕").size(13))
            .padding(8)
            .style(ui_style::tab_close_button)
            .on_press(Message::CancelSessionEdit),
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    let tabs = row![
        button(text("General").size(13))
            .padding([6, 12])
            .style(ui_style::dialog_tab(session_dialog_tab == SessionDialogTab::General))
            .on_press(Message::SessionDialogTabSelected(SessionDialogTab::General)),
        button(text("Port Forwarding").size(13))
            .padding([6, 12])
            .style(ui_style::dialog_tab(
                session_dialog_tab == SessionDialogTab::PortForwarding,
            ))
            .on_press(Message::SessionDialogTabSelected(
                SessionDialogTab::PortForwarding,
            )),
    ]
    .spacing(6);

    let error_banner = validation_error.map_or_else(
        || container("").height(0.0),
        |error| {
            container(
                text(format!("⚠️ {}", error))
                    .size(12)
                    .color(iced::Color::from_rgb(0.9, 0.3, 0.3)),
            )
            .padding(12)
            .width(Length::Fill)
            .style(ui_style::error_banner)
        },
    );

    // Form fields
    let auth_selector = row![
        button(text("Password").size(12))
            .padding([6, 12])
            .style(ui_style::compact_tab(auth_method_password))
            .on_press(if auth_method_password {
                Message::Ignore
            } else {
                Message::ToggleAuthMethod
            }),
        button(text("Private key").size(12))
            .padding([6, 12])
            .style(ui_style::compact_tab(!auth_method_password))
            .on_press(if auth_method_password {
                Message::ToggleAuthMethod
            } else {
                Message::Ignore
            }),
    ]
    .spacing(6);

    let auth_fields = if auth_method_password {
        let eye_icon = if show_password {
            iced::widget::svg(iced::widget::svg::Handle::from_memory(
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/eye-off.svg"))
                    .as_slice(),
            ))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
        } else {
            iced::widget::svg(iced::widget::svg::Handle::from_memory(
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/eye.svg"))
                    .as_slice(),
            ))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
        };

        column![
            text("Password").size(12).style(ui_style::muted_text),
            row![
                text_input("", form_password)
                    .on_input(Message::SessionPasswordChanged)
                    .padding([8, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .secure(!show_password)
                    .width(Length::Fill),
                button(eye_icon)
                    .padding([8, 8])
                    .style(ui_style::icon_button)
                    .on_press(Message::TogglePasswordVisibility),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(6)
    } else {
        let saved_key_section: Element<'a, Message> = if saved_keys.is_empty() {
            column![
                text("Saved key").size(12).style(ui_style::muted_text),
                text("No saved keys yet")
                    .size(12)
                    .style(ui_style::muted_text),
            ]
            .spacing(4)
            .into()
        } else {
            let selected_label = saved_keys
                .iter()
                .find(|key| key.id == form_key_id)
                .map(|key| key.name.as_str());

            let options: Vec<crate::ui::components::dropdown::DropdownOption<String>> = saved_keys
                .iter()
                .map(|key| crate::ui::components::dropdown::DropdownOption {
                    label: key.name.clone(),
                    value: key.id.clone(),
                })
                .collect();

            crate::ui::components::dropdown::render(
                "Saved key",
                "Select a saved key",
                selected_label,
                options,
                saved_key_menu_open,
                false,
                Message::ToggleSavedKeyMenu,
                Message::SessionKeyIdChanged,
                None,
            )
        };

        column![saved_key_section].spacing(6)
    };

    let general_content = column![
        column![
            text("Display name").size(12).style(ui_style::muted_text),
            text_input("Production Server 01", form_name)
                .on_input(Message::SessionNameChanged)
                .padding([8, 10])
                .size(13)
                .style(ui_style::dialog_input),
        ]
        .spacing(6),
        container("").height(12.0),
        row![
            column![
                text("Host address").size(12).style(ui_style::muted_text),
                text_input("192.168.1.1 or example.com", form_host)
                    .on_input(Message::SessionHostChanged)
                    .padding([8, 10])
                    .size(13)
                    .style(ui_style::dialog_input),
        ]
            .spacing(6)
            .width(Length::FillPortion(3)),
            container("").width(12.0),
            column![
                text("Port").size(12).style(ui_style::muted_text),
                text_input("22", form_port)
                    .on_input(Message::SessionPortChanged)
                    .padding([8, 10])
                    .size(13)
                    .style(ui_style::dialog_input)
                    .width(Length::Fixed(80.0)),
        ]
            .spacing(6)
            .width(Length::FillPortion(1)),
        ],
        container("").height(12.0),
        column![
            text("Username").size(12).style(ui_style::muted_text),
            text_input("root", form_username)
                .on_input(Message::SessionUsernameChanged)
                .padding([8, 10])
                .size(13)
                .style(ui_style::dialog_input),
        ]
        .spacing(6),
    ]
    .spacing(0);

    let auth_content = column![
        text("Authentication").size(12).style(ui_style::muted_text),
        auth_selector,
        container("").height(8.0),
        auth_fields,
    ]
    .spacing(0);

    let port_forward_content = editing_session.map_or_else(
        || {
            column![
                text("Save the session to add port forwards.")
                    .size(12)
                    .style(ui_style::muted_text),
            ]
            .into()
        },
        |session| {
            crate::ui::components::port_forward_dialog::render_inline(
                session,
                port_forward_local_host,
                port_forward_local_port,
                port_forward_remote_host,
                port_forward_remote_port,
                port_forward_error,
            )
        },
    );

    let form_content: Element<'a, Message> = match session_dialog_tab {
        SessionDialogTab::General => column![
            general_content,
            container("").height(14.0),
            auth_content
        ]
        .into(),
        SessionDialogTab::PortForwarding => port_forward_content,
    };

    // Footer with buttons
    let status_text = match connection_test_status {
        ConnectionTestStatus::Idle => None,
        ConnectionTestStatus::Testing => Some(text("Testing...").size(12).style(ui_style::muted_text)),
        ConnectionTestStatus::Success => Some(text("Connection ok").size(12)),
        ConnectionTestStatus::Failed(err) => Some(
            text(err)
                .size(12)
                .color(iced::Color::from_rgb(0.9, 0.3, 0.3)),
        ),
    };

    let test_button = match connection_test_status {
        ConnectionTestStatus::Testing => button(text("Testing...").size(12))
            .padding([8, 16])
            .style(ui_style::secondary_button_style),
        _ => button(text("Test Connection").size(12))
            .padding([8, 16])
            .style(ui_style::secondary_button_style)
            .on_press(Message::TestConnection),
    };

    let mut footer = row![test_button];
    if let Some(status) = status_text {
        footer = footer.push(status);
    }
    footer = footer
        .push(container("").width(Length::Fill))
        .push(
            button(text("Cancel").size(12))
                .padding([8, 16])
                .style(ui_style::secondary_button_style)
                .on_press(Message::CancelSessionEdit),
        )
        .push({
            let action_label = if is_new { "Create Session" } else { "Save Changes" };
            button(text(action_label).size(12))
                .padding([8, 16])
                .style(ui_style::primary_button_style)
                .on_press(Message::SaveSession)
        })
        .spacing(12)
        .align_y(Alignment::Center);

    // Assemble dialog content
    let dialog_body = column![
        header,
        container("").height(12.0),
        tabs,
        container("")
            .height(1.0)
            .width(Length::Fill)
            .style(ui_style::divider),
        container("").height(16.0),
        error_banner,
        form_content,
        container("").height(20.0),
        footer,
    ]
    .spacing(0)
    .padding(24)
    .width(Length::Fixed(560.0));

    let dialog_content: Element<'a, Message> = if saved_key_menu_open {
        let dismiss_layer = mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::CloseSavedKeyMenu);

        stack![dialog_body, dismiss_layer].into()
    } else {
        dialog_body.into()
    };

    container(dialog_content)
        .style(ui_style::dialog_container)
        .into()
}
