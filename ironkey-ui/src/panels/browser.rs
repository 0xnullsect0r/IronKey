//! Center panel: file browser.

use iced::{Element, Length};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use ironkey_browser::listing::FileKind;
use crate::app::{IronKeyApp, Message};
use crate::theme;

pub fn view(app: &IronKeyApp) -> Element<'_, Message> {
    let title = text("FILE BROWSER")
        .size(11)
        .color(theme::TEXT_SECONDARY);

    // Breadcrumb path bar
    let breadcrumb = view_breadcrumb(app);

    // Search bar
    let search_bar: Element<Message> = if app.search_open() {
        row![
            text_input("Search…", app.search_query())
                .on_input(Message::SearchQueryChanged)
                .on_submit(Message::SearchSubmit)
                .size(13)
                .width(Length::Fill),
            button(text("✕").size(12))
                .on_press(Message::SearchToggle)
                .padding([2, 6]),
        ]
        .spacing(4)
        .into()
    } else {
        Space::new().height(0).into()
    };

    // File list
    let file_list = view_file_list(app);

    let content = column![
        title,
        Space::new().height(4),
        breadcrumb,
        search_bar,
        Space::new().height(4),
        file_list,
    ]
    .spacing(2)
    .padding(8)
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .style(theme::panel_style)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_breadcrumb(app: &IronKeyApp) -> Element<'_, Message> {
    let path = app.current_path();
    let mut segments: Vec<Element<Message>> = Vec::new();

    let mut accumulated = std::path::PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::RootDir => {
                accumulated.push("/");
                let p = accumulated.clone();
                segments.push(
                    button(text("/").size(12).color(theme::ACCENT_PRIMARY))
                        .on_press(Message::NavigateTo(p))
                        .padding([1, 4])
                        .into(),
                );
            }
            Component::Normal(name) => {
                accumulated.push(name);
                let p = accumulated.clone();
                let label = name.to_string_lossy().to_string();
                segments.push(
                    text(" / ").size(12).color(theme::TEXT_SECONDARY).into(),
                );
                segments.push(
                    button(text(label).size(12).color(theme::ACCENT_PRIMARY))
                        .on_press(Message::NavigateTo(p))
                        .padding([1, 4])
                        .into(),
                );
            }
            _ => {}
        }
    }

    row(segments).spacing(0).wrap().into()
}

fn view_file_list(app: &IronKeyApp) -> Element<'_, Message> {
    // Column headers
    let header = row![
        text("Name").size(11).color(theme::TEXT_SECONDARY).width(Length::FillPortion(5)),
        text("Size").size(11).color(theme::TEXT_SECONDARY).width(Length::FillPortion(1)),
        text("Modified").size(11).color(theme::TEXT_SECONDARY).width(Length::FillPortion(2)),
    ]
    .spacing(4)
    .padding([0, 4]);

    let mut rows: Vec<Element<Message>> = vec![header.into()];

    if app.files().is_empty() && !app.current_path().as_os_str().is_empty() {
        rows.push(
            text("(empty directory)")
                .size(12)
                .color(theme::TEXT_SECONDARY)
                .into(),
        );
    }

    for (idx, entry) in app.files().iter().enumerate() {
        let is_selected = app.selected_file() == Some(idx);
        let name_color = if is_selected {
            theme::ACCENT_PRIMARY
        } else if entry.is_dir() {
            theme::ACCENT_SECONDARY
        } else {
            theme::TEXT_PRIMARY
        };

        let icon = file_icon(&entry.kind, entry.extension.as_deref());
        let name_label = format!("{} {}", icon, entry.name);
        let size_label = entry.size_display();
        let date_label = entry
            .modified
            .as_ref()
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        let file_row = button(
            row![
                text(name_label).size(12).color(name_color).width(Length::FillPortion(5)),
                text(size_label).size(11).color(theme::TEXT_SECONDARY).width(Length::FillPortion(1)),
                text(date_label).size(11).color(theme::TEXT_SECONDARY).width(Length::FillPortion(2)),
            ]
            .spacing(4),
        )
        .on_press(Message::FileSelected(idx))
        .padding([3, 4])
        .width(Length::Fill);

        // Double-click emulated via separate activate button (iced has no native double-click)
        // For now, single-click selects; pressing Enter activates.
        rows.push(file_row.into());
    }

    scrollable(
        column(rows).spacing(1).width(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}

fn file_icon(kind: &FileKind, ext: Option<&str>) -> &'static str {
    match kind {
        FileKind::Directory => "📁",
        FileKind::Symlink => "🔗",
        FileKind::Other => "❓",
        FileKind::RegularFile => match ext {
            Some("rs") => "🦀",
            Some("txt") | Some("md") | Some("log") => "📄",
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("bmp") | Some("svg") => "🖼",
            Some("zip") | Some("tar") | Some("gz") | Some("xz") | Some("bz2") | Some("zst") => "📦",
            Some("py") | Some("js") | Some("ts") | Some("go") | Some("c") | Some("cpp") | Some("h") => "📝",
            Some("json") | Some("toml") | Some("yaml") | Some("yml") | Some("xml") => "⚙",
            Some("sh") | Some("bash") | Some("zsh") => "🐚",
            Some("pdf") => "📕",
            Some("iso") | Some("img") => "💿",
            _ => "📄",
        },
    }
}
