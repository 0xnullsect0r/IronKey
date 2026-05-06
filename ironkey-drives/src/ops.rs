//! Disk partition operations: format, clone, and wipe.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

// ──────────────────────────────────────────────────────────────────────────────
// Public types
// ──────────────────────────────────────────────────────────────────────────────

/// Options for formatting a partition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatOpts {
    /// Target filesystem type
    pub fs_type: FormatFsType,
    /// Optional filesystem label
    pub label: Option<String>,
}

/// Filesystem types supported for formatting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FormatFsType {
    Ext4,
    Fat32,
    ExFat,
    Ntfs,
    Btrfs,
    Xfs,
}

impl FormatFsType {
    pub fn mkfs_binary(&self) -> &'static str {
        match self {
            Self::Ext4 => "mkfs.ext4",
            Self::Fat32 => "mkfs.fat",
            Self::ExFat => "mkfs.exfat",
            Self::Ntfs => "mkfs.ntfs",
            Self::Btrfs => "mkfs.btrfs",
            Self::Xfs => "mkfs.xfs",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Ext4 => "ext4",
            Self::Fat32 => "FAT32",
            Self::ExFat => "exFAT",
            Self::Ntfs => "NTFS",
            Self::Btrfs => "btrfs",
            Self::Xfs => "XFS",
        }
    }
}

/// Progress update for a long-running disk operation.
#[derive(Debug, Clone)]
pub struct CloneProgress {
    /// Bytes transferred so far
    pub bytes_done: u64,
    /// Total bytes to transfer (0 if unknown)
    pub bytes_total: u64,
    /// Current transfer rate in bytes per second
    pub rate_bps: u64,
    /// Estimated seconds remaining (None if unknown)
    pub eta_secs: Option<u64>,
}

impl CloneProgress {
    /// Returns a fraction 0.0–1.0, or 0.0 if total is unknown.
    pub fn fraction(&self) -> f32 {
        if self.bytes_total == 0 {
            0.0
        } else {
            (self.bytes_done as f64 / self.bytes_total as f64) as f32
        }
    }
}

/// Secure wipe mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WipeMode {
    /// Single-pass zero fill (fast)
    Zeros,
    /// Single-pass random data (better for HDDs)
    Random,
    /// DoD 5220.22-M three-pass wipe
    Dod3Pass,
}

// ──────────────────────────────────────────────────────────────────────────────
// Operations
// ──────────────────────────────────────────────────────────────────────────────

/// Format `device_path` with the given filesystem options.
///
/// Requires root. The device **must** be unmounted before calling.
pub fn format_partition(device_path: &Path, opts: &FormatOpts) -> Result<()> {
    let binary = opts.fs_type.mkfs_binary();
    let mut cmd = Command::new(binary);

    // Add label if provided
    if let Some(ref label) = opts.label {
        match opts.fs_type {
            FormatFsType::Ext4 | FormatFsType::Btrfs | FormatFsType::Xfs => {
                cmd.args(["-L", label]);
            }
            FormatFsType::Fat32 => {
                cmd.args(["-n", label]);
            }
            FormatFsType::ExFat => {
                cmd.args(["-n", label]);
            }
            FormatFsType::Ntfs => {
                cmd.args(["-L", label]);
            }
        }
    }

    // Force / suppress prompts
    match opts.fs_type {
        FormatFsType::Ext4 => {
            cmd.arg("-F");
        }
        FormatFsType::Ntfs => {
            cmd.arg("-Q"); // quick format
        }
        FormatFsType::Btrfs => {
            cmd.arg("-f");
        }
        FormatFsType::Xfs => {
            cmd.arg("-f");
        }
        _ => {}
    }

    cmd.arg(device_path);

    log::info!("Running: {:?}", cmd);
    let status = cmd.status().with_context(|| format!("Failed to run {}", binary))?;

    if !status.success() {
        anyhow::bail!(
            "{} failed on {} (exit: {})",
            binary,
            device_path.display(),
            status
        );
    }
    Ok(())
}

/// Clone `source` partition to `destination` using block-level copy.
///
/// Calls a closure with progress updates. Both devices must be accessible;
/// the destination must be at least as large as the source.
pub fn clone_partition<F>(source: &Path, dest: &Path, mut on_progress: F) -> Result<()>
where
    F: FnMut(CloneProgress),
{
    use std::io::{Read, Write};
    use std::time::Instant;

    let source_size = get_block_device_size(source).unwrap_or(0);
    let mut src = std::fs::File::open(source)
        .with_context(|| format!("Opening source {}", source.display()))?;
    let mut dst = std::fs::OpenOptions::new()
        .write(true)
        .open(dest)
        .with_context(|| format!("Opening destination {}", dest.display()))?;

    const BUF_SIZE: usize = 4 * 1024 * 1024; // 4 MiB blocks
    let mut buf = vec![0u8; BUF_SIZE];
    let mut bytes_done: u64 = 0;
    let start = Instant::now();

    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])?;
        bytes_done += n as u64;

        let elapsed = start.elapsed().as_secs_f64().max(0.001);
        let rate_bps = (bytes_done as f64 / elapsed) as u64;
        let eta_secs = if rate_bps > 0 && source_size > bytes_done {
            Some((source_size - bytes_done) / rate_bps)
        } else {
            None
        };

        on_progress(CloneProgress {
            bytes_done,
            bytes_total: source_size,
            rate_bps,
            eta_secs,
        });
    }

    dst.flush()?;
    Ok(())
}

