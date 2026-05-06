//! Passphrase entry modal for LUKS / BitLocker volumes.

use iced::{Element, Length};
use iced::widget::{button, column, container, row, text, text_input, Space};
use crate::app::Message;
use crate::theme;

pub fn view<'a>(device: &'a str, passphrase: &'a str) -> Element<'a, Message> {
    let title = text(format!("UNLOCK /dev/{}", device))
        .size(18)
        .color(theme::ACCENT_PRIMARY);

    let subtitle = text("This partition is encrypted. Enter your passphrase or recovery key.")
        .size(13)
        .color(theme::TEXT_SECONDARY);

    let input = text_input("Passphrase…", passphrase)
        .on_input(Message::ModalPassphraseChanged)
        .on_submit(Message::ModalConfirm)
        .secure(true)
        .size(14)
        .width(Length::Fill);

    let ok_btn = button(text("UNLOCK").size(14).color(theme::ACCENT_PRIMARY))
        .on_press(Message::ModalConfirm)
        .padding([8, 16]);

    let cancel_btn = button(text("CANCEL").size(14))
        .on_press(Message::ModalCancel)
        .padding([8, 16]);

    let inner = column![
        title,
        Space::new().height(8),
        subtitle,
        Space::new().height(12),
        input,
        Space::new().height(12),
        row![cancel_btn, Space::new().width(8), ok_btn],
    ]
    .spacing(4)
    .padding(32)
    .width(480);

    container(inner)
        .style(|_| iced::widget::container::Style {
            background: Some(theme::BG_ELEVATED.into()),
            border: iced::Border {
                color: theme::BORDER_ACTIVE,
                width: 2.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}
