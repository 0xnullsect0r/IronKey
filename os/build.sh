#!/usr/bin/env bash
# IronKey OS Build Script
# Builds a minimal bootable USB image containing the IronKey application.
#
# Requirements (Debian/Ubuntu host):
#   debootstrap squashfs-tools grub-pc-bin grub-efi-amd64-bin xorriso mtools
#   Root privileges · Rust toolchain · 4 GB free in /tmp
#
# On non-Debian hosts (Arch, CachyOS, Fedora, etc.) the script automatically
# re-executes itself inside a debian:bookworm Docker container.
# Docker must be installed: https://docs.docker.com/engine/install/
#
# Usage:
#   sudo bash os/build.sh [output.iso]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT="${1:-ironkey.iso}"
# Resolve OUTPUT to an absolute path so it's reachable from inside Docker too
[[ "$OUTPUT" = /* ]] || OUTPUT="$(pwd)/$OUTPUT"
BUILD_DIR="/tmp/ironkey-build-$$"
ROOTFS_DIR="${BUILD_DIR}/rootfs"
ISO_DIR="${BUILD_DIR}/iso"

# ── Colour output ────────────────────────────────────────────────────────────
CYAN='\033[0;36m'; GREEN='\033[0;32m'; RED='\033[0;31m'; RESET='\033[0m'
info()  { echo -e "${CYAN}▶ $*${RESET}"; }
ok()    { echo -e "${GREEN}✔ $*${RESET}"; }
err()   { echo -e "${RED}✗ $*${RESET}" >&2; exit 1; }

# ── Docker fallback for non-Debian/Ubuntu hosts (e.g. Arch, CachyOS) ─────────
# debootstrap requires Debian-style architecture scripts and keyrings.
# If we're not on a Debian-based host, re-exec inside debian:bookworm via Docker.
_is_debian_based() {
    grep -qiE 'debian|ubuntu' /etc/os-release 2>/dev/null
}

if ! _is_debian_based && [[ -z "${IRONKEY_DOCKER_INNER:-}" ]]; then
    command -v docker >/dev/null || err \
        "Not a Debian/Ubuntu host and 'docker' was not found.
  Install Docker: https://docs.docker.com/engine/install/
  Or run on a Debian/Ubuntu machine."

    info "Non-Debian host — running build inside debian:bookworm container…"

    # Build dependencies to install inside the container
    BUILD_DEPS="debootstrap squashfs-tools grub-pc-bin grub-efi-amd64-bin xorriso mtools"

    docker run --rm --privileged \
        -v "${REPO_ROOT}:${REPO_ROOT}" \
        -e "IRONKEY_DOCKER_INNER=1" \
        ${IRONKEY_PREBUILT:+-e "IRONKEY_PREBUILT=${IRONKEY_PREBUILT}"} \
        debian:bookworm \
        bash -c "apt-get update -qq && \
                 apt-get install -y --no-install-recommends ${BUILD_DEPS} >/dev/null 2>&1 && \
                 bash '${SCRIPT_DIR}/build.sh' '${OUTPUT}'"
    exit $?
fi

# ── Sanity checks ────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]] || err "Must be run as root"
command -v debootstrap  >/dev/null || err "debootstrap not found (apt install debootstrap)"
command -v mksquashfs   >/dev/null || err "mksquashfs not found (apt install squashfs-tools)"
command -v grub-mkrescue>/dev/null || err "grub-mkrescue not found (apt install grub-pc-bin grub-efi-amd64-bin)"
if [[ -z "${IRONKEY_PREBUILT:-}" ]]; then
    command -v cargo >/dev/null || err "cargo not found (install Rust via rustup)"
fi

mkdir -p "$BUILD_DIR" "$ROOTFS_DIR" "$ISO_DIR"

# ── Step 1: Build IronKey binary ─────────────────────────────────────────────
info "Building IronKey application binary…"
cd "$REPO_ROOT"
if [[ -n "${IRONKEY_PREBUILT:-}" ]]; then
    # CI: binary was compiled as the non-root runner user and passed in via env.
    IRONKEY_BIN="$IRONKEY_PREBUILT"
    ok "Using pre-built binary: $IRONKEY_BIN"
else
    cargo build -p ironkey-app --release
    IRONKEY_BIN="$REPO_ROOT/target/release/ironkey"
fi
[[ -f "$IRONKEY_BIN" ]] || err "Build failed: binary not found at $IRONKEY_BIN"
ok "Binary: $IRONKEY_BIN ($(du -sh "$IRONKEY_BIN" | cut -f1))"

# ── Step 2: Bootstrap minimal Debian rootfs ──────────────────────────────────
info "Bootstrapping minimal Debian rootfs (this may take a few minutes)…"
debootstrap \
    --arch=amd64 \
    --variant=minbase \
    --include=systemd,dbus,dbus-user-session,cage,zsh,bash,udev,kmod,linux-image-amd64,initramfs-tools,live-boot,live-boot-initramfs-tools,live-config,firmware-realtek \
    bookworm \
    "$ROOTFS_DIR" \
    http://deb.debian.org/debian

ok "Debootstrap complete"

# ── Step 3: Configure rootfs ─────────────────────────────────────────────────
info "Configuring rootfs…"

# Copy IronKey binary
install -Dm755 "$IRONKEY_BIN" "$ROOTFS_DIR/usr/bin/ironkey"

# Copy config files
cp -r "$SCRIPT_DIR/rootfs/etc/." "$ROOTFS_DIR/etc/"
chmod 644 "$ROOTFS_DIR/etc/zsh/zshrc"
chmod 644 "$ROOTFS_DIR/etc/zsh/ironkey-aliases.zsh"
chmod 644 "$ROOTFS_DIR/etc/starship.toml"
chmod 644 "$ROOTFS_DIR/etc/live/boot.conf"

# Enable IronKey service under graphical.target
mkdir -p "$ROOTFS_DIR/etc/systemd/system/graphical.target.wants"
ln -sf /etc/systemd/system/ironkey.service \
    "$ROOTFS_DIR/etc/systemd/system/graphical.target.wants/ironkey.service"

# Set default systemd target to graphical (so cage/IronKey starts)
ln -sf /lib/systemd/system/graphical.target \
    "$ROOTFS_DIR/etc/systemd/system/default.target"

# Set hostname
echo "ironkey" > "$ROOTFS_DIR/etc/hostname"

# Minimal fstab — live-boot handles the root overlayfs.
# Explicit tmpfs mounts on /tmp /var /run are intentionally omitted;
# they race with live-boot's pivot-root and confuse early systemd startup.
cat > "$ROOTFS_DIR/etc/fstab" <<'EOF'
proc    /proc   proc    defaults    0 0
EOF

ok "Rootfs configured"

# ── Step 3b: Prepare initramfs config for live-boot ──────────────────────────
# These files must be in place BEFORE update-initramfs runs so the hooks pick
# them up and bake them into the initrd image.

# Explicitly include the modules live-boot needs inside the initramfs.
# overlay:   overlayfs union mount for the writable live layer
# squashfs:  read the filesystem.squashfs image
# loop:      mount ISO images as block devices
mkdir -p "$ROOTFS_DIR/etc/initramfs-tools"
cat > "$ROOTFS_DIR/etc/initramfs-tools/modules" <<'EOF'
overlay
squashfs
loop
EOF

# Use gzip for initramfs compression — universally supported by all kernels.
# lz4/zstd may not be compiled in on older or minimal kernel configs.
sed -i 's/^COMPRESS=.*/COMPRESS=gzip/' \
    "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf" 2>/dev/null || \
  echo "COMPRESS=gzip" >> "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf"

# MODULES=most ensures all likely-needed drivers (disk, USB, HID) are included.
sed -i 's/^MODULES=.*/MODULES=most/' \
    "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf" 2>/dev/null || \
  echo "MODULES=most" >> "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf"

# Empty machine-id: live systems should not ship a fixed ID.
# systemd will generate a transient one on first boot automatically.
> "$ROOTFS_DIR/etc/machine-id"

# /run/user/0 is needed by cage (Wayland) at runtime; create it now so the
# rootfs already has the directory, even though it will be remounted tmpfs.
mkdir -p "$ROOTFS_DIR/run/user/0"
chmod 700 "$ROOTFS_DIR/run/user/0"

# Temporary resolv.conf so any chroot network calls (unlikely but safe) work
echo "nameserver 1.1.1.1" > "$ROOTFS_DIR/etc/resolv.conf"

# ── Step 3c: Regenerate initramfs inside chroot ───────────────────────────────
# CRITICAL: debootstrap runs update-initramfs without /proc and /sys mounted.
# This produces a degraded initramfs that does NOT include live-boot hooks,
# which means live-boot's pivot-root never runs → PID 1 dies → kernel panic.
#
# We must regenerate the initramfs inside a proper chroot with all mounts.
info "Regenerating initramfs with live-boot hooks (chroot)…"

# Set up a cleanup trap so mounts are always removed even if we error out
_chroot_cleanup() {
    for _mnt in dev/pts dev/shm dev sys proc; do
        mountpoint -q "$ROOTFS_DIR/$_mnt" 2>/dev/null && \
            umount -lf "$ROOTFS_DIR/$_mnt" 2>/dev/null || true
    done
}
trap _chroot_cleanup EXIT

mount -t proc  proc     "$ROOTFS_DIR/proc"
mount -t sysfs sysfs    "$ROOTFS_DIR/sys"
mount --bind   /dev     "$ROOTFS_DIR/dev"
mount --bind   /dev/pts "$ROOTFS_DIR/dev/pts"
mount -t tmpfs tmpfs    "$ROOTFS_DIR/dev/shm"

# Ensure /sbin/init → systemd symlink exists (kernel needs a valid PID 1 path)
chroot "$ROOTFS_DIR" ln -sfn /lib/systemd/systemd /sbin/init 2>/dev/null || true
chroot "$ROOTFS_DIR" ln -sfn /lib/systemd/systemd /usr/sbin/init 2>/dev/null || true

# Delete any initramfs generated by debootstrap's package scripts — it was built
# without proper /proc & /sys so live-boot hooks may be absent or broken.
rm -f "$ROOTFS_DIR"/boot/initrd.img-*

# Regenerate from scratch. DEBIAN_FRONTEND=noninteractive prevents any prompts.
# update-initramfs now runs with /proc and /sys available so all hooks execute.
chroot "$ROOTFS_DIR" \
    env DEBIAN_FRONTEND=noninteractive \
    update-initramfs -c -k all

# Verify the initrd was actually created
INITRD_CHECK=$(ls "$ROOTFS_DIR"/boot/initrd.img-* 2>/dev/null | head -1)
if [[ -z "$INITRD_CHECK" ]]; then
    _chroot_cleanup
    err "update-initramfs did not produce an initrd — check the output above"
fi
ok "Initramfs created: $(basename "$INITRD_CHECK") ($(du -sh "$INITRD_CHECK" | cut -f1))"

_chroot_cleanup
trap - EXIT

ok "Initramfs regenerated with live-boot hooks"

# ── Step 4: Pack rootfs into squashfs ────────────────────────────────────────
info "Packing rootfs into squashfs…"
# live-boot expects the squashfs at /live/filesystem.squashfs on the ISO.
# ISO_DIR maps to the root of the ISO, so this path is correct.
SQUASHFS="${ISO_DIR}/live/filesystem.squashfs"
mkdir -p "${ISO_DIR}/live"
mksquashfs "$ROOTFS_DIR" "$SQUASHFS" \
    -comp zstd -Xcompression-level 19 \
    -noappend \
    -e "$ROOTFS_DIR/proc" \
    -e "$ROOTFS_DIR/sys" \
    -e "$ROOTFS_DIR/dev" \
    -e "$ROOTFS_DIR/run"
ok "squashfs: $(du -sh "$SQUASHFS" | cut -f1)"

# filesystem.module tells live-boot the overlay type so it doesn't have to guess
echo "squashfs" > "${ISO_DIR}/live/filesystem.module"

# ── Step 5: Copy kernel and initramfs ────────────────────────────────────────
info "Copying kernel and initramfs…"
mkdir -p "${ISO_DIR}/boot"
VMLINUZ=$(ls "$ROOTFS_DIR"/boot/vmlinuz-* 2>/dev/null | head -1)
INITRD=$(ls "$ROOTFS_DIR"/boot/initrd.img-* 2>/dev/null | head -1)

if [[ -z "$VMLINUZ" || -z "$INITRD" ]]; then
    err "Kernel/initrd not found in rootfs (install linux-image-amd64 inside debootstrap)"
fi

cp "$VMLINUZ" "${ISO_DIR}/boot/vmlinuz"
cp "$INITRD"  "${ISO_DIR}/boot/initramfs.img"
ok "Kernel: $(basename $VMLINUZ)"

# ── Step 6: Set up GRUB ──────────────────────────────────────────────────────
info "Setting up GRUB…"
mkdir -p "${ISO_DIR}/boot/grub"
cp "$SCRIPT_DIR/grub/grub.cfg" "${ISO_DIR}/boot/grub/grub.cfg"

# ── Step 7: Build hybrid ISO ─────────────────────────────────────────────────
info "Building bootable hybrid ISO: $OUTPUT…"
# --modules ensures iso9660 and search_label are embedded in core.img for BIOS boot.
# grub-efi-amd64-bin provides the EFI image; grub-pc-bin provides the BIOS image.
# The result is a hybrid ISO that boots from both BIOS (El Torito) and UEFI.
grub-mkrescue \
    -o "$OUTPUT" \
    --modules="iso9660 search search_label part_gpt part_msdos" \
    "$ISO_DIR" \
    -- -volid "IRONKEY" 2>&1 | tail -5

ok "ISO built: $OUTPUT ($(du -sh "$OUTPUT" | cut -f1))"

# ── Cleanup ──────────────────────────────────────────────────────────────────
info "Cleaning up build directory…"
rm -rf "$BUILD_DIR"

echo ""
echo "  ◈ IronKey ISO ready: $OUTPUT"
echo "  Write to USB with: sudo dd if=$OUTPUT of=/dev/sdX bs=4M status=progress conv=fsync"
echo "  Or use: rufus / balenaEtcher on Windows/macOS"
