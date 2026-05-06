#!/usr/bin/env bash
# IronKey — Build & Flash Utility
#
# Builds IronKey from source and writes the ISO directly to a USB drive.
#
# Usage:
#   sudo ./write-usb.sh --drive /dev/sdX [options]
#
# Options:
#   --drive  <dev>    Target block device (required), e.g. /dev/sdb
#   --skip-build      Skip the build step and use an existing ISO
#   --iso    <file>   ISO to flash when --skip-build is used
#                     (default: latest ironkey-*.iso in CWD)
#   --qemu            Boot the built ISO in QEMU instead of flashing to USB
#                     (requires qemu-system-x86_64 and KVM)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ── Colour helpers ────────────────────────────────────────────────────────────
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${CYAN}▶  $*${RESET}"; }
ok()    { echo -e "${GREEN}✔  $*${RESET}"; }
warn()  { echo -e "${YELLOW}⚠  $*${RESET}"; }
err()   { echo -e "${RED}✗  $*${RESET}" >&2; exit 1; }
header(){ echo -e "\n${BOLD}$*${RESET}"; }
rule()  { echo -e "${BOLD}$(printf '═%.0s' {1..60})${RESET}"; }

# ── Usage ─────────────────────────────────────────────────────────────────────
usage() {
  cat <<EOF
${BOLD}IronKey — Build & Flash Utility${RESET}

  Builds IronKey from source and writes it directly to a USB drive.

${BOLD}Usage:${RESET}
  sudo ./write-usb.sh --drive /dev/sdX [options]

${BOLD}Options:${RESET}
  --drive  <dev>    Target block device (required), e.g. /dev/sdb
  --skip-build      Skip the build step and flash an existing ISO
  --iso    <file>   ISO to flash when using --skip-build
                    (default: latest ironkey-*.iso in current directory)
  --qemu            Boot the built ISO in QEMU instead of flashing to USB
                    Requires: qemu-system-x86_64 and /dev/kvm access
  --yes             Skip the confirmation prompt
  --help            Show this help message

${BOLD}Examples:${RESET}
  # Build from source and write to /dev/sdb
  sudo ./write-usb.sh --drive /dev/sdb

  # Test the ISO in QEMU before flashing
  sudo ./write-usb.sh --qemu

  # Skip build, flash an existing ISO
  sudo ./write-usb.sh --drive /dev/sdb --skip-build
  sudo ./write-usb.sh --drive /dev/sdb --skip-build --iso ironkey-v1.0.0.iso

  # Non-interactive (for scripting)
  sudo ./write-usb.sh --drive /dev/sdb --yes
EOF
}

# ── Parse arguments ───────────────────────────────────────────────────────────
DRIVE=""
ISO=""
SKIP_BUILD=false
YES=false
QEMU=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --drive)
      DRIVE="${2:-}"; [[ -z "$DRIVE" ]] && err "--drive requires a device path"
      shift 2 ;;
    --iso)
      ISO="${2:-}"; [[ -z "$ISO" ]] && err "--iso requires a file path"
      shift 2 ;;
    --skip-build)
      SKIP_BUILD=true; shift ;;
    --qemu)
      QEMU=true; shift ;;
    --yes|-y)
      YES=true; shift ;;
    --help|-h)
      usage; exit 0 ;;
    *)
      err "Unknown option: $1  (use --help for usage)" ;;
  esac
done

# ── Root check ────────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]] || err "Must be run as root.  Try: sudo $0 $*"

# ── Validate --drive (not required when --qemu is set) ────────────────────────
if [[ "$QEMU" == false ]]; then
  [[ -z "$DRIVE" ]] && err "--drive is required (or use --qemu to test in a VM). Use --help for usage."
  [[ -e "$DRIVE" ]] || err "Device not found: $DRIVE"
  [[ -b "$DRIVE" ]] || err "$DRIVE is not a block device."
  if [[ "$DRIVE" =~ [0-9]$ ]]; then
    err "$DRIVE looks like a partition. Supply the whole disk (e.g. ${DRIVE%%[0-9]*})."
  fi
fi

