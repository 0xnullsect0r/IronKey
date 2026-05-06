//! Partition info + SMART data modal.

use iced::{Element, Length};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use ironkey_drives::{PartitionInfo, SmartData, format_size};
use crate::app::Message;
use crate::theme;

pub fn view<'a>(part: &PartitionInfo, smart: &Option<SmartData>) -> Element<'a, Message> {
    let title_str = format!("INFO: /dev/{}", part.device);
    let fs_str = part.filesystem.as_deref().unwrap_or("unknown").to_owned();
    let size_str = format_size(part.size_bytes);
    let label_str = part.label.clone();
    let uuid_str = part.uuid.clone();
    let start_str = part.start_sector.map(|s| s.to_string());
    let end_str = part.end_sector.map(|s| s.to_string());
    let dev_path = format!("/dev/{}", part.device);
    let status_str = match &part.status {
        ironkey_drives::PartitionStatus::Mounted(p) => format!("Mounted at {}", p.display()),
        ironkey_drives::PartitionStatus::Unmounted => "Unmounted".to_owned(),
        ironkey_drives::PartitionStatus::Encrypted => "Encrypted".to_owned(),
        ironkey_drives::PartitionStatus::Unformatted => "Unformatted".to_owned(),
        ironkey_drives::PartitionStatus::Error(e) => format!("Error: {}", e),
    };

    let mut detail_rows: Vec<Element<Message>> = vec![
        info_row("Device", dev_path),
        info_row("Filesystem", fs_str),
        info_row("Size", size_str),
    ];
    if let Some(lbl) = label_str {
        detail_rows.push(info_row("Label", lbl));
    }
    if let Some(uuid) = uuid_str {
        detail_rows.push(info_row("UUID", uuid));
    }
    if let Some(s) = start_str {
        detail_rows.push(info_row("Start sector", s));
    }
    if let Some(s) = end_str {
        detail_rows.push(info_row("End sector", s));
    }
    detail_rows.push(info_row("Status", status_str));

    let partition_section: Element<Message> = column(detail_rows).spacing(4).into();

    let smart_section: Element<Message> = if let Some(s) = smart {
        let health_color = if s.overall_health { theme::ACCENT_SUCCESS } else { theme::ACCENT_DANGER };

        let mut smart_rows: Vec<Element<Message>> = vec![
            text("SMART DATA").size(12).color(theme::TEXT_SECONDARY).into(),
            row![
                text("Health:").size(12).color(theme::TEXT_SECONDARY).width(160),
                text(s.health_label()).size(12).color(health_color),
            ]
            .spacing(8)
            .into(),
        ];
        if let Some(temp) = s.temperature_celsius {
            smart_rows.push(info_row("Temperature", format!("{}°C", temp)));
        }
        smart_rows.push(info_row("Power-on hours", s.power_on_hours.to_string()));
        smart_rows.push(info_row("Reallocated sectors", s.reallocated_sectors.to_string()));
        smart_rows.push(info_row("Pending sectors", s.pending_sectors.to_string()));
        smart_rows.push(info_row("Uncorrectable sectors", s.uncorrectable_sectors.to_string()));

        column(smart_rows).spacing(4).into()
    } else {
        text("SMART data not available for this partition")
            .size(12)
            .color(theme::TEXT_SECONDARY)
            .into()
    };

    let close_btn = button(text("CLOSE").size(14))
        .on_press(Message::ModalCancel)
        .padding([8, 24]);

    let inner = column![
        text(title_str).size(18).color(theme::ACCENT_PRIMARY),
        Space::new().height(12),
        text("PARTITION DETAILS").size(11).color(theme::TEXT_SECONDARY),
        Space::new().height(4),
        partition_section,
        Space::new().height(16),
        smart_section,
        Space::new().height(16),
        close_btn,
    ]
    .spacing(2)
    .padding(32)
    .width(520);

    container(scrollable(inner).height(Length::Shrink))
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

fn info_row(label: impl Into<String>, value: impl Into<String>) -> Element<'static, Message> {
    row![
        text(label.into()).size(12).color(theme::TEXT_SECONDARY).width(160),
        text(value.into()).size(12).color(theme::TEXT_PRIMARY),
    ]
    .spacing(8)
    .into()
}
