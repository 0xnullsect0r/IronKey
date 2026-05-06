# IronKey Pre-installed Tools

This document describes every tool bundled on the IronKey USB image.
All tools are accessible from the **Terminal panel** inside the IronKey application.

Rust-native tools are statically linked wherever possible to avoid dependency
issues on the minimal rootfs. C tools use the minimal shared libraries available
in the Debian minbase bootstrap.

---

## Shells

| Tool   | Purpose                          |
|--------|----------------------------------|
| `zsh`  | Default interactive shell        |
| `bash` | POSIX fallback shell             |

---

## Rust-Native CLI Replacements (pre-aliased in .zshrc)

| Tool      | Replaces   | Purpose                                            |
|-----------|------------|----------------------------------------------------|
| `eza`     | `ls`       | File listing with icons and colors                 |
| `bat`     | `cat`      | Syntax-highlighted file viewer                     |
| `fd`      | `find`     | Fast file search                                   |
| `rg`      | `grep`     | Fast content search (ripgrep)                      |
| `dust`    | `du`       | Disk usage visualizer                              |
| `hexyl`   | `hexdump`  | Hex viewer                                         |
| `sd`      | `sed`      | Stream editor                                      |
| `procs`   | `ps`       | Process viewer                                     |
| `btm`     | `top`      | System resource monitor (bottom)                   |
| `xcp`     | `cp`       | Fast copy with progress                            |
| `ouch`    | `tar/zip`  | Compress/decompress any archive format             |
| `starship`| —          | Cross-shell prompt used by the IronKey terminal    |

---

## Partition Table Editors

| Tool      | Purpose                                                          |
|-----------|------------------------------------------------------------------|
| `fdisk`   | Classic MBR/GPT partition editor (interactive + scriptable)     |
| `gdisk`   | GPT-only partition editor, more robust than fdisk for GPT        |
| `cgdisk`  | Curses-based GPT partition editor (ncurses UI)                   |
| `sgdisk`  | Scripted/non-interactive gdisk — ideal for automation            |
| `parted`  | GNU Parted — supports MBR + GPT, scriptable, widely supported    |
| `sfdisk`  | Script-friendly fdisk — JSON output, ideal for imaging/cloning   |
| `cfdisk`  | Curses-based fdisk with a simple visual interface                |
| `gparted` | GTK partition editor — launch with `cage -d -- gparted &`        |

> **Note on GParted**: Since IronKey runs under `cage` (Wayland kiosk compositor),
> GParted is launched as a child window. The alias `gparted-launch` does this for you.

---

## Disk & Filesystem Management

| Tool              | Purpose                                                      |
|-------------------|--------------------------------------------------------------|
| `mkfs.ext4/3/2`   | Create ext2/3/4 filesystems (`e2fsprogs`)                   |
| `mkfs.fat`        | Create FAT12/16/32 filesystems (`dosfstools`)               |
| `mkfs.exfat`      | Create exFAT filesystems (`exfatprogs`)                     |
| `mkfs.ntfs`       | Create NTFS filesystems (`ntfs-3g` / `ntfsprogs`)           |
| `mkfs.btrfs`      | Create btrfs filesystems (`btrfs-progs`)                    |
| `mkfs.xfs`        | Create XFS filesystems (`xfsprogs`)                         |
| `mkfs.f2fs`       | Create F2FS filesystems (flash-friendly, Chromebook common) |
| `mkswap`          | Create swap partition/file                                   |
| `mke2fs`          | Low-level ext filesystem creation                           |
| `tune2fs`         | Adjust ext2/3/4 filesystem parameters                       |
| `resize2fs`       | Resize ext2/3/4 filesystems online or offline               |
| `e2fsck`          | ext2/3/4 filesystem check and repair                        |
| `fsck`            | Generic filesystem checker                                   |
| `fsck.fat`        | FAT filesystem check and repair                             |
| `fsck.exfat`      | exFAT filesystem check and repair                           |
| `ntfsfix`         | NTFS quick fix (clear dirty bit, fix boot sector)           |
| `ntfsresize`      | Resize NTFS filesystem                                      |
| `btrfs`           | btrfs management CLI (check, scrub, balance, etc.)          |
| `xfs_repair`      | XFS filesystem repair                                       |
| `xfs_info`        | XFS filesystem information                                  |
| `xfs_growfs`      | Expand XFS filesystem                                       |

---

## Disk Imaging & Cloning

