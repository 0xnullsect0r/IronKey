//! LUKS encrypted volume mounting via `cryptsetup`.

use crate::MountOptions;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Open and mount a LUKS encrypted partition.
///
/// The passphrase is read from `opts.passphrase`. The unlocked device is
/// mapped as `/dev/mapper/ironkey_<device_name>` and then mounted.
pub fn mount_luks(device: &Path, mount_point: &Path, opts: &MountOptions) -> Result<()> {
    let passphrase = opts
        .passphrase
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("LUKS passphrase required"))?;

    let device_name = device
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "ironkey_luks".to_string());

    let mapper_name = format!("ironkey_{}", device_name);
    let mapper_path = format!("/dev/mapper/{}", mapper_name);

    // Step 1: open the LUKS container
    let mut child = Command::new("cryptsetup")
        .args(["open", "--type", "luks"])
        .arg(device)
        .arg(&mapper_name)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| "Failed to spawn cryptsetup")?;

    // Write passphrase to stdin
    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        stdin
            .write_all(passphrase.as_bytes())
            .with_context(|| "Writing passphrase to cryptsetup stdin")?;
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("cryptsetup open failed (exit: {})", status);
    }

    // Step 2: detect filesystem and mount via generic handler
    let mapper = std::path::PathBuf::from(&mapper_path);
    let fs_type = crate::detect::detect_filesystem(&mapper)?;
    crate::generic::mount_generic(&mapper, &fs_type, mount_point, opts)
        .with_context(|| format!("Mounting unlocked LUKS device {}", mapper_path))
}

/// Close a LUKS device mapped as `mapper_name`.
pub fn close_luks(mapper_name: &str) -> Result<()> {
    let status = Command::new("cryptsetup")
        .args(["close", mapper_name])
        .status()
        .with_context(|| "Failed to run cryptsetup close")?;
    if !status.success() {
        anyhow::bail!("cryptsetup close {} failed (exit: {})", mapper_name, status);
    }
    Ok(())
}
