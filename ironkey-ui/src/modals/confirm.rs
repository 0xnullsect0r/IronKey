//! Generic confirmation modal (yes/no dialog).

use iced::{Element, Length};
use iced::widget::{button, column, container, row, text, Space};
use crate::app::Message;
use crate::theme;

pub fn view<'a>(title: &'a str, message: &'a str) -> Element<'a, Message> {
    let title_widget = text(title).size(18).color(theme::ACCENT_WARNING);
    let msg_widget = text(message).size(13).color(theme::TEXT_PRIMARY);

    let ok_btn = button(text("OK").size(14))
        .on_press(Message::ModalConfirm)
        .padding([8, 24]);

    let cancel_btn = button(text("CANCEL").size(14))
        .on_press(Message::ModalCancel)
        .padding([8, 16]);

    let inner = column![
        title_widget,
        Space::new().height(12),
        msg_widget,
        Space::new().height(16),
        row![cancel_btn, Space::new().width(8), ok_btn],
    ]
    .spacing(4)
    .padding(32)
    .width(480);

    container(inner)
        .style(|_| iced::widget::container::Style {
            background: Some(theme::BG_ELEVATED.into()),
            border: iced::Border {
                color: theme::ACCENT_WARNING,
                width: 2.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}
