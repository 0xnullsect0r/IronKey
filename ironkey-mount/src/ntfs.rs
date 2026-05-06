//! NTFS mounting via the `ntfs3` kernel module (preferred) or `ntfs-3g` FUSE fallback.

use crate::MountOptions;
use anyhow::{Context, Result};
use std::path::Path;

/// Mount an NTFS partition.
///
/// Tries the `ntfs3` kernel driver first (read-write, no FUSE overhead).
/// Falls back to the `ntfs-3g` FUSE binary if ntfs3 is unavailable.
pub fn mount_ntfs(device: &Path, mount_point: &Path, opts: &MountOptions) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use nix::mount::{mount, MsFlags};

        let mut flags = MsFlags::empty();
        if opts.read_only {
            flags |= MsFlags::MS_RDONLY;
        }

        // Try ntfs3 (upstream kernel driver, Linux 5.15+)
        let result = mount(
            Some(device),
            mount_point,
            Some("ntfs3"),
            flags,
            Some("nls=utf8"),
        );

        if result.is_ok() {
            log::info!("Mounted {} via ntfs3", device.display());
            return Ok(());
        }

        log::warn!(
            "ntfs3 failed for {}: {:?} — falling back to ntfs-3g",
            device.display(),
            result
        );
    }

    // FUSE fallback: ntfs-3g
    mount_ntfs_fuse(device, mount_point, opts)
}

fn mount_ntfs_fuse(device: &Path, mount_point: &Path, opts: &MountOptions) -> Result<()> {
    let mut cmd = std::process::Command::new("ntfs-3g");
    cmd.arg(device).arg(mount_point);
    if opts.read_only {
        cmd.arg("-o").arg("ro");
    }
    let status = cmd
        .status()
        .with_context(|| "Failed to run ntfs-3g")?;
    if !status.success() {
        anyhow::bail!(
            "ntfs-3g failed for {} (exit: {})",
            device.display(),
            status
        );
    }
    Ok(())
}
