//! File properties modal.

use iced::{Element, Font, Length};
use iced::widget::{button, column, container, row, text, Space};
use ironkey_browser::listing::{FileEntry, FileKind};
use ironkey_drives::format_size;
use crate::app::Message;
use crate::theme;

pub fn view<'a>(entry: &FileEntry) -> Element<'a, Message> {
    let kind_str = match entry.kind {
        FileKind::Directory => "Directory",
        FileKind::RegularFile => "Regular file",
        FileKind::Symlink => "Symbolic link",
        FileKind::Other => "Other",
    };

    let modified_str = entry
        .modified
        .as_ref()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_owned());

    let name = entry.name.clone();
    let path = entry.path.display().to_string();
    let size_str = format_size(entry.size);
    let perm = entry.permissions.clone();
    let ext = entry.extension.clone();

    let title = text(format!("Properties: {}", name))
        .size(16)
        .color(theme::ACCENT_PRIMARY);

    let mut rows: Vec<Element<Message>> = vec![
        info_row("Name", name),
        info_row("Path", path),
        info_row("Type", kind_str.to_owned()),
    ];

    if !entry.is_dir() {
        rows.push(info_row("Size", size_str));
    }

    rows.push(info_row("Modified", modified_str));
    rows.push(info_row("Permissions", perm));

    if let Some(e) = ext {
        rows.push(info_row("Extension", e));
    }

    let close_btn = button(text("CLOSE").size(14))
        .on_press(Message::ModalCancel)
        .padding([8, 24]);

    let inner = column![
        title,
        Space::new().height(12),
        column(rows).spacing(6),
        Space::new().height(16),
        row![Space::new().width(Length::Fill), close_btn].spacing(0),
    ]
    .spacing(2)
    .padding(32)
    .width(480);

    container(inner)
        .style(|_| iced::widget::container::Style {
            background: Some(theme::BG_ELEVATED.into()),
            border: iced::Border {
                color: theme::BORDER_DEFAULT,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn info_row(label: impl Into<String>, value: impl Into<String>) -> Element<'static, Message> {
    row![
        text(label.into())
            .size(12)
            .color(theme::TEXT_SECONDARY)
            .width(120),
        text(value.into())
            .size(12)
            .color(theme::TEXT_PRIMARY)
            .font(Font::MONOSPACE),
    ]
    .spacing(8)
    .into()
}
