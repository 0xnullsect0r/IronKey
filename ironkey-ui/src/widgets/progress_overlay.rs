//! Full-screen progress overlay for long-running operations.

use iced::{Element, Length};
use iced::widget::{button, column, container, row, text, Space};
use crate::app::Message;
use crate::theme;

/// Render a progress overlay.
pub fn view(
    operation: &str,
    progress: f32,
    speed_bps: u64,
    eta_secs: Option<u64>,
) -> Element<'_, Message> {
    let title = text(operation).size(20).color(theme::ACCENT_PRIMARY);

    let pct = (progress * 100.0) as u32;
    let bar_width = (progress * 60.0) as usize;
    let bar_filled = "█".repeat(bar_width);
    let bar_empty  = "░".repeat(60_usize.saturating_sub(bar_width));
    let bar_label  = format!("[{}{}] {}%", bar_filled, bar_empty, pct);

    let speed = format_speed(speed_bps);
    let eta = eta_secs
        .map(|s| format!("ETA: {}s", s))
        .unwrap_or_else(|| "ETA: calculating…".to_string());

    let cancel = button(text("CANCEL").size(14).color(theme::ACCENT_DANGER))
        .on_press(Message::ModalCancel)
        .padding([8, 16]);

    let inner = column![
        title,
        Space::new().height(12),
        text(bar_label).size(14).font(iced::Font::MONOSPACE).color(theme::ACCENT_PRIMARY),
        Space::new().height(8),
        row![
            text(speed).size(13).color(theme::TEXT_SECONDARY),
            Space::new().width(24),
            text(eta).size(13).color(theme::TEXT_SECONDARY),
        ],
        Space::new().height(16),
        cancel,
    ]
    .spacing(4)
    .padding(32);

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
        .max_width(600)
        .into()
}

fn format_speed(bps: u64) -> String {
    if bps >= 1024 * 1024 * 1024 {
        format!("{:.1} GB/s", bps as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bps >= 1024 * 1024 {
        format!("{:.1} MB/s", bps as f64 / (1024.0 * 1024.0))
    } else if bps >= 1024 {
        format!("{:.1} KB/s", bps as f64 / 1024.0)
    } else {
        format!("{} B/s", bps)
    }
}
