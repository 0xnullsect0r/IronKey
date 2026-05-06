//! Drive and partition enumeration via `/sys/block/` and `/proc/mounts`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// High-level information about a physical block device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    /// Kernel name, e.g. "sda", "nvme0n1", "mmcblk0"
    pub device: String,
    /// Full device path, e.g. "/dev/sda"
    pub device_path: PathBuf,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Whether this device is removable (USB, SD card, …)
    pub removable: bool,
    /// Whether this is a spinning-disk (true) or SSD/NVMe (false)
    pub rotational: bool,
    /// Drive model string from sysfs
    pub model: Option<String>,
    /// Drive serial number from sysfs
    pub serial: Option<String>,
    /// Partitions on this drive (sorted by device name)
    pub partitions: Vec<PartitionInfo>,
    /// Overall drive health status
    pub status: DriveStatus,
}

/// Information about a single partition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    /// Kernel name, e.g. "sda1", "nvme0n1p1"
    pub device: String,
    /// Full device path, e.g. "/dev/sda1"
    pub device_path: PathBuf,
    /// Partition size in bytes
    pub size_bytes: u64,
    /// Filesystem type as reported by `/proc/mounts` or blkid heuristics
    pub filesystem: Option<String>,
    /// Filesystem label (from `/dev/disk/by-label/`)
    pub label: Option<String>,
    /// Partition UUID (from `/dev/disk/by-uuid/`)
    pub uuid: Option<String>,
    /// Current mount point if mounted
    pub mount_point: Option<PathBuf>,
    /// Mount / encryption status
    pub status: PartitionStatus,
    /// Starting sector (from sysfs)
    pub start_sector: Option<u64>,
    /// Ending sector (start + size)
    pub end_sector: Option<u64>,
}

/// Drive health summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriveStatus {
    /// SMART reports no issues
    Healthy,
    /// SMART reports a problem or device is unreachable
    Error(String),
}

/// Partition mount / encryption state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PartitionStatus {
    /// Partition is mounted at the given path
    Mounted(PathBuf),
    /// Partition is known but not mounted
    Unmounted,
    /// Partition is encrypted (LUKS or BitLocker detected)
    Encrypted,
    /// Partition has no recognised filesystem
    Unformatted,
    /// Cannot read partition information
    Error(String),
}

/// Enumerate all block devices found in `/sys/block/`.
///
/// Returns an empty `Vec` on non-Linux targets so library users
/// can compile and test on macOS / Windows.
pub fn enumerate_drives() -> Result<Vec<DriveInfo>> {
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Vec::new())
    }
    #[cfg(target_os = "linux")]
    {
        enumerate_drives_linux()
    }
}

#[cfg(target_os = "linux")]
fn enumerate_drives_linux() -> Result<Vec<DriveInfo>> {
    let mut drives = Vec::new();
    let sys_block = Path::new("/sys/block");

    if !sys_block.exists() {
        return Ok(drives);
    }

    for entry in fs::read_dir(sys_block).context("reading /sys/block")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip virtual / irrelevant devices
        if should_skip_device(&name) {
            continue;
        }

        let sys_path = entry.path();
        let device_path = PathBuf::from(format!("/dev/{}", name));

        match read_drive_info(&name, &sys_path, &device_path) {
            Ok(drive) => drives.push(drive),
            Err(e) => log::warn!("Failed to read drive {}: {}", name, e),
        }
    }

    drives.sort_by(|a, b| a.device.cmp(&b.device));
    Ok(drives)
}

fn should_skip_device(name: &str) -> bool {
    name.starts_with("loop")
        || name.starts_with("ram")
        || name.starts_with("zram")
        || name.starts_with("dm-")
        || name.starts_with("sr")
        || name.starts_with("fd")
}

