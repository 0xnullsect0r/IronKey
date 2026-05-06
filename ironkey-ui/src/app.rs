//! Main application state, message types, update logic, and top-level view.

use iced::{
    Element, Length, Subscription, Task,
};
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Space,
    PaneGrid,
};
use iced::widget::pane_grid::{self, Axis, Content as PaneContent, ResizeEvent};
use ironkey_browser::listing::FileEntry;
use ironkey_drives::{DriveInfo, PartitionInfo};
use ironkey_terminal::TerminalState;
use sysinfo::System;

use crate::theme;

// ──────────────────────────────────────────────────────────────────────────────
// Pane identity
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneKind {
    Drives,
    Browser,
    Terminal,
}

// ──────────────────────────────────────────────────────────────────────────────
// Modal state
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Modal {
    Format {
        device: String,
        confirm_text: String,
        fs_type: ironkey_drives::ops::FormatFsType,
    },
    Passphrase {
        device: String,
        passphrase: String,
    },
    Confirm {
        title: String,
        message: String,
    },
    Progress {
        operation: String,
        progress: f32,
        speed_bps: u64,
        eta_secs: Option<u64>,
    },
}

// ──────────────────────────────────────────────────────────────────────────────
// Message
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    // Pane grid
    PaneResized(ResizeEvent),
    // System tick (2-second refresh)
    Tick,
    // Drives panel
    DrivesLoaded(Vec<DriveInfo>),
    DriveToggled(usize),
    PartitionSelected(String),
    MountSelected,
    UnmountSelected,
    FormatSelected,
    CloneSelected,
    WipeSelected,
    // Modal
    ModalConfirmTextChanged(String),
    ModalPassphraseChanged(String),
    ModalFsTypeChanged(ironkey_drives::ops::FormatFsType),
    ModalConfirm,
    ModalCancel,
    // Browser panel
    NavigateTo(std::path::PathBuf),
    FilesLoaded(Vec<FileEntry>),
    FileSelected(usize),
    FileActivated(usize),
    // Terminal panel
    TerminalInputChanged(String),
    TerminalSubmit,
    TerminalOutput(Vec<u8>),
    // Search
    SearchToggle,
    SearchQueryChanged(String),
    SearchSubmit,
}

// ──────────────────────────────────────────────────────────────────────────────
// Application state
// ──────────────────────────────────────────────────────────────────────────────

pub struct IronKeyApp {
    // Layout
    panes: pane_grid::State<PaneKind>,
    focused_pane: PaneKind,

    // Drives panel
    drives: Vec<DriveInfo>,
    expanded_drives: std::collections::HashSet<usize>,
    selected_partition: Option<String>,

    // Browser panel
    current_path: std::path::PathBuf,
    files: Vec<FileEntry>,
    selected_file: Option<usize>,
    search_open: bool,
    search_query: String,

    // Terminal panel
    terminal_state: TerminalState,
    terminal_input: String,
    pty_session: Option<ironkey_terminal::PtySession>,

    // System stats
    sys: System,
    cpu_usage: f32,
    ram_used: u64,
    ram_total: u64,

    // Modal overlay
    modal: Option<Modal>,
}

impl IronKeyApp {
    /// Boot function: create initial state and kick off the first drive scan.
    pub fn new() -> (Self, Task<Message>) {
        // Build the three-panel pane grid
        let (mut panes, drives_pane) = pane_grid::State::new(PaneKind::Drives);
        if let Some((browser_pane, _)) = panes.split(Axis::Vertical, drives_pane, PaneKind::Browser) {
            let _ = panes.split(Axis::Vertical, browser_pane, PaneKind::Terminal);
        }

        let mut sys = System::new_all();
        sys.refresh_all();
        let cpu_usage = sys.global_cpu_usage();
        let ram_used = sys.used_memory();
        let ram_total = sys.total_memory();

        let app = Self {
            panes,
            focused_pane: PaneKind::Drives,
            drives: Vec::new(),
            expanded_drives: std::collections::HashSet::new(),
            selected_partition: None,
            current_path: std::path::PathBuf::from("/"),
            files: Vec::new(),
            selected_file: None,
            search_open: false,
            search_query: String::new(),
            terminal_state: TerminalState::new(10_000),
            terminal_input: String::new(),
            pty_session: None,
            sys,
            cpu_usage,
            ram_used,
            ram_total,
            modal: None,
        };

        // Load initial drives and files in parallel
        let drives_task = Task::perform(
            async { ironkey_drives::enumerate_drives().unwrap_or_default() },
            Message::DrivesLoaded,
        );

        (app, drives_task)
    }

    /// Update function: handle messages and produce tasks.
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::PaneResized(event) => {
                self.panes.resize(event.split, event.ratio);
                Task::none()
            }

