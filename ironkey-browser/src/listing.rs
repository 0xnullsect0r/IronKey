//! Directory listing and file metadata.

use anyhow::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// The kind of a filesystem entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    Directory,
    RegularFile,
    Symlink,
    Other,
}

/// A single entry in a directory listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File or directory name
    pub name: String,
    /// Full path
    pub path: std::path::PathBuf,
    /// File kind
    pub kind: FileKind,
    /// Size in bytes (0 for directories)
    pub size: u64,
    /// Last modification time
    pub modified: Option<DateTime<Local>>,
    /// File permissions as a Unix octal string, e.g. "rwxr-xr-x"
    pub permissions: String,
    /// Extension (lowercase, without the dot)
    pub extension: Option<String>,
}

impl FileEntry {
    /// Returns true if this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.kind == FileKind::Directory
    }

    /// Returns a short human-readable size string.
    pub fn size_display(&self) -> String {
        if self.is_dir() {
            return String::new();
        }
        format_size(self.size)
    }
}

/// List the contents of `directory`, sorted: directories first, then files,
/// both groups sorted alphabetically (case-insensitive).
pub fn list_directory(directory: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let kind = if metadata.is_dir() {
            FileKind::Directory
        } else if metadata.is_symlink() {
            FileKind::Symlink
        } else if metadata.is_file() {
            FileKind::RegularFile
        } else {
            FileKind::Other
        };

        let modified = metadata
            .modified()
            .ok()
            .map(|t| DateTime::<Local>::from(t));

        let permissions = format_permissions(&metadata);
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());

        entries.push(FileEntry {
            name,
            path,
            kind,
            size: if metadata.is_file() { metadata.len() } else { 0 },
            modified,
            permissions,
            extension,
        });
    }

    // Sort: directories first, then by name (case-insensitive)
    entries.sort_by(|a, b| {
        match (a.is_dir(), b.is_dir()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut idx = 0;
    while size >= 1024.0 && idx < UNITS.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[idx])
    }
}

fn format_permissions(meta: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        let chars = [
            if mode & 0o400 != 0 { 'r' } else { '-' },
            if mode & 0o200 != 0 { 'w' } else { '-' },
            if mode & 0o100 != 0 { 'x' } else { '-' },
            if mode & 0o040 != 0 { 'r' } else { '-' },
            if mode & 0o020 != 0 { 'w' } else { '-' },
            if mode & 0o010 != 0 { 'x' } else { '-' },
            if mode & 0o004 != 0 { 'r' } else { '-' },
            if mode & 0o002 != 0 { 'w' } else { '-' },
            if mode & 0o001 != 0 { 'x' } else { '-' },
        ];
        chars.iter().collect()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        "---------".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tmp_directory() {
        let entries = list_directory(std::path::Path::new("/tmp")).unwrap_or_default();
        // /tmp should be listable on Linux
        let _ = entries;
    }

    #[test]
    fn format_size_cases() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn file_entry_size_display_dir() {
        let entry = FileEntry {
            name: "testdir".to_string(),
            path: std::path::PathBuf::from("/tmp/testdir"),
            kind: FileKind::Directory,
            size: 0,
            modified: None,
            permissions: "rwxr-xr-x".to_string(),
            extension: None,
        };
        assert_eq!(entry.size_display(), "");
    }
}