| Tool          | Purpose                                                          |
|---------------|------------------------------------------------------------------|
| `dd`          | Raw disk/partition clone and wipe (standard, always present)    |
| `ddrescue`    | GNU ddrescue — error-tolerant, recovers failing drives          |
| `dd_rescue`   | Older dd_rescue — complementary to ddrescue                     |
| `dcfldd`      | dd with hashing, progress, split output — forensics-grade       |
| `partclone.*` | Filesystem-aware partition cloning (faster than dd)             |
| `rsync`       | File-level sync and copy with delta compression                 |
| `pv`          | Pipe viewer — adds progress bar and speed to any pipe           |

### Quick clone examples

```bash
# Clone disk to disk with error recovery
ddrescue -d -r3 /dev/sda /dev/sdb /tmp/rescue.log

# Clone disk to image file with progress
pv /dev/sda > /mnt/backup/disk.img

# Filesystem-aware clone (only copies used blocks)
partclone.ext4 -c -s /dev/sda1 -o /mnt/backup/sda1.img
```

---

## Disk Information & Diagnostics

| Tool        | Purpose                                                          |
|-------------|------------------------------------------------------------------|
| `lsblk`     | List block devices in a tree with sizes, types, mount points    |
| `blkid`     | Show filesystem type, UUID, and label for all partitions        |
| `lsscsi`    | List SCSI/SATA/USB storage devices                              |
| `lshw`      | Full hardware inventory (storage section very detailed)         |
| `hwinfo`    | Detailed hardware probe including storage controllers           |
| `smartctl`  | SMART disk health, temperature, error log, self-test            |
| `nvme`      | NVMe-specific CLI: identify, smart-log, error-log, sanitize     |
| `hdparm`    | ATA disk parameters, read speed test, power management          |
| `sdparm`    | SCSI/SAS disk parameters                                        |
| `iostat`    | I/O statistics per device                                       |
| `iotop`     | Real-time per-process I/O monitor                               |
| `badblocks` | Scan a disk for bad sectors                                     |
| `wipefs`    | Erase filesystem and partition table signatures                 |
| `disktype`  | Detect filesystem and partition type by magic bytes             |
| `file`      | Identify any file or device by magic bytes                      |

### Quick diagnostic examples

```bash
# SMART health check
ik-smart /dev/sda
smartctl -a /dev/sda

# Disk overview
ik-disks
lsblk

# NVMe health
nvme smart-log /dev/nvme0n1

# Read speed test
hdparm -tT /dev/sda
```

---

## Encryption & Secure Access

| Tool                     | Purpose                                                 |
|--------------------------|---------------------------------------------------------|
| `cryptsetup`             | LUKS encryption: open, close, format, resize, reencrypt|
| `dislocker`              | Mount BitLocker-encrypted volumes                       |
| `veracrypt`              | VeraCrypt / TrueCrypt volume access (CLI)               |
| `hashdeep`               | Recursive hashing and hash auditing                     |
| `md5sum` / `sha256sum`   | File checksums                                          |
| `gpg`                    | GnuPG — decrypt/verify signed files                    |
| `age`                    | Modern Rust-friendly file encryption                   |

### Quick encryption examples

```bash
# Open LUKS partition
ik-mount /dev/sda1           # auto-detects LUKS, prompts for passphrase
cryptsetup open /dev/sda1 myvolume
mount /dev/mapper/myvolume /mnt/data

# Open BitLocker partition
dislocker-fuse -u "passphrase" /dev/sda2 -- /mnt/bl_staging
mount -o loop /mnt/bl_staging/dislocker-file /mnt/data

# Hash a drive image
sha256sum /dev/sda > /mnt/usb/sda.sha256
```

---

## Data Recovery & Forensics

| Tool           | Purpose                                                         |
|----------------|-----------------------------------------------------------------|
| `testdisk`     | Recover lost partitions, fix partition tables                   |
| `photorec`     | File carving — recover deleted files by type from raw disk      |
| `extundelete`  | Recover deleted files from ext3/ext4 partitions                 |
| `ext4magic`    | Recover from ext3/ext4 journals                                 |
| `ntfsundelete` | Recover recently deleted files from NTFS                        |
| `foremost`     | File carving by header/footer signatures                        |
| `scalpel`      | Fast file carver, configurable signatures                       |
| `safecopy`     | Recover data from damaged media                                 |
| `dc3dd`        | Forensic dd with hashing on the fly                             |
| `mmls`         | Sleuth Kit: display partition layout                            |
| `fsstat`       | Sleuth Kit: filesystem statistics                               |
| `fls`          | Sleuth Kit: list files and directories                          |
| `icat`         | Sleuth Kit: output file content by inode                        |
| `binwalk`      | Firmware/binary analysis, embedded filesystem extraction        |
| `strings`      | Extract printable strings from binary or disk image             |
| `hexedit`      | Interactive terminal hex editor for raw disk editing            |