            Message::Tick => {
                self.sys.refresh_cpu_usage();
                self.sys.refresh_memory();
                self.cpu_usage = self.sys.global_cpu_usage();
                self.ram_used = self.sys.used_memory();
                self.ram_total = self.sys.total_memory();

                // Poll PTY output
                if let Some(ref mut pty) = self.pty_session {
                    let mut buf = [0u8; 4096];
                    if let Ok(n) = pty.read_output(&mut buf) {
                        if n > 0 {
                            let data = buf[..n].to_vec();
                            self.terminal_state.feed(&data);
                        }
                    }
                }

                Task::perform(
                    async { ironkey_drives::enumerate_drives().unwrap_or_default() },
                    Message::DrivesLoaded,
                )
            }

            Message::DrivesLoaded(drives) => {
                self.drives = drives;
                Task::none()
            }

            Message::DriveToggled(idx) => {
                if self.expanded_drives.contains(&idx) {
                    self.expanded_drives.remove(&idx);
                } else {
                    self.expanded_drives.insert(idx);
                }
                Task::none()
            }

            Message::PartitionSelected(dev) => {
                self.selected_partition = Some(dev);
                self.focused_pane = PaneKind::Drives;
                Task::none()
            }

            Message::MountSelected => {
                if let Some(ref dev) = self.selected_partition.clone() {
                    // Check if partition is encrypted → show passphrase modal
                    if let Some(part) = self.find_partition(dev) {
                        if part.status == ironkey_drives::PartitionStatus::Encrypted {
                            self.modal = Some(Modal::Passphrase {
                                device: dev.clone(),
                                passphrase: String::new(),
                            });
                            return Task::none();
                        }
                    }
                    // Normal mount
                    let device_path = std::path::PathBuf::from(format!("/dev/{}", dev));
                    let _ = ironkey_mount::mount(
                        &device_path,
                        ironkey_mount::FsType::Auto,
                        ironkey_mount::MountOptions::default(),
                    );
                }
                Task::none()
            }

            Message::UnmountSelected => {
                if let Some(ref dev) = self.selected_partition {
                    if let Some(part) = self.find_partition(dev) {
                        if let ironkey_drives::PartitionStatus::Mounted(ref mp) = part.status {
                            let _ = ironkey_mount::unmount(mp);
                        }
                    }
                }
                Task::none()
            }

            Message::FormatSelected => {
                if let Some(ref dev) = self.selected_partition.clone() {
                    self.modal = Some(Modal::Format {
                        device: dev.clone(),
                        confirm_text: String::new(),
                        fs_type: ironkey_drives::ops::FormatFsType::Ext4,
                    });
                }
                Task::none()
            }

            Message::CloneSelected => {
                // Placeholder: real impl opens a destination picker
                self.modal = Some(Modal::Confirm {
                    title: "Clone Partition".to_string(),
                    message: "Clone functionality: select a destination partition or image file."
                        .to_string(),
                });
                Task::none()
            }

            Message::WipeSelected => {
                if let Some(ref dev) = self.selected_partition.clone() {
                    self.modal = Some(Modal::Confirm {
                        title: "⚠ Wipe Partition".to_string(),
                        message: format!(
                            "Securely wipe /dev/{}? This is IRREVERSIBLE. Type CONFIRM to proceed.",
                            dev
                        ),
                    });
                }
                Task::none()
            }

            Message::ModalConfirmTextChanged(s) => {
                if let Some(Modal::Format { ref mut confirm_text, .. }) = self.modal {
                    *confirm_text = s;
                }
                Task::none()
            }

            Message::ModalPassphraseChanged(s) => {
                if let Some(Modal::Passphrase { ref mut passphrase, .. }) = self.modal {
                    *passphrase = s;
                }
                Task::none()
            }

            Message::ModalFsTypeChanged(fs) => {
                if let Some(Modal::Format { ref mut fs_type, .. }) = self.modal {
                    *fs_type = fs;
                }
                Task::none()
            }

            Message::ModalConfirm => {
                if let Some(modal) = self.modal.take() {
                    match modal {
                        Modal::Format { device, confirm_text, fs_type } => {
                            if confirm_text == "CONFIRM" {
                                let path = std::path::PathBuf::from(format!("/dev/{}", device));
                                let opts = ironkey_drives::ops::FormatOpts {
                                    fs_type,
                                    label: None,
                                };
                                if let Err(e) = ironkey_drives::ops::format_partition(&path, &opts) {
                                    log::error!("Format failed: {}", e);
                                }
                            }
                        }
                        Modal::Passphrase { device, passphrase } => {
                            let device_path =
                                std::path::PathBuf::from(format!("/dev/{}", device));
                            let _ = ironkey_mount::mount(
                                &device_path,
                                ironkey_mount::FsType::Luks,
                                ironkey_mount::MountOptions {
                                    passphrase: Some(passphrase),
                                    ..Default::default()
                                },
                            );
                        }
                        _ => {}
                    }
                }
                Task::none()
            }

