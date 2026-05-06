//! Left panel: drive tree and partition listing.

use iced::{Element, Length};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use ironkey_drives::{DriveInfo, PartitionStatus, format_size};
use crate::app::{IronKeyApp, Message};
use crate::theme;

pub fn view(app: &IronKeyApp) -> Element<'_, Message> {
    let title = text("DRIVES & PARTITIONS")
        .size(11)
        .color(theme::TEXT_SECONDARY);

    let mut items: Vec<Element<Message>> = vec![
        title.into(),
        Space::new().height(6).into(),
    ];

    for (idx, drive) in app.drives().iter().enumerate() {
        items.push(drive_row(drive, idx, app));

        if app.expanded_drives().contains(&idx) {
            for part in &drive.partitions {
                items.push(partition_row(part, app));
            }
        }
    }

    if app.drives().is_empty() {
        items.push(
            text("Scanning drives…")
                .size(13)
                .color(theme::TEXT_SECONDARY)
                .into(),
        );
    }

    let scroll = scrollable(
        column(items).spacing(2).padding(8).width(Length::Fill),
    )
    .height(Length::Fill);

    container(scroll)
        .style(theme::panel_style)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn drive_row<'a>(
    drive: &'a DriveInfo,
    idx: usize,
    _app: &'a IronKeyApp,
) -> Element<'a, Message> {
    let icon = if drive.rotational { "💿" } else { "⚡" };
    let model = drive.model.as_deref().unwrap_or("Unknown");
    let size = format_size(drive.size_bytes);

    let label = format!("{} /dev/{}  {}  ({})", icon, drive.device, model, size);

    button(
        text(label).size(13).color(theme::ACCENT_PRIMARY),
    )
    .on_press(Message::DriveToggled(idx))
    .padding([2, 6])
    .into()
}

fn partition_row<'a>(
    part: &'a ironkey_drives::PartitionInfo,
    app: &'a IronKeyApp,
) -> Element<'a, Message> {
    let status_icon = match &part.status {
        PartitionStatus::Mounted(_) => "🟢",
        PartitionStatus::Unmounted => "🟡",
        PartitionStatus::Encrypted => "🔵",
        PartitionStatus::Unformatted => "⚪",
        PartitionStatus::Error(_) => "🔴",
    };

    let fs = part.filesystem.as_deref().unwrap_or("?");
    let size = format_size(part.size_bytes);
    let label_str = part.label.as_deref().unwrap_or("");
    let label_suffix = if label_str.is_empty() {
        String::new()
    } else {
        format!(" [{}]", label_str)
    };

    let is_selected = app.selected_partition() == Some(part.device.as_str());
    let text_color = if is_selected {
        theme::ACCENT_PRIMARY
    } else {
        theme::TEXT_PRIMARY
    };

    let line = format!(
        "  {} ├─ /dev/{}{}  {}  {}",
        status_icon, part.device, label_suffix, fs, size
    );

    let dev = part.device.clone();
    button(
        row![
            Space::new().width(12),
            text(line).size(12).color(text_color),
        ]
        .spacing(0),
    )
    .on_press(Message::PartitionSelected(dev))
    .padding([2, 4])
    .into()
}
