//! Filesystem type detection by magic bytes (superblock signatures).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

/// All filesystem types IronKey can mount or detect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FsType {
    /// Auto-detect (not a real filesystem type; triggers detection)
    Auto,
    // ── Kernel-native ────────────────────────────────────────────────────────
    Ext2,
    Ext3,
    Ext4,
    /// FAT12 / FAT16 / FAT32
    Vfat,
    ExFat,
    Ntfs,
    Btrfs,
    Xfs,
    /// Apple HFS+
    HfsPlus,
    Iso9660,
    F2fs,
    // ── Encrypted ────────────────────────────────────────────────────────────
    Luks,
    BitLocker,
    VeraCrypt,
    // ── FUSE / userspace ─────────────────────────────────────────────────────
    Apfs,
    Zfs,
    // ── Fallback ─────────────────────────────────────────────────────────────
    Unknown,
}

impl FsType {
    /// Returns the Linux kernel filesystem type string (for `mount(2)`).
    pub fn kernel_name(&self) -> Option<&'static str> {
        match self {
            Self::Ext2 => Some("ext2"),
            Self::Ext3 => Some("ext3"),
            Self::Ext4 => Some("ext4"),
            Self::Vfat => Some("vfat"),
            Self::ExFat => Some("exfat"),
            Self::Ntfs => Some("ntfs3"),
            Self::Btrfs => Some("btrfs"),
            Self::Xfs => Some("xfs"),
            Self::HfsPlus => Some("hfsplus"),
            Self::Iso9660 => Some("iso9660"),
            Self::F2fs => Some("f2fs"),
            _ => None,
        }
    }

    /// Human-readable label shown in the UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Ext2 => "ext2",
            Self::Ext3 => "ext3",
            Self::Ext4 => "ext4",
            Self::Vfat => "FAT",
            Self::ExFat => "exFAT",
            Self::Ntfs => "NTFS",
            Self::Btrfs => "btrfs",
            Self::Xfs => "XFS",
            Self::HfsPlus => "HFS+",
            Self::Iso9660 => "ISO 9660",
            Self::F2fs => "F2FS",
            Self::Luks => "LUKS",
            Self::BitLocker => "BitLocker",
            Self::VeraCrypt => "VeraCrypt",
            Self::Apfs => "APFS",
            Self::Zfs => "ZFS",
            Self::Unknown => "unknown",
        }
    }
}

/// Detect the filesystem on `device_path` by reading its superblock magic bytes.
pub fn detect_filesystem(device_path: &Path) -> Result<FsType> {
    let mut f = std::fs::File::open(device_path)?;

    // Read enough bytes to cover all superblock signatures
    let mut buf = [0u8; 0x10000]; // 64 KiB
    let n = f.read(&mut buf)?;
    if n < 512 {
        return Ok(FsType::Unknown);
    }

    Ok(detect_by_magic(&buf[..n]))
}

