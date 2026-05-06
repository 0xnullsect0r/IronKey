//! Right panel: embedded terminal emulator.

use iced::{Element, Length};
use iced::widget::{column, container, row, scrollable, text, text_input, Space};
use crate::app::{IronKeyApp, Message};
use crate::theme;

pub fn view(app: &IronKeyApp) -> Element<'_, Message> {
    let title = text("TERMINAL")
        .size(11)
        .color(theme::TEXT_SECONDARY);

    // Scrollback lines
    let lines: Vec<Element<Message>> = app
        .terminal_state()
        .lines
        .iter()
        .rev()
        .take(200) // render at most 200 lines for performance
        .rev()
        .map(|line| {
            let plain = line.plain_text();
            text(plain).size(12).color(theme::TEXT_CODE).font(iced::Font::MONOSPACE).into()
        })
        .collect();

    let output_area = scrollable(
        column(lines)
            .spacing(0)
            .padding(8)
            .width(Length::Fill),
    )
    .height(Length::Fill)
    .anchor_bottom();

    // Input line
    let prompt = text("$ ").size(13).color(theme::ACCENT_PRIMARY).font(iced::Font::MONOSPACE);
    let input = text_input("", app.terminal_input())
        .on_input(Message::TerminalInputChanged)
        .on_submit(Message::TerminalSubmit)
        .size(13)
        .font(iced::Font::MONOSPACE)
        .width(Length::Fill);

    let input_row = row![prompt, input].spacing(0).padding([4, 8]);

    let content = column![
        title,
        Space::new().height(4),
        output_area,
        input_row,
    ]
    .spacing(0)
    .height(Length::Fill)
    .width(Length::Fill);

    container(content)
        .style(theme::panel_style)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
