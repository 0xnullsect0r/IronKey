//! Modal dialogs.

pub mod confirm;
pub mod file_viewer;
pub mod format;
pub mod info;
pub mod passphrase;
pub mod properties;

use iced::{Element, Length};
use iced::widget::{container, opaque, stack};

/// Wrap `content` with a `modal` overlay centered on screen.
pub fn overlay<'a, Message: 'a + Clone>(
    content: Element<'a, Message>,
    modal: Element<'a, Message>,
) -> Element<'a, Message> {
    stack![
        content,
        opaque(
            container(modal)
                .style(|_| iced::widget::container::Style {
                    background: Some(
                        iced::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.75 }.into()
                    ),
                    ..Default::default()
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
    ]
    .into()
}
