//! ironkey-mount — Unified filesystem mount abstraction.
//!
//! Wraps kernel native mounts (ext4, btrfs, xfs, vfat, exfat, ntfs3, hfsplus)
//! and FUSE-based userspace drivers (APFS, LUKS, BitLocker) behind a single
//! `mount()` / `unmount()` API.

pub mod apfs;
pub mod bitlocker;
pub mod detect;
pub mod generic;
pub mod luks;
pub mod ntfs;

pub use detect::{detect_filesystem, FsType};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ──────────────────────────────────────────────────────────────────────────────
// Public types
// ──────────────────────────────────────────────────────────────────────────────

/// Mount options common to all filesystems.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MountOptions {
    /// Mount read-only
    pub read_only: bool,
    /// Mount point override; if `None`, uses `/mnt/ironkey/<label|devname>`
    pub mount_point: Option<PathBuf>,
    /// Passphrase for encrypted volumes (LUKS / BitLocker)
    pub passphrase: Option<String>,
    /// Recovery key path for BitLocker
    pub recovery_key_path: Option<PathBuf>,
}

/// A handle to an active mount. Dropping this does NOT unmount automatically
/// (unmounting requires root and must be explicit).
#[derive(Debug, Clone)]
pub struct MountHandle {
    /// The device that was mounted
    pub device: PathBuf,
    /// The effective mount point
    pub mount_point: PathBuf,
    /// The filesystem type used for the mount
    pub fs_type: FsType,
}

impl MountHandle {
    /// Unmount this filesystem.
    pub fn unmount(&self) -> Result<()> {
        unmount(&self.mount_point)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Core API
// ──────────────────────────────────────────────────────────────────────────────

/// Mount `device` to the given (or auto-generated) mount point.
///
/// Detects the filesystem type automatically if `fs_type` is `FsType::Auto`.
pub fn mount(device: &Path, fs_type: FsType, opts: MountOptions) -> Result<MountHandle> {
    let effective_fs_type = if fs_type == FsType::Auto {
        detect_filesystem(device)?
    } else {
        fs_type
    };

    let mount_point = opts.mount_point.clone().unwrap_or_else(|| {
        let name = device
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "disk".to_string());
        PathBuf::from(format!("/mnt/ironkey/{}", name))
    });

    // Ensure mount point directory exists
    std::fs::create_dir_all(&mount_point)?;

    match effective_fs_type {
        FsType::Luks => luks::mount_luks(device, &mount_point, &opts)?,
        FsType::BitLocker => bitlocker::mount_bitlocker(device, &mount_point, &opts)?,
        FsType::Apfs => apfs::mount_apfs(device, &mount_point, &opts)?,
        FsType::Ntfs => ntfs::mount_ntfs(device, &mount_point, &opts)?,
        _ => generic::mount_generic(device, &effective_fs_type, &mount_point, &opts)?,
    }

    Ok(MountHandle {
        device: device.to_path_buf(),
        mount_point,
        fs_type: effective_fs_type,
    })
}

/// Unmount the filesystem at `mount_point`.
pub fn unmount(mount_point: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use nix::mount::umount2;
        use nix::mount::MntFlags;
        umount2(mount_point, MntFlags::MNT_DETACH)
            .map_err(|e| anyhow::anyhow!("umount2 failed: {}", e))?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = mount_point;
        anyhow::bail!("unmount is only supported on Linux");
    }
    Ok(())
}