            Message::ModalCancel => {
                self.modal = None;
                Task::none()
            }

            Message::NavigateTo(path) => {
                self.current_path = path.clone();
                Task::perform(
                    async move {
                        ironkey_browser::listing::list_directory(&path).unwrap_or_default()
                    },
                    Message::FilesLoaded,
                )
            }

            Message::FilesLoaded(files) => {
                self.files = files;
                self.selected_file = None;
                Task::none()
            }

            Message::FileSelected(idx) => {
                self.selected_file = Some(idx);
                Task::none()
            }

            Message::FileActivated(idx) => {
                if let Some(entry) = self.files.get(idx) {
                    if entry.is_dir() {
                        return self.update(Message::NavigateTo(entry.path.clone()));
                    }
                }
                Task::none()
            }

            Message::TerminalInputChanged(s) => {
                self.terminal_input = s;
                Task::none()
            }

            Message::TerminalSubmit => {
                let input = std::mem::take(&mut self.terminal_input);
                if !input.is_empty() {
                    let line = format!("$ {}\n", input);
                    self.terminal_state.feed(line.as_bytes());

                    // Send to PTY
                    if let Some(ref mut pty) = self.pty_session {
                        let _ = pty.write_input(format!("{}\n", input).as_bytes());
                    }
                }
                Task::none()
            }

            Message::TerminalOutput(data) => {
                self.terminal_state.feed(&data);
                Task::none()
            }

            Message::SearchToggle => {
                self.search_open = !self.search_open;
                self.search_query.clear();
                Task::none()
            }

            Message::SearchQueryChanged(q) => {
                self.search_query = q;
                Task::none()
            }

