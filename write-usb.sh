#!/usr/bin/env bash
# IronKey USB Flash Utility
#
# Writes an IronKey .iso to a USB drive.
#
# Usage:
#   sudo ./write-usb.sh --drive /dev/sdX [--iso ironkey.iso] [--yes]
#
# Options:
#   --drive <dev>   Target block device (required), e.g. /dev/sdb
#   --iso   <file>  Path to the .iso file (default: latest ironkey-*.iso in CWD)
#   --yes           Skip confirmation prompt (for scripting)
#   --help          Show this help message

set -euo pipefail

# ── Colour helpers ────────────────────────────────────────────────────────────
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${CYAN}▶ $*${RESET}"; }
ok()    { echo -e "${GREEN}✔ $*${RESET}"; }
warn()  { echo -e "${YELLOW}⚠ $*${RESET}"; }
err()   { echo -e "${RED}✗ $*${RESET}" >&2; exit 1; }
bold()  { echo -e "${BOLD}$*${RESET}"; }

# ── Usage ─────────────────────────────────────────────────────────────────────
usage() {
  cat <<EOF
${BOLD}IronKey USB Flash Utility${RESET}

  Writes an IronKey .iso to a USB drive.

${BOLD}Usage:${RESET}
  sudo ./write-usb.sh --drive /dev/sdX [--iso ironkey.iso] [--yes]

${BOLD}Options:${RESET}
  --drive <dev>   Target block device (required), e.g. /dev/sdb
  --iso   <file>  Path to the .iso file (default: latest ironkey-*.iso in CWD)
  --yes           Skip confirmation prompt (for scripting)
  --help          Show this help message

${BOLD}Examples:${RESET}
  sudo ./write-usb.sh --drive /dev/sdb
  sudo ./write-usb.sh --drive /dev/sdb --iso ironkey-v0.1.0.iso
  sudo ./write-usb.sh --drive /dev/sdb --yes   # no confirmation prompt
EOF
}

# ── Parse arguments ───────────────────────────────────────────────────────────
DRIVE=""
ISO=""
YES=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --drive)
      DRIVE="${2:-}"
      [[ -z "$DRIVE" ]] && err "--drive requires a device path argument"
      shift 2
      ;;
    --iso)
      ISO="${2:-}"
      [[ -z "$ISO" ]] && err "--iso requires a file path argument"
      shift 2
      ;;
    --yes|-y)
      YES=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      err "Unknown option: $1  (run with --help for usage)"
      ;;
  esac
done

# ── Root check ────────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]] || err "This script must be run as root.  Try: sudo ./write-usb.sh $*"

# ── Validate --drive ──────────────────────────────────────────────────────────
[[ -z "$DRIVE" ]] && err "--drive is required.  Run with --help for usage."

[[ -e "$DRIVE" ]] || err "Device not found: $DRIVE"

[[ -b "$DRIVE" ]] || err "$DRIVE is not a block device."

# Reject partition paths like /dev/sda1 — we want the whole disk
if [[ "$DRIVE" =~ [0-9]$ ]]; then
  PARENT="${DRIVE%%[0-9]*}"
  err "$DRIVE looks like a partition. Did you mean ${PARENT}?  Supply the whole disk, not a partition."
fi

# ── Resolve --iso ─────────────────────────────────────────────────────────────
if [[ -z "$ISO" ]]; then
  # Auto-detect: pick the newest ironkey-*.iso in CWD
  ISO=$(ls -t ironkey-*.iso 2>/dev/null | head -1 || true)
  if [[ -z "$ISO" ]]; then
    ISO=$(ls -t *.iso 2>/dev/null | head -1 || true)
  fi
  [[ -z "$ISO" ]] && err "No .iso file found in the current directory. Use --iso to specify one."
  info "Auto-detected ISO: $ISO"
fi

[[ -f "$ISO" ]] || err "ISO file not found: $ISO"

ISO_SIZE=$(du -sh "$ISO" | cut -f1)

# ── Show drive info ───────────────────────────────────────────────────────────
echo ""
bold "═══════════════════════════════════════════════════════════"
bold "  IronKey USB Flash Utility"
bold "═══════════════════════════════════════════════════════════"
echo ""
info "ISO:   $ISO ($ISO_SIZE)"
info "Drive: $DRIVE"
echo ""
echo "Drive information:"
echo ""
lsblk -o NAME,SIZE,TYPE,VENDOR,MODEL,MOUNTPOINT "$DRIVE" 2>/dev/null || lsblk "$DRIVE" 2>/dev/null || true
echo ""

# ── Warn and confirm ──────────────────────────────────────────────────────────
warn "ALL DATA ON ${DRIVE} WILL BE PERMANENTLY ERASED."
echo ""

if [[ "$YES" == false ]]; then
  DRIVE_SHORT="${DRIVE##*/}"
  echo -e "${YELLOW}To confirm, type the drive name ${BOLD}${DRIVE_SHORT}${RESET}${YELLOW} and press Enter:${RESET}"
  read -r CONFIRM

  if [[ "$CONFIRM" != "$DRIVE_SHORT" ]]; then
    echo ""
    err "Confirmation did not match. Aborting."
  fi
  echo ""
fi

# ── Unmount all partitions ────────────────────────────────────────────────────
info "Unmounting any mounted partitions on ${DRIVE}…"
# Find all partitions of this disk that are currently mounted
while IFS= read -r PART; do
  if mount | grep -q "^${PART} "; then
    info "  Unmounting ${PART}…"
    umount "$PART" 2>/dev/null || warn "  Could not unmount ${PART} — continuing anyway"
  fi
done < <(lsblk -lno NAME "$DRIVE" | awk "NR>1{print \"/dev/\" \$1}")

# ── Write the ISO ─────────────────────────────────────────────────────────────
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
info "Running sync…"
sync

echo ""
ok "Done! $ISO has been written to $DRIVE."
echo ""
echo -e "  ${CYAN}Remove the drive safely and boot your target machine from it.${RESET}"
echo -e "  ${CYAN}Set USB as first boot device in BIOS/UEFI, disable Secure Boot.${RESET}"
echo ""
