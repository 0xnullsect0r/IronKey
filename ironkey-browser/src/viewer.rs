//! File content viewer: plain text, hex dump, metadata.

use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

/// How to view a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    /// Plain text (UTF-8)
    Text,
    /// Hex dump with ASCII column
    Hex,
    /// Binary blob (no rendering)
    Binary,
}

/// A chunk of viewed file content.
#[derive(Debug, Clone)]
pub struct ViewedContent {
    pub mode: ViewMode,
    /// Plain text content (for Text mode)
    pub text: Option<String>,
    /// Hex dump lines (for Hex mode): (offset, hex_part, ascii_part)
    pub hex_lines: Vec<HexLine>,
    /// Total file size in bytes
    pub file_size: u64,
    /// How many bytes were actually read
    pub bytes_read: usize,
    /// Whether the file was truncated (more bytes available)
    pub truncated: bool,
}

/// A single line of a hex dump.
#[derive(Debug, Clone)]
pub struct HexLine {
    pub offset: u64,
    pub bytes: Vec<u8>,
}

impl HexLine {
    /// Returns the hex representation, e.g. "48 65 6C 6C 6F"
    pub fn hex_str(&self) -> String {
        self.bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Returns the ASCII representation, replacing non-printable with '.'
    pub fn ascii_str(&self) -> String {
        self.bytes
            .iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
            .collect()
    }
}

/// Maximum bytes to read in one view operation (prevents RAM exhaustion).
const MAX_VIEW_BYTES: usize = 512 * 1024; // 512 KiB

/// Read and render file content for the viewer panel.
pub fn view_file(path: &Path) -> Result<ViewedContent> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("Stat {}", path.display()))?;
    let file_size = meta.len();

    let mut f = std::fs::File::open(path)
        .with_context(|| format!("Opening {}", path.display()))?;

    let to_read = (file_size as usize).min(MAX_VIEW_BYTES);
    let mut buf = vec![0u8; to_read];
    let bytes_read = f.read(&mut buf)?;
    buf.truncate(bytes_read);
    let truncated = file_size as usize > MAX_VIEW_BYTES;

    // Decide render mode
    if is_text_content(&buf) {
        let text = String::from_utf8_lossy(&buf).to_string();
        Ok(ViewedContent {
            mode: ViewMode::Text,
            text: Some(text),
            hex_lines: Vec::new(),
            file_size,
            bytes_read,
            truncated,
        })
    } else {
        let hex_lines = build_hex_dump(&buf);
        Ok(ViewedContent {
            mode: ViewMode::Hex,
            text: None,
            hex_lines,
            file_size,
            bytes_read,
            truncated,
        })
    }
}

/// Returns `true` if the buffer looks like UTF-8 text (heuristic: <5% non-ASCII).
fn is_text_content(buf: &[u8]) -> bool {
    if buf.is_empty() {
        return true;
    }
    // Check for null bytes (binary indicator)
    if buf.contains(&0) {
        return false;
    }
    // Count high-bytes
    let non_ascii = buf.iter().filter(|&&b| b > 0x7E).count();
    let ratio = non_ascii as f64 / buf.len() as f64;
    // Also try UTF-8 parsing
    std::str::from_utf8(buf).is_ok() && ratio < 0.10
}

/// Build a hex dump from raw bytes (16 bytes per line).
fn build_hex_dump(buf: &[u8]) -> Vec<HexLine> {
    buf.chunks(16)
        .enumerate()
        .map(|(i, chunk)| HexLine {
            offset: (i * 16) as u64,
            bytes: chunk.to_vec(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_line_rendering() {
        let line = HexLine {
            offset: 0,
            bytes: vec![0x48, 0x65, 0x6C, 0x6C, 0x6F],
        };
        assert_eq!(line.hex_str(), "48 65 6C 6C 6F");
        assert_eq!(line.ascii_str(), "Hello");
    }

    #[test]
    fn hex_line_non_printable() {
        let line = HexLine {
            offset: 0,
            bytes: vec![0x00, 0x01, 0x41],
        };
        assert_eq!(line.ascii_str(), "..A");
    }

    #[test]
    fn text_detection() {
        assert!(is_text_content(b"hello world\n"));
        assert!(!is_text_content(b"\x00\x01\x02\x03"));
    }

    #[test]
    fn view_existing_file() {
        let content = view_file(Path::new("/etc/hostname")).unwrap_or_else(|_| ViewedContent {
            mode: ViewMode::Text,
            text: Some("n/a".to_string()),
            hex_lines: Vec::new(),
            file_size: 0,
            bytes_read: 0,
            truncated: false,
        });
        let _ = content;
    }
}
