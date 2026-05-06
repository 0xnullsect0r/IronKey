//! Hex dump viewer widget.

use iced::{Element, Font, Length};
use iced::widget::{column, container, row, scrollable, text};
use ironkey_browser::viewer::{HexLine, ViewedContent, ViewMode};
use crate::theme;

/// Render a `ViewedContent` (text or hex dump) in the viewer modal.
pub fn view<'a, Message: 'a>(content: &'a ViewedContent) -> Element<'a, Message> {
    match content.mode {
        ViewMode::Text => {
            let t = content.text.as_deref().unwrap_or("");
            scrollable(
                text(t)
                    .size(12)
                    .font(Font::MONOSPACE)
                    .color(theme::TEXT_PRIMARY)
                    .width(Length::Fill),
            )
            .height(Length::Fill)
            .into()
        }
        ViewMode::Hex | ViewMode::Binary => {
            let lines: Vec<Element<'a, Message>> = content
                .hex_lines
                .iter()
                .map(|line| hex_line_view(line))
                .collect();
            scrollable(
                column(lines).spacing(0).width(Length::Fill),
            )
            .height(Length::Fill)
            .into()
        }
    }
}

fn hex_line_view<'a, Message: 'a>(line: &'a HexLine) -> Element<'a, Message> {
    let offset = text(format!("{:08X}  ", line.offset))
        .size(11)
        .font(Font::MONOSPACE)
        .color(theme::TEXT_SECONDARY);

    // Pad hex to 16 bytes
    let mut hex_parts: Vec<String> = line.bytes.iter().map(|b| format!("{:02X}", b)).collect();
    while hex_parts.len() < 16 {
        hex_parts.push("  ".to_string());
    }
    let hex_str = hex_parts[..8].join(" ")
        + "  "
        + &hex_parts[8..].join(" ");

    let hex = text(hex_str)
        .size(11)
        .font(Font::MONOSPACE)
        .color(theme::TEXT_CODE);

    let ascii = text(format!("  |{}|", line.ascii_str()))
        .size(11)
        .font(Font::MONOSPACE)
        .color(theme::ACCENT_SUCCESS);

    row![offset, hex, ascii].spacing(0).into()
}
