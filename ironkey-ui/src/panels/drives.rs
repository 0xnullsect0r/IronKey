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
        let is_expanded = app.expanded_drives().contains(&idx);

        // Drive row
        items.push(drive_row(drive, idx));

        if is_expanded {
            // Partition bar
            let bar = crate::widgets::partition_bar::view(drive);
            items.push(bar);
            items.push(Space::new().height(2).into());

            // Partition rows
            for part in &drive.partitions {
                items.push(partition_row(part, app));
            }
            items.push(Space::new().height(4).into());
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

fn drive_row<'a>(drive: &'a DriveInfo, idx: usize) -> Element<'a, Message> {
    let icon = if drive.rotational { "💿" } else { "⚡" };
    let kind = if drive.removable { "USB" } else if drive.rotational { "HDD" } else { "SSD" };
    let model = drive.model.as_deref().unwrap_or("Unknown");
    let size = format_size(drive.size_bytes);

    let health_color = match &drive.status {
        ironkey_drives::DriveStatus::Healthy => theme::ACCENT_SUCCESS,
        ironkey_drives::DriveStatus::Error(_) => theme::ACCENT_DANGER,
    };

    let label = format!("{} /dev/{}  [{}]  {}  ({})", icon, drive.device, kind, model, size);

    button(
        row![
            text(label).size(13).color(theme::ACCENT_PRIMARY),
            Space::new().width(Length::Fill),
            text("●").size(10).color(health_color),
        ]
        .spacing(4)
        .align_y(iced::Center),
    )
    .on_press(Message::DriveToggled(idx))
    .width(Length::Fill)
    .padding([3, 6])
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

    let mp_str = match &part.status {
        PartitionStatus::Mounted(p) => format!("  → {}", p.display()),
        _ => String::new(),
    };

    let line = format!(
        "  {} ├─ /dev/{}{}  {}  {}{}",
        status_icon, part.device, label_suffix, fs, size, mp_str
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
    .width(Length::Fill)
    .padding([2, 4])
    .into()
}
