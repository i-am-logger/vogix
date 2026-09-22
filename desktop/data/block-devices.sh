#!/bin/sh
# Block-device identity for SysStat's I/O rates, by the kernel names
# /proc/diskstats uses. One tab-separated line per fact, parsed by
# Services/lib/blockdev.js:
#
#   mount <target> <kname>   a mount backed by a block device
#   swap <kname>             a swap device
#   disk <kname>             a whole physical disk
#
# Sources resolve through their device-node symlinks, so /dev/mapper/<name>
# (LUKS, LVM) reports dm-N. A "disk" has a device behind it (not dm, md,
# loop or zram) and is not a hidden NVMe multipath leg. Mount targets keep
# findmnt's \xNN escapes. $1 is the sysfs root, $2 the swaps table and $3
# the device directory (defaults /sys, /proc/swaps, /dev), so the logic
# check runs this against fixtures.
sys=${1:-/sys}
swaps=${2:-/proc/swaps}
devroot=${3:-/dev}

# Sets $name to the kernel name of a device node under $devroot,
# following a symlink (/dev/mapper/*, /dev/disk/by-*) to the node it
# names; fails for anything else. Plain nodes cost no subprocess.
kname() {
  case $1 in "$devroot"/*) ;; *) return 1 ;; esac
  if [ -L "$1" ]; then
    node=$(realpath -e "$1" 2>/dev/null) || return 1
  else
    [ -e "$1" ] || return 1
    node=$1
  fi
  name=${node##*/}
}

findmnt -rn -o TARGET,SOURCE 2>/dev/null | while read -r target source; do
  # A bind mount or btrfs subvolume names its subtree in brackets.
  kname "${source%%\[*}" || continue
  printf 'mount\t%s\t%s\n' "$target" "$name"
done

tail -n +2 "$swaps" 2>/dev/null | while read -r file _; do
  kname "$file" || continue
  printf 'swap\t%s\n' "$name"
done

for disk in "$sys"/block/*; do
  [ -e "$disk/device" ] || continue
  [ "$(cat "$disk/hidden" 2>/dev/null)" = 1 ] && continue
  printf 'disk\t%s\n' "${disk##*/}"
done
