//! Format partition modal dialog.

use iced::{Element, Length};
use iced::widget::{button, column, container, row, text, text_input, Space};
use crate::app::Message;
use crate::theme;

pub fn view<'a>(device: &'a str, confirm_text: &'a str) -> Element<'a, Message> {
    let title = text(format!("FORMAT /dev/{}", device))
        .size(18)
        .color(theme::ACCENT_DANGER);

    let warning = text(
        "⚠ THIS WILL PERMANENTLY ERASE ALL DATA ON THIS PARTITION ⚠\n\
         Type CONFIRM below to proceed.",
    )
    .size(13)
    .color(theme::ACCENT_WARNING);

    let confirm_input = text_input("Type CONFIRM to proceed…", confirm_text)
        .on_input(Message::ModalConfirmTextChanged)
        .on_submit(Message::ModalConfirm)
        .size(14)
        .width(Length::Fill);

    let can_confirm = confirm_text == "CONFIRM";

    let ok_btn = if can_confirm {
        button(text("FORMAT").size(14).color(theme::ACCENT_DANGER))
            .on_press(Message::ModalConfirm)
            .padding([8, 16])
    } else {
        button(text("FORMAT").size(14).color(theme::TEXT_SECONDARY)).padding([8, 16])
    };

    let cancel_btn = button(text("CANCEL").size(14))
        .on_press(Message::ModalCancel)
        .padding([8, 16]);

    let inner = column![
        title,
        Space::new().height(12),
        warning,
        Space::new().height(12),
        confirm_input,
        Space::new().height(12),
        row![cancel_btn, Space::new().width(8), ok_btn],
    ]
    .spacing(4)
    .padding(32)
    .width(500);

    container(inner)
        .style(|_| iced::widget::container::Style {
            background: Some(theme::BG_ELEVATED.into()),
            border: iced::Border {
                color: theme::ACCENT_DANGER,
                width: 2.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}