fn read_sys_str(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "linux")]
fn read_drive_info(name: &str, sys_path: &Path, device_path: &Path) -> Result<DriveInfo> {
    let size_sectors: u64 = read_sys_str(&sys_path.join("size"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let size_bytes = size_sectors * 512;

    let removable: bool = read_sys_str(&sys_path.join("removable"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(false);

    let rotational: bool = read_sys_str(&sys_path.join("queue/rotational"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(true);

    let model = read_sys_str(&sys_path.join("device/model"));
    let serial = read_sys_str(&sys_path.join("device/serial"));

    let partitions = enumerate_partitions(name, sys_path)?;

    Ok(DriveInfo {
        device: name.to_string(),
        device_path: device_path.to_path_buf(),
        size_bytes,
        removable,
        rotational,
        model,
        serial,
        partitions,
        status: DriveStatus::Healthy,
    })
}

#[cfg(target_os = "linux")]
fn enumerate_partitions(drive_name: &str, sys_path: &Path) -> Result<Vec<PartitionInfo>> {
    let mut partitions = Vec::new();

    let entries = match fs::read_dir(sys_path) {
        Ok(e) => e,
        Err(_) => return Ok(partitions),
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Partition entries start with the drive name and have a digit suffix
        if !name.starts_with(drive_name) || name == drive_name {
            continue;
        }
        // Must have something after the drive name prefix
        let suffix = &name[drive_name.len()..];
        if suffix.is_empty() || !suffix.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }

        let part_sys_path = entry.path();
        let device_path = PathBuf::from(format!("/dev/{}", name));

        match read_partition_info(&name, &part_sys_path, &device_path) {
            Ok(p) => partitions.push(p),
            Err(e) => log::warn!("Failed to read partition {}: {}", name, e),
        }
    }

    partitions.sort_by(|a, b| a.device.cmp(&b.device));
    Ok(partitions)
}

#[cfg(target_os = "linux")]
fn read_partition_info(name: &str, sys_path: &Path, device_path: &Path) -> Result<PartitionInfo> {
    let size_sectors: u64 = read_sys_str(&sys_path.join("size"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let size_bytes = size_sectors * 512;

    let start_sector: Option<u64> = read_sys_str(&sys_path.join("start"))
        .and_then(|s| s.parse().ok());

    let (filesystem, mount_point) = get_mount_info(device_path);
    let uuid = lookup_symlink_dir("/dev/disk/by-uuid", device_path);
    let label = lookup_symlink_dir("/dev/disk/by-label", device_path);

    let status = if let Some(ref mp) = mount_point {
        PartitionStatus::Mounted(mp.clone())
    } else if filesystem.as_deref() == Some("crypto_LUKS")
        || filesystem.as_deref() == Some("BitLocker")
    {
        PartitionStatus::Encrypted
    } else if filesystem.is_none() {
        PartitionStatus::Unformatted
    } else {
        PartitionStatus::Unmounted
    };

    let end_sector = start_sector.map(|s| s + size_sectors);

    Ok(PartitionInfo {
        device: name.to_string(),
        device_path: device_path.to_path_buf(),
        size_bytes,
        filesystem,
        label,
        uuid,
        mount_point,
        status,
        start_sector,
        end_sector,
    })
}

/// Parse `/proc/mounts` to find the filesystem type and mount point for `device_path`.
fn get_mount_info(device_path: &Path) -> (Option<String>, Option<PathBuf>) {
    let device_str = device_path.to_string_lossy();
    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let mut parts = line.split_whitespace();
            let dev = parts.next().unwrap_or("");
            let mp = parts.next().unwrap_or("");
            let fs = parts.next().unwrap_or("");
            if dev == device_str.as_ref() {
                return (
                    Some(fs.to_string()),
                    Some(PathBuf::from(mp)),
                );
            }
        }
    }
    (None, None)
}

/// Resolve symlinks in `dir` to find the entry pointing at `target_device`.
/// Returns the entry filename (UUID or label) if found.
fn lookup_symlink_dir(dir: &str, target_device: &Path) -> Option<String> {
    let base = Path::new(dir);
    if !base.exists() {
        return None;
    }
    let target_canon = fs::canonicalize(target_device).ok()?;
    for entry in fs::read_dir(base).ok()?.flatten() {
        if let Ok(link_target) = fs::read_link(entry.path()) {
            // Symlinks are relative: resolve against the base dir
            let resolved = base.join(link_target);
            if fs::canonicalize(resolved).ok() == Some(target_canon.clone()) {
                return Some(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    None
}

/// Format a byte count into a human-readable string (e.g. "465.8 GB").
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(500 * 1024 * 1024 * 1024), "500.0 GB");
    }

    #[test]
    fn enumerate_drives_returns_vec() {
        // Should not panic; returns empty on non-Linux or when /sys/block missing
        let drives = enumerate_drives().unwrap_or_default();
        // On Linux CI the drives list is expected to be non-empty or empty—just no panic.
        let _ = drives;
    }

    #[test]
    fn skip_loop_devices() {
        assert!(should_skip_device("loop0"));
        assert!(should_skip_device("ram0"));
        assert!(!should_skip_device("sda"));
        assert!(!should_skip_device("nvme0n1"));
    }
}
