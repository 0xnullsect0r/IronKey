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
# Keep --include to the absolute minimum needed for debootstrap's second stage
# to succeed. Complex packages (cage, Wayland libs, firmware) are installed
# separately inside a full chroot in Step 3a, where /proc and /sys are mounted
# and apt-get can properly run all post-install scripts.
info "Bootstrapping minimal Debian rootfs (this may take a few minutes)…"
debootstrap \
    --arch=amd64 \
    --variant=minbase \
    --components=main,non-free-firmware \
    --include=systemd,systemd-sysv,dbus,bash,udev,kmod,linux-image-amd64,initramfs-tools,live-boot,live-boot-initramfs-tools,live-config \
    bookworm \
    "$ROOTFS_DIR" \
    http://deb.debian.org/debian

ok "Debootstrap complete"

# ── Force dpkg non-interactive mode in chroot (must happen before ANY dpkg) ──
# /etc/zsh/zshrc is a conffile owned by zsh-common. If it already exists on
# disk when zsh-common is configured, dpkg asks an interactive question. Since
# Docker has no stdin TTY, dpkg reads EOF and aborts. We prevent this globally
# by writing a dpkg config file that forces non-interactive behaviour for ALL
# dpkg invocations inside this chroot — dpkg --configure -a, apt-get install,
# and any maintainer script that calls dpkg internally.
mkdir -p "$ROOTFS_DIR/etc/dpkg/dpkg.cfg.d"
cat > "$ROOTFS_DIR/etc/dpkg/dpkg.cfg.d/99-noninteractive" <<'EOF'
force-confold
force-confdef
EOF
# Also tell apt not to allocate a PTY for dpkg (suppresses more interactive prompts).
mkdir -p "$ROOTFS_DIR/etc/apt/apt.conf.d"
cat > "$ROOTFS_DIR/etc/apt/apt.conf.d/99-noninteractive" <<'EOF'
Dpkg::Use-Pty "false";
EOF

# ── Step 3: Pre-chroot config ────────────────────────────────────────────────
# Write files that must exist BEFORE the chroot apt-get runs.
# IMPORTANT: Do NOT copy /etc/zsh/* or any other file that dpkg owns as a
# conffile here — if the file already exists when the package is installed,
# dpkg asks an interactive "keep/replace?" question. Since Docker's stdin is
# closed, that question hits EOF and aborts the build. We copy all custom
# etc/ files in Step 3b, AFTER apt-get finishes.

install -Dm755 "$IRONKEY_BIN" "$ROOTFS_DIR/usr/bin/ironkey"

# Basic system identity (not owned as conffiles by any package we install).
echo "ironkey" > "$ROOTFS_DIR/etc/hostname"

# Minimal fstab — live-boot manages root overlayfs; no extra tmpfs entries.
cat > "$ROOTFS_DIR/etc/fstab" <<'EOF'
proc    /proc   proc    defaults    0 0
EOF

# Empty machine-id so systemd generates a transient one on each boot.
> "$ROOTFS_DIR/etc/machine-id"

# Temporary resolv.conf for chroot apt-get calls.
echo "nameserver 1.1.1.1" > "$ROOTFS_DIR/etc/resolv.conf"

# Initramfs-tools config — must exist BEFORE apt-get runs because installing
# packages triggers update-initramfs, which reads these files.
mkdir -p "$ROOTFS_DIR/etc/initramfs-tools"
cat > "$ROOTFS_DIR/etc/initramfs-tools/modules" <<'EOF'
overlay
squashfs
loop
usb_storage
uas
EOF
# gzip: universally supported; most: include all likely-needed drivers.
grep -q '^COMPRESS=' "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf" 2>/dev/null \
    && sed -i 's/^COMPRESS=.*/COMPRESS=gzip/' "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf" \
    || echo "COMPRESS=gzip" >> "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf"
grep -q '^MODULES=' "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf" 2>/dev/null \
    && sed -i 's/^MODULES=.*/MODULES=most/' "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf" \
    || echo "MODULES=most" >> "$ROOTFS_DIR/etc/initramfs-tools/initramfs.conf"

