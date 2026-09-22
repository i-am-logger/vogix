#!/bin/sh
# GPU enumeration for the shell's GPU cell: one tab-separated line per DRM
# card, carrying the evidence Services/lib/gpu.js picks a busy source from:
#
#   card driver power/control boot_vga pci-address busy-path idle-path smi
#
# busy-path is amdgpu's gpu_busy_percent; idle-path is the i915 RC6 or xe
# GT-idle residency counter; smi is 1 when nvidia-smi is on PATH. Absent
# values are "-". $1 is the sysfs root (default /sys), so the logic check
# runs this against a fixture tree.
sys=${1:-/sys}
smi=0
command -v nvidia-smi >/dev/null 2>&1 && smi=1

for card in "$sys"/class/drm/card*; do
  name=${card##*/}
  # Connector entries (card1-DP-1) share the prefix.
  case $name in *-*) continue ;; esac
  dev=$card/device
  [ -d "$dev" ] || continue

  driver=$(readlink "$dev/driver")
  driver=${driver##*/}
  control=$(cat "$dev/power/control" 2>/dev/null)
  bootvga=$(cat "$dev/boot_vga" 2>/dev/null)
  pci=$(readlink -f "$dev")
  pci=${pci##*/}

  busy=-
  [ -r "$dev/gpu_busy_percent" ] && busy=$dev/gpu_busy_percent

  idle=-
  for f in "$card/gt/gt0/rc6_residency_ms" "$card/power/rc6_residency_ms" \
    "$dev/tile0/gt0/gtidle/idle_residency_ms"; do
    if [ -r "$f" ]; then
      idle=$f
      break
    fi
  done

  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "${driver:--}" \
    "${control:--}" "${bootvga:--}" "${pci:--}" "$busy" "$idle" "$smi"
done