# ── Banner ────────────────────────────────────────────────────────────────────
echo ""
rule
echo -e "${BOLD}  ⚿  IronKey — Build & Flash Utility${RESET}"
rule

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 1 — BUILD
# ══════════════════════════════════════════════════════════════════════════════
if [[ "$SKIP_BUILD" == false ]]; then
  header "Phase 1/2 — Building IronKey ISO"
  echo ""

  # Derive a versioned ISO name from git describe, fall back to timestamp
  VERSION=$(git -C "$SCRIPT_DIR" describe --tags --always 2>/dev/null || date +%Y%m%d)
  ISO="${SCRIPT_DIR}/ironkey-${VERSION}.iso"

  info "Version : $VERSION"
  info "Output  : $ISO"
  info "Build script: os/build.sh"
  echo ""

  # Check system build deps
  for dep in debootstrap mksquashfs grub-mkrescue; do
    command -v "$dep" >/dev/null || err \
      "$dep not found. Install build deps first:
  sudo apt-get install -y debootstrap squashfs-tools grub-pc-bin grub-efi-amd64-bin xorriso mtools"
  done

  # Check Rust / build the binary as the invoking user then hand it off
  REAL_USER="${SUDO_USER:-$(whoami)}"
  REAL_HOME=$(getent passwd "$REAL_USER" | cut -d: -f6)
  CARGO_BIN="${REAL_HOME}/.cargo/bin/cargo"
  [[ -x "$CARGO_BIN" ]] || command -v cargo &>/dev/null || \
    err "Rust/cargo not found. Install via: curl https://sh.rustup.rs -sSf | sh"

  CARGO="${CARGO_BIN}"
  command -v cargo &>/dev/null && CARGO="cargo"

  info "Compiling IronKey binary (this may take a few minutes)…"
  sudo -u "$REAL_USER" "$CARGO" build \
    --manifest-path "$SCRIPT_DIR/Cargo.toml" \
    -p ironkey-app \
    --release

  PREBUILT="$SCRIPT_DIR/target/release/ironkey"
  [[ -f "$PREBUILT" ]] || err "Binary not found at $PREBUILT — cargo build may have failed."
  ok "Binary compiled: $(du -sh "$PREBUILT" | cut -f1)"

  info "Running os/build.sh…"
  IRONKEY_PREBUILT="$PREBUILT" bash "$SCRIPT_DIR/os/build.sh" "$ISO"

  [[ -f "$ISO" ]] || err "os/build.sh completed but ISO not found at $ISO"
  ok "ISO built: $ISO ($(du -sh "$ISO" | cut -f1))"

