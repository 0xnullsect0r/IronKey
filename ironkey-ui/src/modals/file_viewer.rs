//! File content viewer modal (text, hex dump).

use iced::{Element, Font, Length};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use ironkey_browser::viewer::{ViewedContent, ViewMode};
use crate::app::Message;
use crate::theme;

pub fn view<'a>(path: &'a std::path::Path, content: &'a ViewedContent) -> Element<'a, Message> {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let title = text(format!("  {}", file_name))
        .size(15)
        .color(theme::ACCENT_PRIMARY)
        .font(Font::MONOSPACE);

    let size_info = text(format!(
        "{}  {}{}",
        ironkey_drives::format_size(content.file_size),
        match content.mode {
            ViewMode::Text => "TEXT",
            ViewMode::Hex | ViewMode::Binary => "HEX",
        },
        if content.truncated { "  (truncated — first 512 KiB shown)" } else { "" },
    ))
    .size(11)
    .color(theme::TEXT_SECONDARY);

    let body: Element<'a, Message> = match content.mode {
        ViewMode::Text => {
            let t = content.text.as_deref().unwrap_or("");
            scrollable(
                text(t)
                    .size(12)
                    .font(Font::MONOSPACE)
                    .color(theme::TEXT_PRIMARY)
                    .width(Length::Fill),
            )
            .height(500)
            .into()
        }
        ViewMode::Hex | ViewMode::Binary => {
            let lines: Vec<Element<'a, Message>> = content
                .hex_lines
                .iter()
                .map(|line| {
                    let offset = text(format!("{:08X}  ", line.offset))
                        .size(11)
                        .font(Font::MONOSPACE)
                        .color(theme::TEXT_SECONDARY);

                    let mut hex_parts: Vec<String> =
                        line.bytes.iter().map(|b| format!("{:02X}", b)).collect();
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
                })
                .collect();

            scrollable(column(lines).spacing(0).width(Length::Fill))
                .height(500)
                .into()
        }
    };

    let close_btn = button(text("CLOSE").size(14))
        .on_press(Message::ModalCancel)
        .padding([8, 24]);

    let inner = column![
        title,
        Space::new().height(2),
        text(path.display().to_string())
            .size(10)
            .color(theme::TEXT_SECONDARY)
            .font(Font::MONOSPACE),
        size_info,
        Space::new().height(8),
        body,
        Space::new().height(12),
        row![Space::new().width(Length::Fill), close_btn].spacing(0),
    ]
    .spacing(2)
    .padding([16, 24]);

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
        .max_width(900)
        .into()
}
