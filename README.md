<div align="center">

```
  ⚿ IronKey
```

# IronKey

**A bootable USB forensics and drive-recovery operating system, written entirely in Rust.**

[![License: MIT](https://img.shields.io/badge/license-MIT-orange.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/0xnullsect0r/IronKey?color=f97316)](https://github.com/0xnullsect0r/IronKey/releases)
[![Docs](https://img.shields.io/badge/docs-pages-f97316)](https://0xnullsect0r.github.io/IronKey)

</div>

---

IronKey boots directly into a fullscreen, three-panel Rust application. No desktop environment. No login screen. Just a focused tool for working with drives, files, and terminals on bare metal.

It mounts LUKS, NTFS, APFS, and BitLocker volumes, reads SMART diagnostics, clones and wipes drives, browses and inspects filesystems, and drops into a fully capable zsh terminal — all from a single USB stick you can build and flash yourself in one command.

---

## Features

| | |
|---|---|
| **Drives Panel** | Enumerate all physical drives and partitions. Mount/unmount, format, clone, wipe, inspect SMART data, view partition tables. |
| **File Browser** | Navigate any mounted filesystem. Copy, cut, paste, rename, delete, search. Inline hex viewer and text viewer. |
| **Terminal** | Full PTY-backed zsh terminal with ANSI colour, scrollback, and pre-installed Rust-native CLI tools. |
| **LUKS 1/2** | Unlock and mount LUKS-encrypted volumes with passphrase. |
| **NTFS** | Read/write NTFS via `ntfs-3g`. |
| **APFS** | Read APFS volumes (macOS drives) via `apfs-fuse`. |
| **BitLocker** | Mount BitLocker volumes via `dislocker`. |
| **SMART** | Per-drive SMART attribute table, health status, reallocated sectors, temperature. |
| **Forensics tools** | `ddrescue`, `testdisk`, `photorec`, `foremost`, `sleuthkit`, `volatility3`, `binwalk`, and more — all pre-installed. |
| **Rust-native CLI** | `eza`, `bat`, `fd`, `ripgrep`, `dust`, `procs`, `bottom`, `hexyl`, `delta` — modern replacements for standard tools. |
| **Zero install** | Boots on any x86-64 machine. Nothing is installed to the host. Writes are tmpfs-backed and discarded on reboot. |

---

## Architecture

IronKey is a Cargo workspace with seven crates:

```
IronKey/
├── ironkey-app/        # Binary entry point — boots the iced application
├── ironkey-ui/         # iced 0.14 fullscreen UI (pane grid, panels, modals, theme)
│   ├── panels/
│   │   ├── drives.rs   # Drives panel
│   │   ├── browser.rs  # File browser panel
│   │   └── terminal.rs # Terminal panel
│   └── modals/         # Passphrase, format confirm, progress, file viewer, properties
├── ironkey-drives/     # sysfs drive enumeration, SMART, format, clone, wipe
├── ironkey-mount/      # LUKS, NTFS, APFS, BitLocker, generic mount/unmount
├── ironkey-browser/    # Directory listing, file ops, search, hex/text viewer
├── ironkey-terminal/   # PTY management + ANSI renderer
└── xtask/              # Build automation (cargo xtask)

os/
├── build.sh            # Debootstrap → squashfs → GRUB hybrid ISO
├── grub/grub.cfg       # GRUB config (searches by volume label IRONKEY)
├── rootfs/             # Files overlaid into the rootfs
│   └── etc/
│       ├── systemd/system/ironkey.service
│       ├── live/boot.conf
│       └── zsh/
└── tools-manifest.toml # Full list of pre-installed tools
```

The OS layer is a minimal Debian bookworm rootfs built with `debootstrap`, packed into a squashfs, and assembled into a bootable hybrid ISO (BIOS + UEFI) using `grub-mkrescue`. At runtime, `live-boot` mounts the squashfs and sets up a tmpfs overlay. `cage` (Wayland kiosk compositor) launches the IronKey binary as the sole application.

```
GRUB → linux kernel → initramfs (live-boot) → squashfs + overlayfs → systemd → cage → ironkey
```

---

## Quick Start

### One command — build and flash

```bash
git clone https://github.com/0xnullsect0r/IronKey
cd IronKey
sudo ./write-usb.sh --drive /dev/sdX
```

This compiles the Rust binary, builds the ISO inside a Docker container (handles non-Debian hosts like Arch/CachyOS automatically), and writes it directly to your USB drive.

> **⚠ All data on the target drive will be erased.**

### Requirements

| Requirement | Notes |
|---|---|
| x86-64 Linux host | Arch, Debian, Ubuntu, Fedora — all work |
| Rust (stable) | Install via [rustup](https://rustup.rs) |
| Docker | Required on non-Debian hosts (Arch, CachyOS, etc.) |
| `debootstrap`, `squashfs-tools`, `grub-pc-bin`, `grub-efi-amd64-bin`, `xorriso`, `mtools` | Required on Debian/Ubuntu hosts only (auto-used inside Docker otherwise) |
| ~4 GB free disk in `/tmp` | For the build workspace |
| 8 GB+ USB drive | Target for flashing |

### Boot requirements

- x86-64 machine
- UEFI or legacy BIOS
- **Secure Boot must be disabled** (GRUB is not signed)
- Set USB as the first boot device in your firmware

---

## `write-usb.sh`

```
Usage: sudo ./write-usb.sh --drive <device> [options]

Options:
  --drive  <dev>    Target block device, e.g. /dev/sdb  (required)
  --skip-build      Skip the build step and flash an existing ISO
  --iso    <file>   ISO to flash when --skip-build is used
  --qemu            Boot the built ISO in QEMU instead of flashing (for testing)
  --yes             Skip the confirmation prompt
  --help            Show help
```

**Examples:**

```bash
# Build from source and write to /dev/sdb
sudo ./write-usb.sh --drive /dev/sdb

# Test in QEMU before flashing (requires qemu-system-x86_64)
sudo ./write-usb.sh --qemu

# Flash an existing ISO without rebuilding
sudo ./write-usb.sh --drive /dev/sdb --skip-build --iso ironkey-v1.0.2.iso

# Non-interactive
sudo ./write-usb.sh --drive /dev/sdb --yes
```

---

## Building manually

### Build the Rust binary

```bash
cargo build -p ironkey-app --release
# Binary: target/release/ironkey
```

### Build the bootable ISO

**Debian/Ubuntu host:**
```bash
sudo apt-get install -y debootstrap squashfs-tools grub-pc-bin grub-efi-amd64-bin xorriso mtools
sudo bash os/build.sh ironkey.iso
```

**Any host (Docker required):**
```bash
# Docker is used automatically on non-Debian systems
sudo bash os/build.sh ironkey.iso
```

The build script accepts an optional `IRONKEY_PREBUILT` environment variable pointing to a pre-compiled binary, skipping the cargo step:

```bash
IRONKEY_PREBUILT=target/release/ironkey sudo bash os/build.sh ironkey.iso
```

### Test in QEMU

```bash
qemu-system-x86_64 \
  -enable-kvm \
  -m 2G \
  -cpu host \
  -cdrom ironkey.iso \
  -boot d
```

---

## Development

```bash
# Check the whole workspace
cargo check --workspace

# Run tests
cargo test --workspace

# Build in debug mode
cargo build -p ironkey-app

# Run directly on a Linux desktop (Wayland or X11 via cage)
cargo run -p ironkey-app
```

The application can be run directly on a Linux desktop during development — `iced` handles rendering via wgpu or tiny-skia. Mounting operations require root privileges and appropriate kernel modules.

---

## Pre-installed Tools

The full list is in [`os/tools-manifest.toml`](os/tools-manifest.toml) and browsable at the [docs tools page](https://0xnullsect0r.github.io/IronKey/tools/). Highlights:

| Category | Tools |
|---|---|
| **Rust-native** | `eza`, `bat`, `fd`, `rg`, `dust`, `procs`, `bottom`, `hexyl`, `delta`, `tokei`, `bandwhich` |
| **Forensics** | `ddrescue`, `testdisk`, `photorec`, `foremost`, `sleuthkit`, `autopsy`, `volatility3`, `binwalk` |
| **Partitioning** | `fdisk`, `gdisk`, `parted`, `gparted`, `sfdisk` |
| **Filesystems** | `mkfs.*`, `fsck.*`, `ntfs-3g`, `dosfstools`, `e2fsprogs`, `btrfs-progs`, `xfsprogs` |
| **Mounting** | `cryptsetup`, `dislocker`, `apfs-fuse`, `fuse` |
| **Bootloaders** | `grub`, `efibootmgr`, `ms-sys` |
| **Editors** | `neovim`, `nano`, `micro` |
| **System** | `htop`, `lsof`, `strace`, `pciutils`, `usbutils`, `smartmontools`, `hdparm` |

---

## Project Structure

```
.
├── Cargo.toml              # Workspace manifest
├── Cargo.lock
├── write-usb.sh            # Build-and-flash utility
├── os/
│   ├── build.sh            # ISO build script
│   ├── grub/grub.cfg       # GRUB boot config
│   ├── rootfs/             # Files overlaid into the rootfs
│   └── tools-manifest.toml
├── docs/                   # GitHub Pages documentation site
│   ├── index.html
│   ├── getting-started/
│   ├── install/
│   ├── user-guide/
│   ├── tools/
│   ├── architecture/
│   └── assets/
├── ironkey-app/
├── ironkey-ui/
├── ironkey-drives/
├── ironkey-mount/
├── ironkey-browser/
├── ironkey-terminal/
└── xtask/
```

---

## Documentation

Full documentation is available at **[0xnullsect0r.github.io/IronKey](https://0xnullsect0r.github.io/IronKey)**.

- [Getting Started](https://0xnullsect0r.github.io/IronKey/getting-started/)
- [Installation Guide](https://0xnullsect0r.github.io/IronKey/install/)
- [User Guide](https://0xnullsect0r.github.io/IronKey/user-guide/)
- [Pre-installed Tools](https://0xnullsect0r.github.io/IronKey/tools/)
- [Architecture](https://0xnullsect0r.github.io/IronKey/architecture/)

---

## Releases

Pre-built ISOs are published on the [Releases page](https://github.com/0xnullsect0r/IronKey/releases) for every version tag. Each release includes:

- `ironkey-vX.Y.Z.iso` — bootable hybrid ISO (BIOS + UEFI)
- `ironkey-vX.Y.Z.iso.sha256` — SHA-256 checksum

Verify before flashing:
```bash
sha256sum -c ironkey-vX.Y.Z.iso.sha256
```

---

## Contributing

1. Fork the repo
2. Create a branch: `git checkout -b feat/my-feature`
3. Make changes, run `cargo check --workspace` and `cargo test --workspace`
4. Open a pull request

---

## License

MIT — see [LICENSE](LICENSE).
