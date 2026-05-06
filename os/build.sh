#!/usr/bin/env bash
# IronKey OS Build Script
# Builds a minimal bootable USB image containing the IronKey application.
#
# Requirements:
#   - Debian/Ubuntu host with: debootstrap squashfs-tools grub-pc-bin grub-efi-amd64-bin
#   - Root privileges
#   - Rust toolchain (rustup + cargo)
#   - At least 4 GB free disk space in /tmp
#
# Usage:
#   sudo bash os/build.sh [output.iso]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT="${1:-ironkey.iso}"
BUILD_DIR="/tmp/ironkey-build-$$"
ROOTFS_DIR="${BUILD_DIR}/rootfs"
ISO_DIR="${BUILD_DIR}/iso"

# ── Colour output ────────────────────────────────────────────────────────────
CYAN='\033[0;36m'; GREEN='\033[0;32m'; RED='\033[0;31m'; RESET='\033[0m'
info()  { echo -e "${CYAN}▶ $*${RESET}"; }
ok()    { echo -e "${GREEN}✔ $*${RESET}"; }
err()   { echo -e "${RED}✗ $*${RESET}" >&2; exit 1; }

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
    --variant=minbase \
    --include=systemd,dbus,cage,zsh,bash,udev \
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

# Enable IronKey service
mkdir -p "$ROOTFS_DIR/etc/systemd/system/multi-user.target.wants"
ln -sf /etc/systemd/system/ironkey.service \
    "$ROOTFS_DIR/etc/systemd/system/multi-user.target.wants/ironkey.service"

# Set hostname
echo "ironkey" > "$ROOTFS_DIR/etc/hostname"

# tmpfs overlay for runtime writes
cat > "$ROOTFS_DIR/etc/fstab" <<'EOF'
tmpfs   /tmp    tmpfs   defaults,size=256m  0 0
tmpfs   /var    tmpfs   defaults,size=128m  0 0
tmpfs   /run    tmpfs   defaults,size=64m   0 0
EOF

ok "Rootfs configured"

# ── Step 4: Pack rootfs into squashfs ────────────────────────────────────────
info "Packing rootfs into squashfs…"
SQUASHFS="${ISO_DIR}/live/filesystem.squashfs"
mkdir -p "${ISO_DIR}/live"
mksquashfs "$ROOTFS_DIR" "$SQUASHFS" \
    -comp zstd -Xcompression-level 19 \
    -noappend \
    -e "$ROOTFS_DIR/proc" \
    -e "$ROOTFS_DIR/sys" \
    -e "$ROOTFS_DIR/dev"
ok "squashfs: $(du -sh "$SQUASHFS" | cut -f1)"

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
grub-mkrescue \
    -o "$OUTPUT" \
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
