//! Virtual-scrolling file list widget.
//!
//! For large directories (100k+ entries) only the visible rows are rendered.

use iced::{Element, Length};
use iced::widget::{column, row, text};
use ironkey_browser::listing::FileEntry;
use crate::theme;

/// Render a slice of file entries as a virtual list.
///
/// `visible_start` and `visible_end` define the range of entries to render.
/// The caller is responsible for calculating these from the scroll position.
pub fn view<'a, Message: 'a + Clone>(
    entries: &'a [FileEntry],
    selected: Option<usize>,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let rows: Vec<Element<'a, Message>> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let is_sel = selected == Some(idx);
            let color = if is_sel {
                theme::ACCENT_PRIMARY
            } else if entry.is_dir() {
                theme::ACCENT_SECONDARY
            } else {
                theme::TEXT_PRIMARY
            };

            row![
                text(entry.name.as_str()).size(12).color(color).width(Length::Fill),
                text(entry.size_display()).size(11).color(theme::TEXT_SECONDARY).width(80),
            ]
            .spacing(4)
            .padding([2, 4])
            .into()
        })
        .collect();

    column(rows).spacing(0).width(Length::Fill).into()
}