else
  # ── --skip-build: resolve an existing ISO ───────────────────────────────────
  header "Phase 1/2 — Skipping build (--skip-build)"
  echo ""

  if [[ -z "$ISO" ]]; then
    ISO=$(ls -t "$SCRIPT_DIR"/ironkey-*.iso 2>/dev/null | head -1 || true)
    [[ -z "$ISO" ]] && ISO=$(ls -t "$SCRIPT_DIR"/*.iso 2>/dev/null | head -1 || true)
    [[ -z "$ISO" ]] && err "No .iso found. Build first or use --iso <file>."
    info "Auto-detected ISO: $ISO"
  fi

  [[ -f "$ISO" ]] || err "ISO not found: $ISO"
  ok "Using ISO: $ISO ($(du -sh "$ISO" | cut -f1))"
fi

# ══════════════════════════════════════════════════════════════════════════════
# PHASE 2 — QEMU TEST or FLASH
# ══════════════════════════════════════════════════════════════════════════════

if [[ "$QEMU" == true ]]; then
  # ── Boot in QEMU ─────────────────────────────────────────────────────────────
  header "Phase 2/2 — Booting in QEMU"
  echo ""

  command -v qemu-system-x86_64 >/dev/null || \
    err "qemu-system-x86_64 not found. Install with: sudo apt install qemu-system-x86  (or pacman -S qemu-system-x86)"

  ok "ISO: $ISO ($(du -sh "$ISO" | cut -f1))"
  echo ""

  KVM_FLAG=""
  if [[ -r /dev/kvm ]]; then
    KVM_FLAG="-enable-kvm -cpu host"
    info "KVM acceleration enabled."
  else
    warn "/dev/kvm not readable — QEMU will run without KVM (slow)."
  fi

  # UEFI: use OVMF if available (matches how most real machines boot)
  BIOS_FLAG=""
  for OVMF_PATH in \
      /usr/share/ovmf/OVMF.fd \
      /usr/share/edk2/x64/OVMF.fd \
      /usr/share/edk2-ovmf/x64/OVMF_CODE.fd \
      /usr/share/OVMF/OVMF_CODE.fd; do
    if [[ -f "$OVMF_PATH" ]]; then
      BIOS_FLAG="-bios $OVMF_PATH"
      info "UEFI firmware: $OVMF_PATH"
      break
    fi
  done
  if [[ -z "$BIOS_FLAG" ]]; then
    warn "OVMF not found — booting in legacy BIOS mode."
    warn "Install with: sudo apt install ovmf  (or pacman -S edk2-ovmf)"
  fi

  info "Starting QEMU — kernel console output will appear here in the terminal."
  info "Close the QEMU window or press Ctrl-C in this terminal to stop."
  echo ""

  # Run as the real user so the QEMU window appears on their display
  REAL_USER="${SUDO_USER:-$(whoami)}"
  REAL_DISPLAY=$(su -s /bin/sh "$REAL_USER" -c 'echo $DISPLAY' 2>/dev/null || echo ":0")
  REAL_XAUTH=$(su -s /bin/sh "$REAL_USER" -c 'echo $XAUTHORITY' 2>/dev/null || echo "")

  # -serial stdio:       kernel console output (console=tty0) appears in this terminal
  # -machine q35:        modern chipset; better PCIe / IOMMU compatibility
  # -device virtio-gpu:  DRM-capable GPU so cage/Wayland can open a KMS device
  # -usb + usb-tablet:   relative pointer tracking so mouse isn't grabbed
  env DISPLAY="$REAL_DISPLAY" XAUTHORITY="$REAL_XAUTH" \
    sudo -u "$REAL_USER" \
    qemu-system-x86_64 \
      $KVM_FLAG \
      $BIOS_FLAG \
      -machine type=q35 \
      -m 2G \
      -smp 2 \
      -cdrom "$ISO" \
      -boot d \
      -device virtio-gpu \
      -serial stdio \
      -usb -device usb-tablet \
      -no-reboot

  exit 0
fi

# ── FLASH ─────────────────────────────────────────────────────────────────────
header "Phase 2/2 — Flashing to USB"
echo ""

ISO_SIZE=$(du -sh "$ISO" | cut -f1)
info "ISO   : $ISO ($ISO_SIZE)"
info "Drive : $DRIVE"
echo ""

echo "Drive information:"
echo ""
lsblk -o NAME,SIZE,TYPE,VENDOR,MODEL,MOUNTPOINT "$DRIVE" 2>/dev/null || lsblk "$DRIVE" 2>/dev/null || true
echo ""

warn "ALL DATA ON ${DRIVE} WILL BE PERMANENTLY ERASED."
echo ""

# ── Confirmation ──────────────────────────────────────────────────────────────
if [[ "$YES" == false ]]; then
  DRIVE_SHORT="${DRIVE##*/}"
  echo -e "${YELLOW}To confirm, type the drive name ${BOLD}${DRIVE_SHORT}${RESET}${YELLOW} and press Enter:${RESET}"
  read -r CONFIRM
  [[ "$CONFIRM" == "$DRIVE_SHORT" ]] || err "Confirmation did not match. Aborting."
  echo ""
fi

# ── Unmount all partitions ────────────────────────────────────────────────────
info "Unmounting any mounted partitions on ${DRIVE}…"
while IFS= read -r PART; do
  if mount | grep -q "^${PART} "; then
    info "  Unmounting ${PART}…"
    umount "$PART" 2>/dev/null || warn "  Could not unmount ${PART} — continuing anyway"
  fi
done < <(lsblk -lno NAME "$DRIVE" | awk "NR>1{print \"/dev/\" \$1}")

# ── Write ─────────────────────────────────────────────────────────────────────
echo ""
info "Writing $ISO to $DRIVE…"
echo ""

dd \
  if="$ISO" \
  of="$DRIVE" \
  bs=4M \
  status=progress \
  conv=fsync

echo ""
info "Syncing…"
sync

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
rule
ok "IronKey has been written to $DRIVE."
rule
echo ""
echo -e "  ${CYAN}Remove the drive safely, then boot your target machine from it.${RESET}"
echo -e "  ${CYAN}In BIOS/UEFI: set USB as first boot device and disable Secure Boot.${RESET}"
echo ""
