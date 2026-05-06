//! BitLocker encrypted volume mounting via `dislocker`.

use crate::MountOptions;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Mount a BitLocker encrypted partition using `dislocker-fuse`.
///
/// Supports:
/// - Passphrase (`opts.passphrase`)
/// - Recovery key file (`opts.recovery_key_path`)
pub fn mount_bitlocker(device: &Path, mount_point: &Path, opts: &MountOptions) -> Result<()> {
    // dislocker mounts to a FUSE staging directory, then we bind-mount the
    // contained virtual disk image.
    let staging_dir = mount_point.with_extension("_dislocker");
    std::fs::create_dir_all(&staging_dir)?;

    let mut cmd = Command::new("dislocker-fuse");
    cmd.arg(device);

    if let Some(pass) = &opts.passphrase {
        cmd.arg("-u").arg(pass);
    } else if let Some(key_path) = &opts.recovery_key_path {
        cmd.arg("-r").arg(key_path);
    } else {
        anyhow::bail!("BitLocker requires a passphrase or recovery key");
    }

    cmd.arg("--").arg(&staging_dir);

    let status = cmd
        .status()
        .with_context(|| "Failed to run dislocker-fuse")?;

    if !status.success() {
        anyhow::bail!(
            "dislocker-fuse failed for {} (exit: {})",
            device.display(),
            status
        );
    }

    // The dislocker-fuse staging dir contains "dislocker-file" — mount that.
    let virt_disk = staging_dir.join("dislocker-file");

    #[cfg(target_os = "linux")]
    {
        use nix::mount::{mount, MsFlags};
        let mut flags = if opts.read_only {
            MsFlags::MS_RDONLY
        } else {
            MsFlags::empty()
        };
        // Loop device must be set up first; use the 'mount' command for simplicity
        // as nix doesn't have MS_LOOP.
        let status = std::process::Command::new("mount")
            .args(["-o", if opts.read_only { "loop,ro" } else { "loop" }])
            .arg(&virt_disk)
            .arg(mount_point)
            .status()
            .with_context(|| {
                format!(
                    "Mounting BitLocker virtual disk {} at {}",
                    virt_disk.display(),
                    mount_point.display()
                )
            })?;
        if !status.success() {
            anyhow::bail!(
                "mount loop failed for {} (exit: {})",
                virt_disk.display(),
                status
            );
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = virt_disk;
        anyhow::bail!("BitLocker mounting is only supported on Linux");
    }

    Ok(())
}
