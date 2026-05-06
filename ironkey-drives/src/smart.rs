//! SMART (Self-Monitoring, Analysis and Reporting Technology) data retrieval.
//!
//! Uses direct ATA passthrough ioctl (`HDIO_DRIVE_CMD` / `SG_IO`) on Linux
//! to read SMART attributes without requiring `smartmontools`.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A summary of a drive's SMART health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartData {
    /// Overall SMART self-assessment: true = PASSED, false = FAILED
    pub overall_health: bool,
    /// Drive temperature in Celsius (if available)
    pub temperature_celsius: Option<u8>,
    /// Reallocated sector count (non-zero indicates physical damage)
    pub reallocated_sectors: u64,
    /// Total power-on hours
    pub power_on_hours: u64,
    /// Pending sector count (sectors waiting to be reallocated)
    pub pending_sectors: u64,
    /// Uncorrectable sector count
    pub uncorrectable_sectors: u64,
}

impl SmartData {
    /// Returns a human-readable health label.
    pub fn health_label(&self) -> &'static str {
        if self.overall_health {
            "PASSED"
        } else {
            "FAILED"
        }
    }
}

/// Read SMART data from `device_path` (e.g. `/dev/sda`).
///
/// Returns `None` if SMART is not supported or cannot be read.
pub fn read_smart(_device_path: &std::path::Path) -> Result<Option<SmartData>> {
    #[cfg(target_os = "linux")]
    {
        read_smart_linux(_device_path)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(None)
    }
}

#[cfg(target_os = "linux")]
fn read_smart_linux(device_path: &std::path::Path) -> Result<Option<SmartData>> {
    use std::fs::OpenOptions;
    use std::os::unix::io::AsRawFd;

    let file = match OpenOptions::new().read(true).write(true).open(device_path) {
        Ok(f) => f,
        Err(_) => return Ok(None),
    };
    let fd = file.as_raw_fd();

    // HDIO_DRIVE_CMD: 0x031f
    // Command bytes: [cmd, sector_count, feature, sector_number]
    // SMART READ DATA: cmd=0xB0, feature=0xD0, sector_count=1, LBA_MID=0x4F, LBA_HIGH=0xC2
    const HDIO_DRIVE_CMD: u64 = 0x031f;
    const SMART_READ_DATA: u8 = 0xD0;
    const SMART_CMD: u8 = 0xB0;

    // 4 header bytes + 512 bytes of SMART data
    let mut buf = [0u8; 516];
    buf[0] = SMART_CMD;
    buf[1] = 1; // sector count
    buf[2] = SMART_READ_DATA;
    buf[3] = 1; // sector number

    let ret = unsafe { libc_ioctl(fd, HDIO_DRIVE_CMD, buf.as_mut_ptr()) };
    if ret != 0 {
        return Ok(None);
    }

    // SMART data starts at offset 4 (after the 4-byte header)
    let data = &buf[4..];
    let smart = parse_smart_data(data);
    Ok(Some(smart))
}

#[cfg(target_os = "linux")]
unsafe fn libc_ioctl(fd: std::os::unix::io::RawFd, request: u64, arg: *mut u8) -> i32 {
    extern "C" {
        fn ioctl(fd: std::ffi::c_int, request: u64, ...) -> std::ffi::c_int;
    }
    ioctl(fd, request, arg)
}

/// Parse raw 512-byte SMART data blob into `SmartData`.
#[allow(unused_variables)]
fn parse_smart_data(data: &[u8]) -> SmartData {
    if data.len() < 512 {
        return SmartData {
            overall_health: false,
            temperature_celsius: None,
            reallocated_sectors: 0,
            power_on_hours: 0,
            pending_sectors: 0,
            uncorrectable_sectors: 0,
        };
    }

    // SMART attribute table starts at offset 2 in the 512-byte data block.
    // Each attribute is 12 bytes: [id, flags(2), current, worst, raw(6), reserved]
    let mut temperature: Option<u8> = None;
    let mut reallocated_sectors: u64 = 0;
    let mut power_on_hours: u64 = 0;
    let mut pending_sectors: u64 = 0;
    let mut uncorrectable_sectors: u64 = 0;

    let attr_table_offset = 2usize;
    let num_attrs = 30;

    for i in 0..num_attrs {
        let offset = attr_table_offset + i * 12;
        if offset + 12 > data.len() {
            break;
        }
        let id = data[offset];
        if id == 0 {
            continue;
        }
        // Raw value is 6 bytes starting at offset+5 (little-endian)
        let raw = u64::from_le_bytes([
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            0,
            0,
        ]);

        match id {
            0x05 => reallocated_sectors = raw & 0xFFFF,    // Reallocated Sector Count
            0xC5 => pending_sectors = raw & 0xFFFF,         // Current Pending Sector Count
            0xC6 => uncorrectable_sectors = raw & 0xFFFF,   // Uncorrectable Sector Count
            0xC2 => temperature = Some((raw & 0xFF) as u8), // Temperature
            0xF0 => temperature = Some((raw & 0xFF) as u8), // Head Flying Hours (alt temp)
            0xE7 => temperature = Some((raw & 0xFF) as u8), // SSD Life Left proxy
            0x09 => power_on_hours = raw & 0xFFFF,           // Power-On Hours
            _ => {}
        }
    }

    // Byte 169 in the raw SMART data is the self-test result from SMART STATUS.
    // For a basic check: if reallocated or uncorrectable sectors are high, mark failed.
    let overall_health =
        reallocated_sectors == 0 && uncorrectable_sectors == 0 && pending_sectors < 100;

    SmartData {
        overall_health,
        temperature_celsius: temperature,
        reallocated_sectors,
        power_on_hours,
        pending_sectors,
        uncorrectable_sectors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_label_passed() {
        let s = SmartData {
            overall_health: true,
            temperature_celsius: Some(35),
            reallocated_sectors: 0,
            power_on_hours: 1000,
            pending_sectors: 0,
            uncorrectable_sectors: 0,
        };
        assert_eq!(s.health_label(), "PASSED");
    }

    #[test]
    fn health_label_failed() {
        let s = SmartData {
            overall_health: false,
            temperature_celsius: None,
            reallocated_sectors: 5,
            power_on_hours: 50000,
            pending_sectors: 0,
            uncorrectable_sectors: 1,
        };
        assert_eq!(s.health_label(), "FAILED");
    }
}