/// Securely wipe `device_path` using the given `WipeMode`.
pub fn wipe_partition<F>(device_path: &Path, mode: WipeMode, mut on_progress: F) -> Result<()>
where
    F: FnMut(CloneProgress),
{
    match mode {
        WipeMode::Zeros => wipe_with_fill(device_path, 0x00, &mut on_progress),
        WipeMode::Random => wipe_with_random(device_path, &mut on_progress),
        WipeMode::Dod3Pass => {
            wipe_with_fill(device_path, 0x00, &mut on_progress)?;
            wipe_with_fill(device_path, 0xFF, &mut on_progress)?;
            wipe_with_random(device_path, &mut on_progress)
        }
    }
}

fn wipe_with_fill<F>(device_path: &Path, byte: u8, on_progress: &mut F) -> Result<()>
where
    F: FnMut(CloneProgress),
{
    use std::io::Write;
    use std::time::Instant;

    let device_size = get_block_device_size(device_path).unwrap_or(0);
    let mut dst = std::fs::OpenOptions::new()
        .write(true)
        .open(device_path)
        .with_context(|| format!("Opening {} for wipe", device_path.display()))?;

    const BUF_SIZE: usize = 4 * 1024 * 1024;
    let buf = vec![byte; BUF_SIZE];
    let mut bytes_done: u64 = 0;
    let start = Instant::now();

    loop {
        let remaining = device_size.saturating_sub(bytes_done);
        if remaining == 0 {
            break;
        }
        let to_write = (remaining as usize).min(BUF_SIZE);
        match dst.write(&buf[..to_write]) {
            Ok(0) => break,
            Ok(n) => bytes_done += n as u64,
            Err(e) if e.kind() == std::io::ErrorKind::WriteZero => break,
            Err(e) => return Err(e.into()),
        }

        let elapsed = start.elapsed().as_secs_f64().max(0.001);
        let rate_bps = (bytes_done as f64 / elapsed) as u64;
        on_progress(CloneProgress {
            bytes_done,
            bytes_total: device_size,
            rate_bps,
            eta_secs: if rate_bps > 0 {
                Some(remaining / rate_bps)
            } else {
                None
            },
        });
    }
    dst.flush()?;
    Ok(())
}

fn wipe_with_random<F>(device_path: &Path, on_progress: &mut F) -> Result<()>
where
    F: FnMut(CloneProgress),
{
    use std::io::{Read, Write};
    use std::time::Instant;

    let device_size = get_block_device_size(device_path).unwrap_or(0);
    let mut urandom = std::fs::File::open("/dev/urandom")?;
    let mut dst = std::fs::OpenOptions::new()
        .write(true)
        .open(device_path)
        .with_context(|| format!("Opening {} for random wipe", device_path.display()))?;

    const BUF_SIZE: usize = 4 * 1024 * 1024;
    let mut buf = vec![0u8; BUF_SIZE];
    let mut bytes_done: u64 = 0;
    let start = Instant::now();

    loop {
        let remaining = device_size.saturating_sub(bytes_done);
        if remaining == 0 {
            break;
        }
        let to_read = (remaining as usize).min(BUF_SIZE);
        urandom.read_exact(&mut buf[..to_read])?;
        match dst.write(&buf[..to_read]) {
            Ok(0) => break,
            Ok(n) => bytes_done += n as u64,
            Err(e) if e.kind() == std::io::ErrorKind::WriteZero => break,
            Err(e) => return Err(e.into()),
        }

        let elapsed = start.elapsed().as_secs_f64().max(0.001);
        let rate_bps = (bytes_done as f64 / elapsed) as u64;
        on_progress(CloneProgress {
            bytes_done,
            bytes_total: device_size,
            rate_bps,
            eta_secs: if rate_bps > 0 {
                Some(remaining / rate_bps)
            } else {
                None
            },
        });
    }
    dst.flush()?;
    Ok(())
}

/// Read block device size from `/sys/block/<name>/size` (in 512-byte sectors).
fn get_block_device_size(device_path: &Path) -> Option<u64> {
    // Try sysfs first
    let name = device_path.file_name()?.to_string_lossy().to_string();
    let sysfs_size = Path::new("/sys/block").join(&name).join("size");
    if let Some(s) = std::fs::read_to_string(&sysfs_size)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
    {
        return Some(s * 512);
    }

    // Fall back to BLKGETSIZE64 ioctl
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::os::unix::io::AsRawFd;
        const BLKGETSIZE64: u64 = 0x80081272;
        let file = File::open(device_path).ok()?;
        let fd = file.as_raw_fd();
        let mut size: u64 = 0;
        let ret =
            unsafe { libc_ioctl_u64(fd, BLKGETSIZE64, &mut size as *mut u64 as *mut libc_c_void) };
        if ret == 0 { Some(size) } else { None }
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
unsafe fn libc_ioctl_u64(
    fd: std::os::unix::io::RawFd,
    request: u64,
    arg: *mut libc_c_void,
) -> i32 {
    extern "C" {
        fn ioctl(fd: std::ffi::c_int, request: u64, ...) -> std::ffi::c_int;
    }
    ioctl(fd, request, arg)
}

#[cfg(target_os = "linux")]
type libc_c_void = std::ffi::c_void;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_opts_display() {
        let opts = FormatOpts {
            fs_type: FormatFsType::Ext4,
            label: Some("test".to_string()),
        };
        assert_eq!(opts.fs_type.display_name(), "ext4");
        assert_eq!(opts.fs_type.mkfs_binary(), "mkfs.ext4");
    }

    #[test]
    fn clone_progress_fraction() {
        let p = CloneProgress {
            bytes_done: 512,
            bytes_total: 1024,
            rate_bps: 100,
            eta_secs: Some(5),
        };
        assert!((p.fraction() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn clone_progress_fraction_zero_total() {
        let p = CloneProgress {
            bytes_done: 0,
            bytes_total: 0,
            rate_bps: 0,
            eta_secs: None,
        };
        assert_eq!(p.fraction(), 0.0);
    }
}
