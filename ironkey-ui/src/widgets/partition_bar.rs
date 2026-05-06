//! Horizontal partition bar visualizer (scaled disk layout).

use iced::{Element, Length};
use iced::widget::{container, row, text, Space};
use ironkey_drives::DriveInfo;
use crate::theme;

/// View the partition layout of a drive as a horizontal bar.
pub fn view<'a, Message: 'a + Clone>(drive: &'a DriveInfo) -> Element<'a, Message> {
    if drive.size_bytes == 0 || drive.partitions.is_empty() {
        return Space::new().height(0).into();
    }

    let total = drive.size_bytes as f64;
    let mut segments: Vec<Element<'a, Message>> = Vec::new();

    for part in &drive.partitions {
        let frac = (part.size_bytes as f64 / total) as f32;
        let color = partition_color(&part.status);
        let tooltip = format!(
            "/dev/{} {}",
            part.device,
            ironkey_drives::format_size(part.size_bytes)
        );

        segments.push(
            container(text(tooltip).size(9).color(theme::BG_BASE))
                .style(move |_| iced::widget::container::Style {
                    background: Some(color.into()),
                    border: iced::Border {
                        color: theme::BG_BASE,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                })
                .width(Length::FillPortion((frac * 1000.0) as u16))
                .height(20)
                .into(),
        );
    }

    row(segments).spacing(1).height(20).into()
}

fn partition_color(status: &ironkey_drives::PartitionStatus) -> iced::Color {
    match status {
        ironkey_drives::PartitionStatus::Mounted(_) => theme::ACCENT_SUCCESS,
        ironkey_drives::PartitionStatus::Unmounted => theme::ACCENT_WARNING,
        ironkey_drives::PartitionStatus::Encrypted => theme::ACCENT_SECONDARY,
        ironkey_drives::PartitionStatus::Unformatted => theme::TEXT_SECONDARY,
        ironkey_drives::PartitionStatus::Error(_) => theme::ACCENT_DANGER,
    }
}
