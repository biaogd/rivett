use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::style as ui_style;

#[derive(Debug, Clone)]
pub struct DropdownOption<T> {
    pub label: String,
    pub value: T,
}

pub fn render<'a, Message, T>(
    label: &'a str,
    placeholder: &'a str,
    selected_label: Option<&'a str>,
    options: Vec<DropdownOption<T>>,
    open: bool,
    disabled: bool,
    on_toggle: Message,
    on_select: impl Fn(T) -> Message + 'a,
    helper_text: Option<&'a str>,
) -> Element<'a, Message>
where
    T: Clone + 'a,
    Message: Clone + 'a,
{
    let display = selected_label.unwrap_or(placeholder);

    let mut selector = button(
        row![
            text(display).size(13),
            container("").width(Length::Fill),
            text("▾").size(12),
        ]
        .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .style(if disabled {
        ui_style::dropdown_button_disabled
    } else {
        ui_style::dropdown_button
    })
    .width(Length::Fill);
    if !disabled {
        selector = selector.on_press(on_toggle);
    }

    let menu_column = options.iter().fold(column![], |column, option| {
        column.push(
            button(text(option.label.clone()).size(13))
                .padding([6, 10])
                .style(ui_style::menu_item_button)
                .width(Length::Fill)
                .on_press(on_select(option.value.clone())),
        )
    });

    let menu_panel = container(menu_column.spacing(2))
        .padding(6)
        .style(ui_style::popover_menu);

    let anchored = crate::ui::components::anchored_menu::anchored_menu(
        selector,
        menu_panel,
        open && !options.is_empty() && !disabled,
        6.0,
    );

    let mut content = column![
        text(label).size(12).style(ui_style::muted_text),
        anchored
    ]
    .spacing(6);

    if let Some(helper) = helper_text {
        if !helper.trim().is_empty() {
            content = content.push(text(helper).size(11).style(ui_style::muted_text));
        }
    }

    content.into()
}