### Quick recovery examples

```bash
# Recover deleted partitions
testdisk /dev/sda

# Carve files from raw disk
photorec /dev/sda1

# Recover deleted ext4 files
extundelete /dev/sda1 --restore-all

# Forensic copy with hash verification
dc3dd if=/dev/sda hash=sha256 of=/mnt/usb/evidence.img hlog=/mnt/usb/evidence.sha256
```

---

## Mounting & Filesystem Access

| Tool        | Purpose                                                         |
|-------------|-----------------------------------------------------------------|
| `mount`     | Standard Linux mount                                            |
| `umount`    | Standard Linux unmount                                          |
| `apfs-fuse` | Mount APFS volumes from macOS drives (read-only)               |
| `ntfs-3g`   | Userspace NTFS driver (fallback if ntfs3 kernel unavailable)    |
| `zpool/zfs` | OpenZFS CLI                                                     |
| `bindfs`    | FUSE bind-mount with permission remapping                       |
| `fuse`      | FUSE userspace filesystem framework                             |

---

## Bootloader & MBR Tools

| Tool            | Purpose                                                  |
|-----------------|----------------------------------------------------------|
| `grub-install`  | Install GRUB2 bootloader to a disk                       |
| `grub-mkconfig` | Generate GRUB config (`grub.cfg`)                        |
| `efibootmgr`    | Manage UEFI boot entries — add, remove, reorder          |
| `syslinux`      | Syslinux/Extlinux bootloader tools                       |

---

## System & Process Utilities

| Tool       | Purpose                                                          |
|------------|------------------------------------------------------------------|
| `htop`     | Interactive process viewer (ncurses)                             |
| `lsof`     | List open files and the processes using them                     |
| `fuser`    | Identify processes using a file or mount point                   |
| `strace`   | Trace system calls — debug mount failures                        |
| `dmesg`    | Kernel ring buffer — hardware events, driver errors, USB hotplug |
| `udevadm`  | Query and monitor udev device events                             |
| `lspci`    | List PCI devices (storage controllers, GPU)                      |
| `lsusb`    | List USB devices                                                 |

---

## Text Processing & Scripting

| Tool   | Purpose                              |
|--------|--------------------------------------|
| `awk`  | Pattern scanning and processing      |
| `sd`   | Rust-based sed replacement           |
| `jq`   | JSON processor (`lsblk -J \| jq .`) |
| `rg`   | ripgrep — fast grep                  |
| `less` | Pager                                |

---

## Text Editors

| Tool    | Purpose                                      |
|---------|----------------------------------------------|
| `nano`  | Beginner-friendly terminal editor            |
| `vim`   | Advanced terminal editor                     |
| `micro` | Modern terminal editor with mouse support    |

---

## Miscellaneous Utilities

| Tool        | Purpose                                |
|-------------|----------------------------------------|
| `tar`       | Archive/extract tarballs               |
| `ouch`      | Universal compress/decompress          |
| `file`      | Identify file type by magic bytes      |
| `stat`      | Detailed file/filesystem metadata      |
| `tree`      | Directory tree visualizer              |
| `blockdev`  | Block device control                   |
| `wipefs`    | Erase filesystem signatures            |
| `sync`      | Flush filesystem write buffers         |

---

## IronKey Shell Helpers

The following `ik-*` functions are defined in `/etc/zsh/zshrc`:

| Function       | Purpose                                           |
|----------------|---------------------------------------------------|
| `ik-disks`     | Quick overview of all block devices               |
| `ik-mount`     | Mount a device (auto-detects LUKS/BitLocker/APFS) |
| `ik-umount`    | Unmount by device or mountpoint                   |
| `ik-clone`     | Clone a disk or partition with progress           |
| `ik-smart`     | SMART health check for a drive                    |
| `ik-hash`      | SHA-256 and MD5 of a file or device               |
| `ik-wipe`      | Zero-fill wipe with confirmation                  |
| `ik-info`      | Forensic info about a device                      |
