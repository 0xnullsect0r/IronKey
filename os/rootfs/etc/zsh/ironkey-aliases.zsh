# IronKey — comprehensive tool aliases
# All tools below are pre-installed on the USB image.

# ──────────────────────────────────────────────────────────────────────────────
# Rust-native CLI replacements
# ──────────────────────────────────────────────────────────────────────────────

# eza — ls replacement with icons and colors
if command -v eza &>/dev/null; then
    alias ls='eza --icons --color=always'
    alias ll='eza -la --icons --color=always --git'
    alias la='eza -a --icons --color=always'
    alias lt='eza --tree --icons --color=always'
    alias llt='eza --tree -la --icons --color=always'
fi

# bat — cat replacement with syntax highlighting
if command -v bat &>/dev/null; then
    alias cat='bat --style=plain'
    alias catp='bat'
    alias batl='bat --style=full'
fi

# fd — find replacement
if command -v fd &>/dev/null; then
    alias find='fd'
fi

# ripgrep — grep replacement
if command -v rg &>/dev/null; then
    alias grep='rg'
fi

# dust — du replacement
if command -v dust &>/dev/null; then
    alias du='dust'
fi

# hexyl — hex viewer
if command -v hexyl &>/dev/null; then
    alias hexdump='hexyl'
    alias xxd='hexyl'
fi

# sd — sed replacement
if command -v sd &>/dev/null; then
    alias sed='sd'
fi

# procs — ps replacement
if command -v procs &>/dev/null; then
    alias ps='procs'
fi

# bottom — top replacement
if command -v btm &>/dev/null; then
    alias top='btm'
    alias htop='btm'
fi

# xcp — cp replacement with progress
if command -v xcp &>/dev/null; then
    alias cp='xcp'
fi

# ouch — universal archive tool
if command -v ouch &>/dev/null; then
    alias compress='ouch compress'
    alias decompress='ouch decompress'
    alias lsarchive='ouch list'
fi

# ──────────────────────────────────────────────────────────────────────────────
# Partition table editors
# ──────────────────────────────────────────────────────────────────────────────

# GParted: launch in a nested cage window
alias gparted-launch='cage -d -- gparted &'

# sgdisk shortcuts
alias sgdisk-list='sgdisk -p'
alias sgdisk-backup='sgdisk --backup'

# parted shortcuts
alias parted-list='parted -l'

# ──────────────────────────────────────────────────────────────────────────────
# Disk & filesystem management
# ──────────────────────────────────────────────────────────────────────────────

# mkfs shortcuts
alias mkext4='mkfs.ext4'
alias mkext3='mkfs.ext3'
alias mkext2='mkfs.ext2'
alias mkfat32='mkfs.fat -F 32'
alias mkfat16='mkfs.fat -F 16'
alias mkexfat='mkfs.exfat'
alias mkntfs='mkfs.ntfs -Q'
alias mkbtrfs='mkfs.btrfs'
alias mkxfs='mkfs.xfs'
alias mkf2fs='mkfs.f2fs'

# fsck shortcuts
alias fschk='fsck'
alias e2chk='e2fsck -f'

# ──────────────────────────────────────────────────────────────────────────────
# Disk imaging & cloning
# ──────────────────────────────────────────────────────────────────────────────

# dd with nice defaults
alias dd-safe='dd bs=4M conv=fsync status=progress'

# ddrescue with retry
alias rescue='ddrescue -d -r3'

# rsync with progress and archive mode
alias rsync-clone='rsync -aHAX --info=progress2'

# pv pipe helpers
alias dd-pv='pv | dd bs=4M conv=fsync'

# partclone shortcuts
alias clone-ext='partclone.ext4'
alias clone-ntfs='partclone.ntfs'
alias clone-fat='partclone.fat'

# ──────────────────────────────────────────────────────────────────────────────
# Disk information & diagnostics
# ──────────────────────────────────────────────────────────────────────────────

# lsblk with useful columns
alias lsblk='lsblk -o NAME,SIZE,TYPE,FSTYPE,MOUNTPOINT,LABEL,UUID,MODEL'
alias lsblkj='lsblk -J'

# blkid with all info
alias blkid-all='blkid -o full'

# smartctl shortcuts
alias smart='smartctl -a'
alias smart-test='smartctl -t short'
alias smart-long='smartctl -t long'
alias smart-health='smartctl -H'

# nvme shortcuts
alias nvme-list='nvme list'
alias nvme-smart='nvme smart-log'
alias nvme-info='nvme id-ctrl'

# hdparm read test (non-destructive)
alias hdparm-speed='hdparm -tT'

