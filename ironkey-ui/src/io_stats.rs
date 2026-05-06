//! Disk I/O statistics from `/proc/diskstats`.

use std::time::Instant;

/// A snapshot of `/proc/diskstats` for all block devices.
#[derive(Debug, Clone)]
pub struct DiskStats {
    /// Total 512-byte sectors read across all physical drives
    pub sectors_read: u64,
    /// Total 512-byte sectors written across all physical drives
    pub sectors_written: u64,
    /// Timestamp when this snapshot was taken
    pub timestamp: Instant,
}

impl DiskStats {
    /// Read the current aggregate disk stats from `/proc/diskstats`.
    ///
    /// Returns a zeroed snapshot on non-Linux targets or if the file is unreadable.
    pub fn read() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self::read_linux()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Self { sectors_read: 0, sectors_written: 0, timestamp: Instant::now() }
        }
    }

    #[cfg(target_os = "linux")]
    fn read_linux() -> Self {
        let mut sectors_read: u64 = 0;
        let mut sectors_written: u64 = 0;

        if let Ok(content) = std::fs::read_to_string("/proc/diskstats") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // /proc/diskstats columns (0-indexed):
                // 0=major 1=minor 2=name 3=rd_ios 4=rd_merges 5=rd_sectors
                // 6=rd_time 7=wr_ios 8=wr_merges 9=wr_sectors ...
                if parts.len() < 10 {
                    continue;
                }
                let name = parts[2];
                // Skip partitions (e.g. sda1) and virtual devices
                // Physical drives: sda, nvme0n1, mmcblk0 — no trailing digits after prefix
                if !is_physical_drive(name) {
                    continue;
                }
                let rd: u64 = parts[5].parse().unwrap_or(0);
                let wr: u64 = parts[9].parse().unwrap_or(0);
                sectors_read = sectors_read.saturating_add(rd);
                sectors_written = sectors_written.saturating_add(wr);
            }
        }

        Self { sectors_read, sectors_written, timestamp: Instant::now() }
    }

    /// Compute bytes/second rates since `prev` was taken.
    ///
    /// Returns `(read_bps, write_bps)`.
    pub fn rate_since(&self, prev: &DiskStats) -> (u64, u64) {
        let elapsed = self.timestamp.duration_since(prev.timestamp).as_secs_f64().max(0.001);
        let rd_sectors = self.sectors_read.saturating_sub(prev.sectors_read);
        let wr_sectors = self.sectors_written.saturating_sub(prev.sectors_written);
        let read_bps = (rd_sectors as f64 * 512.0 / elapsed) as u64;
        let write_bps = (wr_sectors as f64 * 512.0 / elapsed) as u64;
        (read_bps, write_bps)
    }
}

/// Returns true for top-level physical block devices (not partition sub-entries).
fn is_physical_drive(name: &str) -> bool {
    if name.starts_with("loop")
        || name.starts_with("ram")
        || name.starts_with("dm-")
        || name.starts_with("sr")
        || name.starts_with("fd")
        || name.starts_with("zram")
    {
        return false;
    }
    // For nvme: nvme0n1 is a drive; nvme0n1p1 is a partition
    // For mmcblk: mmcblk0 is a drive; mmcblk0p1 is a partition
    // For sda/hda: sda is a drive; sda1 is a partition
    !name.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false)
        || name.starts_with("nvme")
        || name.starts_with("mmcblk")
}

/// Format a bytes-per-second rate for display.
pub fn format_rate(bps: u64) -> String {
    if bps == 0 {
        return "0 B/s".to_string();
    } else if bps >= 1_073_741_824 {
        format!("{:.1} GB/s", bps as f64 / 1_073_741_824.0)
    } else if bps >= 1_048_576 {
        format!("{:.1} MB/s", bps as f64 / 1_048_576.0)
    } else if bps >= 1024 {
        format!("{:.0} KB/s", bps as f64 / 1024.0)
    } else {
        format!("{} B/s", bps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_rate_zero() {
        assert_eq!(format_rate(0), "0 B/s");
    }

    #[test]
    fn format_rate_megabytes() {
        assert_eq!(format_rate(10 * 1024 * 1024), "10.0 MB/s");
    }

    #[test]
    fn disk_stats_read_no_panic() {
        let s = DiskStats::read();
        let _ = s;
    }

    #[test]
    fn is_physical_drive_checks() {
        assert!(is_physical_drive("sda"));
        assert!(!is_physical_drive("sda1"));
        assert!(is_physical_drive("nvme0n1"));
        assert!(!is_physical_drive("loop0"));
    }
}