# ── Step 3a: Full chroot package configuration ───────────────────────────────
# debootstrap's second stage runs post-install scripts under a policy-rc.d that
# blocks daemon starts, and WITHOUT a proper /proc or /sys. This means:
#   • systemd's post-install may not fully complete
#   • packages that need /proc during configuration are silently skipped
# We now mount proc/sys/dev, run dpkg --configure -a to finish any partial
# configurations, then apt-get install the UI + firmware packages that need
# a working chroot environment.
info "Configuring packages inside chroot…"

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

# Finish configuring any packages that debootstrap left in a partial state.
# --force-confold/--force-confdef: belt-and-suspenders even though
# /etc/dpkg/dpkg.cfg.d/99-noninteractive already sets these globally.
chroot "$ROOTFS_DIR" env DEBIAN_FRONTEND=noninteractive \
    dpkg --force-confold --force-confdef --configure -a

# Install the remaining packages now that the chroot is fully functional.
# cage: Wayland kiosk compositor (has many Wayland/DRM/libinput dependencies)
# zsh + dbus-user-session + firmware-realtek: user shell, D-Bus session, NIC fw
#
# --force-confold: if a package conffile already exists on disk (from a prior
#   build or from another package), keep the installed version without prompting.
#   Prevents "end of file on stdin at conffile prompt" aborts in Docker.
chroot "$ROOTFS_DIR" env DEBIAN_FRONTEND=noninteractive \
    apt-get install -y --no-install-recommends \
        -o Dpkg::Options::="--force-confold" \
        -o Dpkg::Options::="--force-confdef" \
        cage \
        zsh \
        dbus-user-session \
        firmware-realtek

# ── Step 3b: Copy custom etc/ files ──────────────────────────────────────────
# Done AFTER apt-get so our files overwrite package defaults silently (no dpkg
# conffile tracking involved — we write directly to the filesystem).
info "Copying rootfs config files…"
cp -r "$SCRIPT_DIR/rootfs/etc/." "$ROOTFS_DIR/etc/"
chmod 644 "$ROOTFS_DIR/etc/live/boot.conf"
chmod 644 "$ROOTFS_DIR/etc/zsh/zshrc.local"            2>/dev/null || true
chmod 644 "$ROOTFS_DIR/etc/zsh/ironkey-aliases.zsh"  2>/dev/null || true
chmod 644 "$ROOTFS_DIR/etc/starship.toml"             2>/dev/null || true
ok "Config files copied"

# ── Verify the systemd binary is actually present ─────────────────────────────
# If it's missing the next stage will panic with "can't execute init".
if [[ ! -f "$ROOTFS_DIR/usr/lib/systemd/systemd" ]]; then
    _chroot_cleanup
    err "/usr/lib/systemd/systemd not found in rootfs — debootstrap or dpkg --configure -a failed"
fi
ok "systemd binary confirmed: /usr/lib/systemd/systemd"

# ── Step 3c: systemd target + service wiring ─────────────────────────────────
# Set default target to graphical so cage/IronKey launches on boot.
mkdir -p "$ROOTFS_DIR/etc/systemd/system/graphical.target.wants"
ln -sf /etc/systemd/system/ironkey.service \
    "$ROOTFS_DIR/etc/systemd/system/graphical.target.wants/ironkey.service"
# Use /usr/lib path explicitly — bookworm uses usrmerge so /lib→usr/lib, but
# symlink targets from outside the chroot should use the canonical path.
ln -sf /usr/lib/systemd/system/graphical.target \
    "$ROOTFS_DIR/etc/systemd/system/default.target"

ok "Rootfs configured"

# ── Step 3d: Regenerate initramfs inside chroot ───────────────────────────────
# Now that all packages are installed and configured, rebuild the initramfs.
# The debootstrap-generated initrd was built without /proc+/sys so live-boot
# hooks were absent — delete it first so we get a guaranteed clean build.
info "Regenerating initramfs with live-boot hooks…"

rm -f "$ROOTFS_DIR"/boot/initrd.img-*

chroot "$ROOTFS_DIR" \
    env DEBIAN_FRONTEND=noninteractive \
    update-initramfs -c -k all

INITRD_CHECK=$(ls "$ROOTFS_DIR"/boot/initrd.img-* 2>/dev/null | head -1)
if [[ -z "$INITRD_CHECK" ]]; then
    _chroot_cleanup
    err "update-initramfs produced no initrd — check the output above"
fi
ok "Initramfs: $(basename "$INITRD_CHECK") ($(du -sh "$INITRD_CHECK" | cut -f1))"

_chroot_cleanup
trap - EXIT

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