# iostat
alias iostat-watch='iostat -x 1'

# iotop shortcuts
alias iotop-once='iotop -b -n 1'

# ──────────────────────────────────────────────────────────────────────────────
# Encryption & secure access
# ──────────────────────────────────────────────────────────────────────────────

# cryptsetup shortcuts
alias luks-open='cryptsetup open --type luks'
alias luks-close='cryptsetup close'
alias luks-status='cryptsetup status'
alias luks-format='cryptsetup luksFormat'

# dislocker shortcuts
alias bl-mount='dislocker-fuse'

# hashdeep
alias hash-dir='hashdeep -r'
alias hash-audit='hashdeep -r -a'

# Checksum shortcuts
alias sha256='sha256sum'
alias sha512='sha512sum'
alias md5='md5sum'

# ──────────────────────────────────────────────────────────────────────────────
# Data recovery & forensics
# ──────────────────────────────────────────────────────────────────────────────

# testdisk / photorec
alias td='testdisk'
alias pr='photorec'

# Sleuth Kit shortcuts
alias mmls-list='mmls'
alias fls-list='fls -r'
alias fsstat-info='fsstat'

# binwalk
alias bw='binwalk'
alias bw-extract='binwalk -e'

# strings with min length
alias strings4='strings -n 4'
alias strings8='strings -n 8'

# Hex editor
alias hex='hexedit'

# ──────────────────────────────────────────────────────────────────────────────
# Mounting & filesystem access
# ──────────────────────────────────────────────────────────────────────────────

# APFS
alias mount-apfs='apfs-fuse -o allow_other'

# ZFS
alias zpool-list='zpool list'
alias zfs-list='zfs list'
alias zpool-import-all='zpool import -a'

# FUSE umount
alias fusermount-u='fusermount -u'

# ──────────────────────────────────────────────────────────────────────────────
# Bootloader & MBR tools
# ──────────────────────────────────────────────────────────────────────────────

alias grub-install-mbr='grub-install --target=i386-pc'
alias grub-install-efi='grub-install --target=x86_64-efi'
alias efi-list='efibootmgr -v'

# ──────────────────────────────────────────────────────────────────────────────
# System & process utilities
# ──────────────────────────────────────────────────────────────────────────────

# lsof for mount troubleshooting
alias who-uses='lsof +D'
alias fuser-kill='fuser -km'

# dmesg with human-readable timestamps
alias dmesg='dmesg -T --color=always'
alias dmesg-usb='dmesg -T | grep -i usb'
alias dmesg-disk='dmesg -T | grep -iE "sd[a-z]|nvme|mmcblk|ata|scsi"'

# udevadm shortcuts
alias udev-watch='udevadm monitor'
alias udev-info='udevadm info'

# Hardware listing
alias lspci='lspci -v'
alias lsusb='lsusb -v 2>/dev/null | head -100'

# Memory
alias meminfo='free -h'
alias vmstat-watch='vmstat 1'

# ──────────────────────────────────────────────────────────────────────────────
# Text processing
# ──────────────────────────────────────────────────────────────────────────────

# jq pretty-print
alias jqp='jq .'
alias lsblkjq='lsblk -J | jq .'

# less with colors
alias less='less -R'
alias more='less'

# ──────────────────────────────────────────────────────────────────────────────
# Miscellaneous
# ──────────────────────────────────────────────────────────────────────────────

# Compression shortcuts (use ouch when available, else native tools)
if command -v ouch &>/dev/null; then
    alias tar-extract='ouch decompress'
    alias tar-create='ouch compress'
else
    alias tar-extract='tar -xvf'
    alias tar-create='tar -cvf'
fi

alias tar-list='tar -tvf'
alias gz-extract='gunzip'
alias bz2-extract='bunzip2'
alias xz-extract='xz -d'
alias zst-extract='zstd -d'

# stat with nice output
alias stat='stat -c "%n: type=%F size=%s perms=%A owner=%U:%G mtime=%y"'

# tree with colors
alias tree='tree -C'

# syncfs
alias sync-all='sync && echo "Buffers flushed"'

# Quick block device size
alias blk-size='blockdev --getsize64'

# wipefs
alias wipe-sigs='wipefs -a'

# IronKey specific
alias ironkey-restart='systemctl restart ironkey'
alias ik='ironkey'

# Safety aliases (prompt before destructive ops)
alias rm='rm -i'
alias mv='mv -i'
alias cp='cp -i'

# But allow force when needed
alias rmf='/bin/rm -f'
alias rmrf='/bin/rm -rf'