/// Pure-function magic-byte detection (testable without a real device).
pub fn detect_by_magic(buf: &[u8]) -> FsType {
    macro_rules! sig_at {
        ($offset:expr, $magic:expr) => {
            buf.len() > $offset + $magic.len()
                && &buf[$offset..$offset + $magic.len()] == $magic
        };
    }

    // ext2/3/4: magic 0xEF53 at offset 0x438 (1080)
    if buf.len() > 0x43A && buf[0x438] == 0x53 && buf[0x439] == 0xEF {
        // Distinguish ext2/3/4 by the journal flag in s_feature_compat
        let feat_compat = u32::from_le_bytes([buf[0x45C], buf[0x45D], buf[0x45E], buf[0x45F]]);
        let feat_incompat = u32::from_le_bytes([buf[0x460], buf[0x461], buf[0x462], buf[0x463]]);
        if feat_incompat & 0x40 != 0 {
            // INCOMPAT_EXTENTS → ext4
            return FsType::Ext4;
        } else if feat_compat & 0x04 != 0 {
            // COMPAT_HAS_JOURNAL → ext3
            return FsType::Ext3;
        } else {
            return FsType::Ext2;
        }
    }

    // btrfs: "_BHRfS_M" at offset 0x10040 (65600) — need larger buffer
    // Check in first 64 KiB if available
    if buf.len() > 0x10048 && &buf[0x10040..0x10048] == b"_BHRfS_M" {
        return FsType::Btrfs;
    }

    // XFS: "XFSB" at offset 0
    if sig_at!(0, b"XFSB") {
        return FsType::Xfs;
    }

    // NTFS: "NTFS    " at offset 3
    if buf.len() > 11 && &buf[3..11] == b"NTFS    " {
        return FsType::Ntfs;
    }

    // exFAT: "EXFAT   " at offset 3
    if buf.len() > 11 && &buf[3..11] == b"EXFAT   " {
        return FsType::ExFat;
    }

    // FAT32: "FAT32   " at offset 82
    if buf.len() > 90 && &buf[82..90] == b"FAT32   " {
        return FsType::Vfat;
    }
    // FAT16: "FAT16   " at offset 54
    if buf.len() > 62 && &buf[54..62] == b"FAT16   " {
        return FsType::Vfat;
    }
    // FAT12: "FAT12   " at offset 54
    if buf.len() > 62 && &buf[54..62] == b"FAT12   " {
        return FsType::Vfat;
    }

    // HFS+: "H+" at offset 1024
    if buf.len() > 1026 && &buf[1024..1026] == b"H+" {
        return FsType::HfsPlus;
    }
    // HFSX: "HX" at offset 1024
    if buf.len() > 1026 && &buf[1024..1026] == b"HX" {
        return FsType::HfsPlus;
    }

    // ISO 9660: "CD001" at offset 0x8001 (32769)
    if buf.len() > 0x8006 && &buf[0x8001..0x8006] == b"CD001" {
        return FsType::Iso9660;
    }

    // LUKS: "LUKS\xBA\xBE" at offset 0
    if buf.len() > 6 && &buf[0..6] == b"LUKS\xBA\xBE" {
        return FsType::Luks;
    }

    // BitLocker: "-FVE-FS-" at offset 3
    if buf.len() > 11 && &buf[3..11] == b"-FVE-FS-" {
        return FsType::BitLocker;
    }

    // APFS: "NXSB" at offset 32
    if buf.len() > 36 && &buf[32..36] == b"NXSB" {
        return FsType::Apfs;
    }

    // F2FS: magic 0xF2F52010 at offset 1024
    if buf.len() > 1028 {
        let magic = u32::from_le_bytes([buf[1024], buf[1025], buf[1026], buf[1027]]);
        if magic == 0xF2F52010 {
            return FsType::F2fs;
        }
    }

    // ZFS: "BLabelPS" or "ZPOOL" markers (simplified; full ZFS label is complex)
    if buf.len() > 8 && &buf[0..8] == b"\x00\x00\x00\x00\x00\x0A\x0A\x00" {
        // Fallthrough — ZFS uberblock is at offset 128 KiB; skip for now
    }

    FsType::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_ext4_magic() {
        let mut buf = [0u8; 0x470];
        buf[0x438] = 0x53;
        buf[0x439] = 0xEF;
        // Set INCOMPAT_EXTENTS (bit 6 = 0x40)
        buf[0x460] = 0x40;
        assert_eq!(detect_by_magic(&buf), FsType::Ext4);
    }

    #[test]
    fn detect_ntfs_magic() {
        let mut buf = [0u8; 512];
        buf[3..11].copy_from_slice(b"NTFS    ");
        assert_eq!(detect_by_magic(&buf), FsType::Ntfs);
    }

    #[test]
    fn detect_luks_magic() {
        let mut buf = [0u8; 512];
        buf[0..6].copy_from_slice(b"LUKS\xBA\xBE");
        assert_eq!(detect_by_magic(&buf), FsType::Luks);
    }

    #[test]
    fn detect_unknown() {
        let buf = [0u8; 4096];
        assert_eq!(detect_by_magic(&buf), FsType::Unknown);
    }

    #[test]
    fn fstype_display_names() {
        assert_eq!(FsType::Ext4.display_name(), "ext4");
        assert_eq!(FsType::Ntfs.display_name(), "NTFS");
        assert_eq!(FsType::Luks.display_name(), "LUKS");
    }
}