            Message::SearchSubmit => {
                // Perform a name-based search in the current directory
                let opts = ironkey_browser::search::SearchOpts {
                    name_glob: Some(self.search_query.clone()),
                    max_depth: Some(3),
                    ..Default::default()
                };
                let root = self.current_path.clone();
                Task::perform(
                    async move {
                        let results =
                            ironkey_browser::search::search_directory(&root, &opts)
                                .unwrap_or_default();
                        results
                            .into_iter()
                            .map(|r| ironkey_browser::listing::FileEntry {
                                name: r.path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                path: r.path.clone(),
                                kind: ironkey_browser::listing::FileKind::RegularFile,
                                size: 0,
                                modified: None,
                                permissions: String::new(),
                                extension: r.path.extension()
                                    .map(|e| e.to_string_lossy().to_lowercase()),
                            })
                            .collect::<Vec<_>>()
                    },
                    Message::FilesLoaded,
                )
            }
        }
    }

    /// Subscription: tick every 2 seconds for drive refresh and CPU/RAM stats.
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::Tick)
    }

    /// View function: render the full UI.
    pub fn view(&self) -> Element<'_, Message> {
        let header = self.view_header();
        let main = self.view_main();
        let status_bar = self.view_status_bar();

        let base: Element<Message> = column![header, main, status_bar]
            .spacing(0)
            .height(Length::Fill)
            .into();

        if let Some(ref modal) = self.modal {
            crate::modals::overlay(base, self.view_modal(modal))
        } else {
            base
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Header
    // ──────────────────────────────────────────────────────────────────────────

    fn view_header(&self) -> Element<'_, Message> {
        let logo = text("◈ IRONKEY  v0.1.0")
            .size(18)
            .color(theme::ACCENT_PRIMARY);

        let cpu = text(format!("CPU {:.0}%", self.cpu_usage))
            .size(13)
            .color(theme::TEXT_SECONDARY);

        let ram_mb_used = self.ram_used / 1024 / 1024;
        let ram_mb_total = self.ram_total / 1024 / 1024;
        let ram = text(format!("RAM {} / {} MB", ram_mb_used, ram_mb_total))
            .size(13)
            .color(theme::TEXT_SECONDARY);

        let time_str = chrono::Local::now().format("%H:%M:%S").to_string();
        let clock = text(time_str).size(13).color(theme::TEXT_SECONDARY);

        let right = row![cpu, Space::new().width(16), ram, Space::new().width(16), clock]
            .spacing(0)
            .align_y(iced::Center);

        container(
            row![
                logo,
                Space::new().width(Length::Fill),
                right,
            ]
            .align_y(iced::Center)
            .padding(8),
        )
        .style(theme::header_style)
        .width(Length::Fill)
        .into()
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Main pane grid
    // ──────────────────────────────────────────────────────────────────────────

    fn view_main(&self) -> Element<'_, Message> {
        PaneGrid::new(&self.panes, |_pane, kind, _is_maximized| {
            let content: Element<Message> = match kind {
                PaneKind::Drives => crate::panels::drives::view(self),
                PaneKind::Browser => crate::panels::browser::view(self),
                PaneKind::Terminal => crate::panels::terminal::view(self),
            };
            PaneContent::new(content)
        })
        .on_resize(10, Message::PaneResized)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Status bar
    // ──────────────────────────────────────────────────────────────────────────

    fn view_status_bar(&self) -> Element<'_, Message> {
        let partition_info = if let Some(ref dev) = self.selected_partition {
            if let Some(part) = self.find_partition(dev) {
                let fs = part.filesystem.as_deref().unwrap_or("?");
                let size = ironkey_drives::format_size(part.size_bytes);
                let mp = match &part.status {
                    ironkey_drives::PartitionStatus::Mounted(p) => {
                        format!(" │ MOUNTED at {}", p.display())
                    }
                    _ => String::new(),
                };
                format!("/dev/{}  │  {}  │  {}{}", dev, fs, size, mp)
            } else {
                format!("/dev/{}", dev)
            }
        } else {
            "No partition selected".to_string()
        };

        let actions = self.view_status_actions();

        container(
            row![
                text(partition_info)
                    .size(12)
                    .color(theme::TEXT_SECONDARY)
                    .width(Length::FillPortion(2)),
                actions,
            ]
            .spacing(8)
            .align_y(iced::Center)
            .padding(6),
        )
        .style(theme::header_style)
        .width(Length::Fill)
        .into()
    }

    fn view_status_actions(&self) -> Element<'_, Message> {
        let has_selection = self.selected_partition.is_some();
        let is_mounted = self.selected_partition.as_ref().and_then(|d| self.find_partition(d)).map_or(false, |p| {
            matches!(p.status, ironkey_drives::PartitionStatus::Mounted(_))
        });

        fn btn(label: &'static str, msg: Message, enabled: bool) -> iced::Element<'static, Message> {
            let b = button(text(label).size(12));
            if enabled {
                b.on_press(msg)
            } else {
                b
            }
            .into()
        }

        row![
            btn("MOUNT",   Message::MountSelected,   has_selection && !is_mounted),
            btn("UNMOUNT", Message::UnmountSelected,  has_selection && is_mounted),
            btn("FORMAT",  Message::FormatSelected,   has_selection),
            btn("CLONE",   Message::CloneSelected,    has_selection),
            btn("WIPE",    Message::WipeSelected,     has_selection),
        ]
        .spacing(4)
        .into()
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Modal view
    // ──────────────────────────────────────────────────────────────────────────

    fn view_modal<'a>(&'a self, modal: &'a Modal) -> Element<'a, Message> {
        match modal {
            Modal::Format { device, confirm_text, fs_type: _ } => {
                crate::modals::format::view(device, confirm_text)
            }
            Modal::Passphrase { device, passphrase } => {
                crate::modals::passphrase::view(device, passphrase)
            }
            Modal::Confirm { title, message } => {
                crate::modals::confirm::view(title, message)
            }
            Modal::Progress { operation, progress, speed_bps, eta_secs } => {
                crate::widgets::progress_overlay::view(operation, *progress, *speed_bps, *eta_secs)
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────────────────────────────────

    /// Look up a partition by its short device name (e.g. "sda1").
    pub fn find_partition(&self, dev_name: &str) -> Option<&PartitionInfo> {
        for drive in &self.drives {
            for part in &drive.partitions {
                if part.device == dev_name {
                    return Some(part);
                }
            }
        }
        None
    }
}

// Expose these fields for panel rendering
impl IronKeyApp {
    pub fn drives(&self) -> &[DriveInfo] {
        &self.drives
    }
    pub fn expanded_drives(&self) -> &std::collections::HashSet<usize> {
        &self.expanded_drives
    }
    pub fn selected_partition(&self) -> Option<&str> {
        self.selected_partition.as_deref()
    }
    pub fn current_path(&self) -> &std::path::Path {
        &self.current_path
    }
    pub fn files(&self) -> &[FileEntry] {
        &self.files
    }
    pub fn selected_file(&self) -> Option<usize> {
        self.selected_file
    }
    pub fn search_open(&self) -> bool {
        self.search_open
    }
    pub fn search_query(&self) -> &str {
        &self.search_query
    }
    pub fn terminal_state(&self) -> &TerminalState {
        &self.terminal_state
    }
    pub fn terminal_input(&self) -> &str {
        &self.terminal_input
    }
}
