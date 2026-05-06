//! Generic kernel-native filesystem mounts (ext2/3/4, vfat, exfat, btrfs, xfs, …).

use crate::{FsType, MountOptions};
use anyhow::{Context, Result};
use std::path::Path;

/// Mount a device with a kernel-native filesystem driver.
pub fn mount_generic(
    device: &Path,
    fs_type: &FsType,
    mount_point: &Path,
    opts: &MountOptions,
) -> Result<()> {
    let fs_name = fs_type
        .kernel_name()
        .ok_or_else(|| anyhow::anyhow!("No kernel driver for {:?}", fs_type))?;

    #[cfg(target_os = "linux")]
    {
        use nix::mount::{mount, MsFlags};

        let mut flags = MsFlags::empty();
        if opts.read_only {
            flags |= MsFlags::MS_RDONLY;
        }

        mount(
            Some(device),
            mount_point,
            Some(fs_name),
            flags,
            None::<&str>,
        )
        .with_context(|| {
            format!(
                "mount({}, {}, {})",
                device.display(),
                mount_point.display(),
                fs_name
            )
        })?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (device, mount_point, fs_name, opts);
        anyhow::bail!("Kernel mounts are only supported on Linux");
    }

    Ok(())
}
