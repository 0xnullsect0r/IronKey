//! Main application state, message types, update logic, and top-level view.

use iced::{
    Element, Length, Subscription, Task,
};
use iced::widget::{
    button, column, container, row, text, Space,
    PaneGrid,
};
use iced::widget::pane_grid::{self, Axis, Content as PaneContent, ResizeEvent};
use ironkey_browser::listing::FileEntry;
use ironkey_drives::{DriveInfo, PartitionInfo, SmartData};
use ironkey_terminal::TerminalState;
use sysinfo::System;

use crate::io_stats::{DiskStats, format_rate};
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
// Clipboard
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardOp {
    Copy,
    Cut,
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
    /// Partition info + SMART data
    Info {
        part: PartitionInfo,
        smart: Option<SmartData>,
    },
    /// File content viewer
    FileViewer {
        path: std::path::PathBuf,
        content: ironkey_browser::viewer::ViewedContent,
    },
    /// File properties
    Properties {
        entry: FileEntry,
    },
    /// Shutdown confirmation
    Shutdown,
    /// Reboot confirmation
    Reboot,
    /// New partition table confirmation
    NewTable {
        device: String,
        confirm_text: String,
    },
}

// ──────────────────────────────────────────────────────────────────────────────
// Message
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    // Pane grid
    PaneResized(ResizeEvent),
    // Focus pane
    FocusPane(PaneKind),
    FocusNext,
    // System tick (2-second refresh)
    Tick,
    Refresh,
    // Drives panel
    DrivesLoaded(Vec<DriveInfo>),
    DriveToggled(usize),
    PartitionSelected(String),
    MountSelected,
    UnmountSelected,
    FormatSelected,
    CloneSelected,
    WipeSelected,
    InfoSelected,
    NewTableSelected,
    SmartLoaded(String, Option<SmartData>),
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
    // File operations
    FileCopySelected,
    FileCutSelected,
    FilePasteSelected,
    FileDeleteSelected,
    FileRenameStart,
    FileRenameInputChanged(String),
    FileRenameCommit,
    FileCancelInlineEdit,
    FileNewFolderStart,
    FileNewFolderInputChanged(String),
    FileNewFolderCommit,
    FilePropertiesSelected,
    ViewFile(std::path::PathBuf),
    FileViewerLoaded(std::path::PathBuf, ironkey_browser::viewer::ViewedContent),
    // Terminal panel
    TerminalInputChanged(String),
    TerminalSubmit,
    TerminalOutput(Vec<u8>),
    // Search
    SearchToggle,
    SearchQueryChanged(String),
    SearchSubmit,
    // System
    ShutdownRequested,
    RebootRequested,
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

    // File operations
    clipboard: Option<(std::path::PathBuf, ClipboardOp)>,
    rename_index: Option<usize>,
    rename_input: String,
    new_folder_active: bool,
    new_folder_input: String,

    // Terminal panel
    terminal_state: TerminalState,
    terminal_input: String,
    pty_session: Option<ironkey_terminal::PtySession>,

    // System stats
    sys: System,
    cpu_usage: f32,
    ram_used: u64,
    ram_total: u64,

    // Disk I/O
    disk_read_bps: u64,
    disk_write_bps: u64,
    prev_diskstats: Option<DiskStats>,

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

        // Try to spawn PTY session (zsh/bash on Linux)
        let pty_session = ironkey_terminal::PtySession::spawn(
            ironkey_terminal::pty::PtyOptions::default(),
        )
        .map_err(|e| log::warn!("Could not spawn PTY: {}", e))
        .ok();

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
            clipboard: None,
            rename_index: None,
            rename_input: String::new(),
            new_folder_active: false,
            new_folder_input: String::new(),
            terminal_state: TerminalState::new(10_000),
            terminal_input: String::new(),
            pty_session,
            sys,
            cpu_usage,
            ram_used,
            ram_total,
            disk_read_bps: 0,
            disk_write_bps: 0,
            prev_diskstats: Some(DiskStats::read()),
            modal: None,
        };

        // Load initial drives
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

            Message::FocusPane(kind) => {
                self.focused_pane = kind;
                Task::none()
            }

            Message::FocusNext => {
                self.focused_pane = match self.focused_pane {
                    PaneKind::Drives => PaneKind::Browser,
                    PaneKind::Browser => PaneKind::Terminal,
                    PaneKind::Terminal => PaneKind::Drives,
                };
                Task::none()
            }

            Message::Refresh => {
                let path = self.current_path.clone();
                let files_task = Task::perform(
                    async move { ironkey_browser::listing::list_directory(&path).unwrap_or_default() },
                    Message::FilesLoaded,
                );
                let drives_task = Task::perform(
                    async { ironkey_drives::enumerate_drives().unwrap_or_default() },
                    Message::DrivesLoaded,
                );
                Task::batch([drives_task, files_task])
            }

            Message::Tick => {
                self.sys.refresh_cpu_usage();
                self.sys.refresh_memory();
                self.cpu_usage = self.sys.global_cpu_usage();
                self.ram_used = self.sys.used_memory();
                self.ram_total = self.sys.total_memory();

                // Update disk I/O rate
                let current = DiskStats::read();
                if let Some(ref prev) = self.prev_diskstats {
                    let (rd, wr) = current.rate_since(prev);
                    self.disk_read_bps = rd;
                    self.disk_write_bps = wr;
                }
                self.prev_diskstats = Some(current);

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
                    if let Some(part) = self.find_partition(dev) {
                        if part.status == ironkey_drives::PartitionStatus::Encrypted {
                            self.modal = Some(Modal::Passphrase {
                                device: dev.clone(),
                                passphrase: String::new(),
                            });
                            return Task::none();
                        }
                    }
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
                self.modal = Some(Modal::Confirm {
                    title: "Clone Partition".to_string(),
                    message: "Select a destination partition or image file in the terminal, then \
                              use: dd if=/dev/SOURCE of=/dev/DEST bs=4M status=progress"
                        .to_string(),
                });
                Task::none()
            }

            Message::WipeSelected => {
                if let Some(ref dev) = self.selected_partition.clone() {
                    self.modal = Some(Modal::Confirm {
                        title: "⚠ Wipe Partition".to_string(),
                        message: format!(
                            "Securely wipe /dev/{}? This is IRREVERSIBLE.\n\
                             All data will be permanently destroyed.",
                            dev
                        ),
                    });
                }
                Task::none()
            }

            Message::InfoSelected => {
                if let Some(ref dev) = self.selected_partition.clone() {
                    if let Some(part) = self.find_partition(dev) {
                        let part = part.clone();
                        // SMART is read from the parent drive, not the partition
                        let drive_name = drive_name_from_partition(dev);
                        let drive_path =
                            std::path::PathBuf::from(format!("/dev/{}", drive_name));
                        self.modal = Some(Modal::Info {
                            part: part.clone(),
                            smart: None,
                        });
                        let dev = dev.clone();
                        return Task::perform(
                            async move {
                                let smart = ironkey_drives::smart::read_smart(&drive_path)
                                    .unwrap_or(None);
                                (dev, smart)
                            },
                            |(dev, smart)| Message::SmartLoaded(dev, smart),
                        );
                    }
                }
                Task::none()
            }

            Message::SmartLoaded(dev, smart) => {
                let _ = dev;
                if let Some(Modal::Info { smart: ref mut modal_smart, .. }) = self.modal {
                    *modal_smart = smart;
                }
                Task::none()
            }

            Message::NewTableSelected => {
                if let Some(ref dev) = self.selected_partition.clone() {
                    // dev might be a partition; get the parent drive
                    let drive_name = drive_name_from_partition(dev);
                    self.modal = Some(Modal::NewTable {
                        device: drive_name,
                        confirm_text: String::new(),
                    });
                }
                Task::none()
            }

            Message::ModalConfirmTextChanged(s) => {
                match &mut self.modal {
                    Some(Modal::Format { ref mut confirm_text, .. }) => *confirm_text = s,
                    Some(Modal::NewTable { ref mut confirm_text, .. }) => *confirm_text = s,
                    _ => {}
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
                                let opts = ironkey_drives::ops::FormatOpts { fs_type, label: None };
                                if let Err(e) = ironkey_drives::ops::format_partition(&path, &opts) {
                                    log::error!("Format failed: {}", e);
                                }
                            }
                        }
                        Modal::NewTable { device, confirm_text } => {
                            if confirm_text == "CONFIRM" {
                                log::info!("Would create new GPT on /dev/{}", device);
                                // parted /dev/<device> mklabel gpt
                                let status = std::process::Command::new("parted")
                                    .args(["--script", &format!("/dev/{}", device), "mklabel", "gpt"])
                                    .status();
                                match status {
                                    Ok(s) if s.success() => log::info!("New GPT table created on /dev/{}", device),
                                    Ok(s) => log::error!("parted failed: {}", s),
                                    Err(e) => log::error!("parted error: {}", e),
                                }
                            }
                        }
                        Modal::Passphrase { device, passphrase } => {
                            let device_path = std::path::PathBuf::from(format!("/dev/{}", device));
                            let _ = ironkey_mount::mount(
                                &device_path,
                                ironkey_mount::FsType::Luks,
                                ironkey_mount::MountOptions {
                                    passphrase: Some(passphrase),
                                    ..Default::default()
                                },
                            );
                        }
                        Modal::Shutdown => {
                            log::info!("Shutting down…");
                            let _ = std::process::Command::new("poweroff").status();
                        }
                        Modal::Reboot => {
                            log::info!("Rebooting…");
                            let _ = std::process::Command::new("reboot").status();
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
                self.cancel_inline_edits();
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
                self.focused_pane = PaneKind::Browser;
                self.cancel_inline_edits();
                Task::none()
            }

            Message::FileActivated(idx) => {
                if let Some(entry) = self.files.get(idx) {
                    if entry.is_dir() {
                        return self.update(Message::NavigateTo(entry.path.clone()));
                    } else {
                        // Open file viewer
                        return self.update(Message::ViewFile(entry.path.clone()));
                    }
                }
                Task::none()
            }

            // ── File operations ────────────────────────────────────────────

            Message::FileCopySelected => {
                if let Some(idx) = self.selected_file {
                    if let Some(entry) = self.files.get(idx) {
                        self.clipboard = Some((entry.path.clone(), ClipboardOp::Copy));
                    }
                }
                Task::none()
            }

            Message::FileCutSelected => {
                if let Some(idx) = self.selected_file {
                    if let Some(entry) = self.files.get(idx) {
                        self.clipboard = Some((entry.path.clone(), ClipboardOp::Cut));
                    }
                }
                Task::none()
            }

            Message::FilePasteSelected => {
                if let Some((src, op)) = self.clipboard.clone() {
                    let dst = self.current_path.join(
                        src.file_name().unwrap_or_else(|| std::ffi::OsStr::new("file")),
                    );
                    match op {
                        ClipboardOp::Copy => {
                            if let Err(e) = ironkey_browser::ops::copy_file(&src, &dst) {
                                log::error!("Copy failed: {}", e);
                            }
                        }
                        ClipboardOp::Cut => {
                            if let Err(e) = ironkey_browser::ops::move_file(&src, &dst) {
                                log::error!("Move failed: {}", e);
                            }
                            self.clipboard = None;
                        }
                    }
                    let path = self.current_path.clone();
                    return Task::perform(
                        async move { ironkey_browser::listing::list_directory(&path).unwrap_or_default() },
                        Message::FilesLoaded,
                    );
                }
                Task::none()
            }

            Message::FileDeleteSelected => {
                if let Some(idx) = self.selected_file {
                    if let Some(entry) = self.files.get(idx).cloned() {
                        match ironkey_browser::ops::delete_file(&entry.path) {
                            Ok(_) => {
                                self.selected_file = None;
                                let path = self.current_path.clone();
                                return Task::perform(
                                    async move {
                                        ironkey_browser::listing::list_directory(&path).unwrap_or_default()
                                    },
                                    Message::FilesLoaded,
                                );
                            }
                            Err(e) => log::error!("Delete failed: {}", e),
                        }
                    }
                }
                Task::none()
            }

            Message::FileRenameStart => {
                if let Some(idx) = self.selected_file {
                    if let Some(entry) = self.files.get(idx) {
                        self.rename_index = Some(idx);
                        self.rename_input = entry.name.clone();
                    }
                }
                Task::none()
            }

            Message::FileRenameInputChanged(s) => {
                self.rename_input = s;
                Task::none()
            }

            Message::FileRenameCommit => {
                if let Some(idx) = self.rename_index {
                    let new_name = self.rename_input.trim().to_string();
                    if !new_name.is_empty() {
                        if let Some(entry) = self.files.get(idx) {
                            if let Err(e) = ironkey_browser::ops::rename_file(&entry.path, &new_name) {
                                log::error!("Rename failed: {}", e);
                            }
                        }
                    }
                    self.rename_index = None;
                    self.rename_input.clear();
                    let path = self.current_path.clone();
                    return Task::perform(
                        async move {
                            ironkey_browser::listing::list_directory(&path).unwrap_or_default()
                        },
                        Message::FilesLoaded,
                    );
                }
                Task::none()
            }

            Message::FileCancelInlineEdit => {
                self.cancel_inline_edits();
                Task::none()
            }

            Message::FileNewFolderStart => {
                self.new_folder_active = true;
                self.new_folder_input = String::from("New Folder");
                Task::none()
            }

            Message::FileNewFolderInputChanged(s) => {
                self.new_folder_input = s;
                Task::none()
            }

            Message::FileNewFolderCommit => {
                let name = self.new_folder_input.trim().to_string();
                if !name.is_empty() {
                    let dir_path = self.current_path.join(&name);
                    if let Err(e) = ironkey_browser::ops::create_directory(&dir_path) {
                        log::error!("New folder failed: {}", e);
                    }
                }
                self.new_folder_active = false;
                self.new_folder_input.clear();
                let path = self.current_path.clone();
                Task::perform(
                    async move {
                        ironkey_browser::listing::list_directory(&path).unwrap_or_default()
                    },
                    Message::FilesLoaded,
                )
            }

            Message::FilePropertiesSelected => {
                if let Some(idx) = self.selected_file {
                    if let Some(entry) = self.files.get(idx).cloned() {
                        self.modal = Some(Modal::Properties { entry });
                    }
                }
                Task::none()
            }

            Message::ViewFile(path) => {
                let p = path.clone();
                Task::perform(
                    async move {
                        match ironkey_browser::viewer::view_file(&p) {
                            Ok(content) => (path, content),
                            Err(_) => (
                                path.clone(),
                                ironkey_browser::viewer::ViewedContent {
                                    mode: ironkey_browser::viewer::ViewMode::Text,
                                    text: Some("(Could not read file)".to_string()),
                                    hex_lines: Vec::new(),
                                    file_size: 0,
                                    bytes_read: 0,
                                    truncated: false,
                                },
                            ),
                        }
                    },
                    |(path, content)| Message::FileViewerLoaded(path, content),
                )
            }

            Message::FileViewerLoaded(path, content) => {
                self.modal = Some(Modal::FileViewer { path, content });
                Task::none()
            }

            // ── Terminal ───────────────────────────────────────────────────

            Message::TerminalInputChanged(s) => {
                self.terminal_input = s;
                Task::none()
            }

            Message::TerminalSubmit => {
                let input = std::mem::take(&mut self.terminal_input);
                if !input.is_empty() {
                    if let Some(ref mut pty) = self.pty_session {
                        let _ = pty.write_input(format!("{}\n", input).as_bytes());
                    } else {
                        // No PTY: echo to display
                        let line = format!("$ {}\n", input);
                        self.terminal_state.feed(line.as_bytes());
                    }
                }
                Task::none()
            }

            Message::TerminalOutput(data) => {
                self.terminal_state.feed(&data);
                Task::none()
            }

            // ── Search ─────────────────────────────────────────────────────

            Message::SearchToggle => {
                self.search_open = !self.search_open;
                self.search_query.clear();
                self.focused_pane = PaneKind::Browser;
                Task::none()
            }

            Message::SearchQueryChanged(q) => {
                self.search_query = q;
                Task::none()
            }

            Message::SearchSubmit => {
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
                                name: r.path
                                    .file_name()
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

            // ── System ─────────────────────────────────────────────────────

            Message::ShutdownRequested => {
                self.modal = Some(Modal::Shutdown);
                Task::none()
            }

            Message::RebootRequested => {
                self.modal = Some(Modal::Reboot);
                Task::none()
            }
        }
    }

    /// Subscription: tick every 2 seconds + keyboard events.
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::keyboard::{self, key};

        let tick = iced::time::every(std::time::Duration::from_secs(2))
            .map(|_| Message::Tick);

        let keys = keyboard::listen().map(|event| {
            use keyboard::Event;
            match event {
                Event::KeyPressed { key, modifiers, .. } => {
                    use key::Named;
                    match key.as_ref() {
                        // Tab cycles panels
                        keyboard::Key::Named(Named::Tab) => {
                            if modifiers.shift() {
                                // Shift+Tab: reverse cycle (same behaviour for 3 panels)
                                return Some(Message::FocusNext);
                            }
                            return Some(Message::FocusNext);
                        }
                        // F5 — refresh
                        keyboard::Key::Named(Named::F5) => return Some(Message::Refresh),
                        // F2 — rename
                        keyboard::Key::Named(Named::F2) => return Some(Message::FileRenameStart),
                        // Delete — delete file
                        keyboard::Key::Named(Named::Delete) => {
                            return Some(Message::FileDeleteSelected)
                        }
                        // Escape — cancel modal / inline edit
                        keyboard::Key::Named(Named::Escape) => {
                            return Some(Message::ModalCancel)
                        }
                        // Character shortcuts with modifiers
                        keyboard::Key::Character(c) => {
                            if modifiers.control() {
                                if c == "1" { return Some(Message::FocusPane(PaneKind::Drives)); }
                                if c == "2" { return Some(Message::FocusPane(PaneKind::Browser)); }
                                if c == "3" { return Some(Message::FocusPane(PaneKind::Terminal)); }
                                if c == "q" || c == "Q" { return Some(Message::ShutdownRequested); }
                                // Ctrl+R: reboot (lowercase only — shift+r produces 'R' but with
                                // shift modifier; we intentionally treat both uniformly)
                                if c == "r" || c == "R" {
                                    return Some(Message::RebootRequested);
                                }
                                if c == "f" || c == "F" {
                                    return Some(Message::SearchToggle);
                                }
                                if c == "d" || c == "D" { return Some(Message::UnmountSelected); }
                                if c == "c" || c == "C" { return Some(Message::FileCopySelected); }
                                if c == "x" || c == "X" { return Some(Message::FileCutSelected); }
                                if c == "v" || c == "V" { return Some(Message::FilePasteSelected); }
                            }
                        }
                        _ => {}
                    }
                    None
                }
                _ => None,
            }
        })
        .filter_map(|x| x);

        Subscription::batch([tick, keys])
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

        let right = row![
            cpu,
            Space::new().width(16),
            ram,
            Space::new().width(16),
            clock,
        ]
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

        // Disk I/O
        let io_info = text(format!(
            "↑ {}  ↓ {}",
            format_rate(self.disk_write_bps),
            format_rate(self.disk_read_bps),
        ))
        .size(11)
        .color(theme::TEXT_SECONDARY);

        container(
            row![
                text(partition_info)
                    .size(12)
                    .color(theme::TEXT_SECONDARY)
                    .width(Length::FillPortion(2)),
                actions,
                Space::new().width(Length::Fill),
                io_info,
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
        let is_mounted = self.selected_partition.as_ref()
            .and_then(|d| self.find_partition(d))
            .map_or(false, |p| matches!(p.status, ironkey_drives::PartitionStatus::Mounted(_)));

        fn btn<'a>(label: &'static str, msg: Message, enabled: bool) -> Element<'a, Message> {
            let b = button(text(label).size(12));
            if enabled { b.on_press(msg) } else { b }
                .padding([3, 8])
                .into()
        }

        row![
            btn("MOUNT",     Message::MountSelected,   has_selection && !is_mounted),
            btn("UNMOUNT",   Message::UnmountSelected,  has_selection && is_mounted),
            btn("FORMAT",    Message::FormatSelected,   has_selection),
            btn("CLONE",     Message::CloneSelected,    has_selection),
            btn("WIPE",      Message::WipeSelected,     has_selection),
            btn("NEW TABLE", Message::NewTableSelected, has_selection),
            btn("INFO",      Message::InfoSelected,     has_selection),
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
            Modal::Info { part, smart } => {
                crate::modals::info::view(part, smart)
            }
            Modal::FileViewer { path, content } => {
                crate::modals::file_viewer::view(path, content)
            }
            Modal::Properties { entry } => {
                crate::modals::properties::view(entry)
            }
            Modal::Shutdown => {
                crate::modals::confirm::view(
                    "⏻ Shutdown",
                    "Power off the system? All unsaved data will be lost.",
                )
            }
            Modal::Reboot => {
                crate::modals::confirm::view(
                    "↺ Reboot",
                    "Reboot the system?",
                )
            }
            Modal::NewTable { device, confirm_text } => {
                new_table_modal_view(device, confirm_text)
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

    /// Cancel any active inline edit (rename / new folder).
    fn cancel_inline_edits(&mut self) {
        self.rename_index = None;
        self.rename_input.clear();
        self.new_folder_active = false;
        self.new_folder_input.clear();
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// New-table modal
// ──────────────────────────────────────────────────────────────────────────────

fn new_table_modal_view<'a>(device: &'a str, confirm_text: &'a str) -> Element<'a, Message> {
    use iced::widget::{column, container, row, text, text_input, button, Space};
    let title = text(format!("NEW PARTITION TABLE: /dev/{}", device))
        .size(18)
        .color(theme::ACCENT_DANGER);
    let warning = text(
        "⚠ THIS WILL ERASE THE PARTITION TABLE AND ALL DATA ON THIS DISK ⚠\n\
         A new GPT partition table will be created.\n\
         Type CONFIRM below to proceed.",
    )
    .size(13)
    .color(theme::ACCENT_WARNING);
    let input = text_input("Type CONFIRM to proceed…", confirm_text)
        .on_input(Message::ModalConfirmTextChanged)
        .on_submit(Message::ModalConfirm)
        .size(14)
        .width(Length::Fill);
    let can = confirm_text == "CONFIRM";
    let ok = if can {
        button(text("CREATE GPT TABLE").size(14).color(theme::ACCENT_DANGER))
            .on_press(Message::ModalConfirm)
            .padding([8, 16])
    } else {
        button(text("CREATE GPT TABLE").size(14).color(theme::TEXT_SECONDARY)).padding([8, 16])
    };
    let cancel = button(text("CANCEL").size(14))
        .on_press(Message::ModalCancel)
        .padding([8, 16]);
    let inner = column![
        title, Space::new().height(12), warning, Space::new().height(12),
        input, Space::new().height(12),
        row![cancel, Space::new().width(8), ok],
    ]
    .spacing(4).padding(32).width(560);
    container(inner)
        .style(|_| iced::widget::container::Style {
            background: Some(theme::BG_ELEVATED.into()),
            border: iced::Border { color: theme::ACCENT_DANGER, width: 2.0, radius: 8.0.into() },
            ..Default::default()
        })
        .into()
}

// ──────────────────────────────────────────────────────────────────────────────
// Expose these fields for panel rendering
// ──────────────────────────────────────────────────────────────────────────────

impl IronKeyApp {
    pub fn drives(&self) -> &[DriveInfo] { &self.drives }
    pub fn expanded_drives(&self) -> &std::collections::HashSet<usize> { &self.expanded_drives }
    pub fn selected_partition(&self) -> Option<&str> { self.selected_partition.as_deref() }
    pub fn current_path(&self) -> &std::path::Path { &self.current_path }
    pub fn files(&self) -> &[FileEntry] { &self.files }
    pub fn selected_file(&self) -> Option<usize> { self.selected_file }
    pub fn search_open(&self) -> bool { self.search_open }
    pub fn search_query(&self) -> &str { &self.search_query }
    pub fn terminal_state(&self) -> &TerminalState { &self.terminal_state }
    pub fn terminal_input(&self) -> &str { &self.terminal_input }
    pub fn clipboard(&self) -> Option<&(std::path::PathBuf, ClipboardOp)> { self.clipboard.as_ref() }
    pub fn rename_index(&self) -> Option<usize> { self.rename_index }
    pub fn rename_input(&self) -> &str { &self.rename_input }
    pub fn new_folder_active(&self) -> bool { self.new_folder_active }
    pub fn new_folder_input(&self) -> &str { &self.new_folder_input }
    pub fn focused_pane(&self) -> PaneKind { self.focused_pane }
}

// ──────────────────────────────────────────────────────────────────────────────
// Utility
// ──────────────────────────────────────────────────────────────────────────────

/// Strip trailing partition digit(s) to get the parent drive name.
/// e.g. "sda1" → "sda", "nvme0n1p2" → "nvme0n1", "mmcblk0p1" → "mmcblk0"
fn drive_name_from_partition(dev: &str) -> String {
    // NVMe / MMC: strip trailing "p<digits>"
    if dev.contains('p') {
        let without = dev.trim_end_matches(|c: char| c.is_ascii_digit());
        if without.ends_with('p') && without.len() > 1 {
            return without.trim_end_matches('p').to_string();
        }
    }
    // SATA/IDE: strip trailing digits
    dev.trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}
