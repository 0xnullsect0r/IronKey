//! APFS mounting via `apfs-fuse` (read-only, FUSE-based).

use crate::MountOptions;
use anyhow::{Context, Result};
use std::path::Path;

/// Mount an APFS container using `apfs-fuse`.
///
/// APFS is read-only on non-Apple systems. The `apfs-fuse` binary must
/// be installed on the IronKey USB.
pub fn mount_apfs(device: &Path, mount_point: &Path, _opts: &MountOptions) -> Result<()> {
    let status = std::process::Command::new("apfs-fuse")
        .arg("-o")
        .arg("allow_other")
        .arg(device)
        .arg(mount_point)
        .status()
        .with_context(|| "Failed to run apfs-fuse")?;

    if !status.success() {
        anyhow::bail!(
            "apfs-fuse failed for {} (exit: {})",
            device.display(),
            status
        );
    }
    Ok(())
}
