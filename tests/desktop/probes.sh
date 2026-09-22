#!/bin/sh
# The shell's sysfs probes, run against fixture trees shaped like the real
# kernel layout. $1 is the packaged desktop/data directory.
set -eu
data=$1
fail=0

# expect NAME EXPECTED ACTUAL
expect() {
  if [ "$2" != "$3" ]; then
    printf 'FAIL %s\n--- expected\n%s\n--- got\n%s\n' "$1" "$2" "$3"
    fail=1
  else
    printf 'PASS %s\n' "$1"
  fi
}

# pci_gpu ROOT CARD PCI DRIVER CONTROL BOOT_VGA — a DRM card whose device
# link resolves to a PCI function bound to DRIVER.
pci_gpu() {
  dev=$1/devices/pci0000:00/$3
  mkdir -p "$dev/power" "$1/bus/pci/drivers/$4" "$1/class/drm/$2"
  ln -s "../../../bus/pci/drivers/$4" "$dev/driver"
  echo "$5" >"$dev/power/control"
  echo "$6" >"$dev/boot_vga"
  ln -s "../../../devices/pci0000:00/$3" "$1/class/drm/$2/device"
  # A connector entry beside the card, as the kernel lists them.
  mkdir -p "$1/class/drm/$2-DP-1"
  ln -s "../$2" "$1/class/drm/$2-DP-1/device"
}

tab=$(printf '\t')

# An AMD desktop: one amdgpu card with a busy percent.
amd=$TMPDIR/sys-amd
pci_gpu "$amd" card1 0000:78:00.0 amdgpu on 1
echo 7 >"$amd/devices/pci0000:00/0000:78:00.0/gpu_busy_percent"
expect "amdgpu card" \
  "card1${tab}amdgpu${tab}on${tab}1${tab}0000:78:00.0${tab}$amd/class/drm/card1/device/gpu_busy_percent${tab}-${tab}0" \
  "$(sh "$data/gpu-probe.sh" "$amd")"

# A hybrid laptop in PRIME sync: an i915 iGPU (per-GT and legacy RC6
# counters; the per-GT one wins) plus an always-on NVIDIA dGPU, with
# nvidia-smi on PATH.
hyb=$TMPDIR/sys-hybrid
pci_gpu "$hyb" card0 0000:00:02.0 i915 auto 1
mkdir -p "$hyb/class/drm/card0/gt/gt0" "$hyb/class/drm/card0/power"
echo 100 >"$hyb/class/drm/card0/gt/gt0/rc6_residency_ms"
echo 100 >"$hyb/class/drm/card0/power/rc6_residency_ms"
pci_gpu "$hyb" card1 0000:01:00.0 nvidia on 0
mkdir -p "$TMPDIR/bin"
printf '#!/bin/sh\nexit 0\n' >"$TMPDIR/bin/nvidia-smi"
chmod +x "$TMPDIR/bin/nvidia-smi"
expect "hybrid i915 + nvidia" \
  "card0${tab}i915${tab}auto${tab}1${tab}0000:00:02.0${tab}-${tab}$hyb/class/drm/card0/gt/gt0/rc6_residency_ms${tab}1
card1${tab}nvidia${tab}on${tab}0${tab}0000:01:00.0${tab}-${tab}-${tab}1" \
  "$(PATH=$TMPDIR/bin:$PATH sh "$data/gpu-probe.sh" "$hyb")"

# An xe card: the GT-idle residency lives under the PCI device's tile.
xe=$TMPDIR/sys-xe
pci_gpu "$xe" card0 0000:00:02.0 xe on 1
mkdir -p "$xe/devices/pci0000:00/0000:00:02.0/tile0/gt0/gtidle"
echo 5 >"$xe/devices/pci0000:00/0000:00:02.0/tile0/gt0/gtidle/idle_residency_ms"
expect "xe card" \
  "card0${tab}xe${tab}on${tab}1${tab}0000:00:02.0${tab}-${tab}$xe/class/drm/card0/device/tile0/gt0/gtidle/idle_residency_ms${tab}0" \
  "$(sh "$data/gpu-probe.sh" "$xe")"

# hwmon: a board chip with an unlabelled tachometer, a cooler chip with a
# labelled one that reports its maximum, and a temperature-only chip.
hw=$TMPDIR/sys-hwmon
mkdir -p "$hw/class/hwmon/hwmon0" "$hw/class/hwmon/hwmon1" "$hw/class/hwmon/hwmon2"
echo nct6799 >"$hw/class/hwmon/hwmon0/name"
echo 812 >"$hw/class/hwmon/hwmon0/fan2_input"
echo dell_smm >"$hw/class/hwmon/hwmon1/name"
echo 2400 >"$hw/class/hwmon/hwmon1/fan1_input"
echo "Processor Fan" >"$hw/class/hwmon/hwmon1/fan1_label"
echo 4900 >"$hw/class/hwmon/hwmon1/fan1_max"
echo k10temp >"$hw/class/hwmon/hwmon2/name"
echo 41000 >"$hw/class/hwmon/hwmon2/temp1_input"
expect "hwmon fans" \
  "$hw/class/hwmon/hwmon0/fan2_input${tab}nct6799${tab}2${tab}-${tab}-
$hw/class/hwmon/hwmon1/fan1_input${tab}dell_smm${tab}1${tab}Processor Fan${tab}4900" \
  "$(sh "$data/fan-probe.sh" "$hw")"

# No tachometer anywhere: no output at all.
expect "no fans" "" "$(sh "$data/fan-probe.sh" "$amd")"

exit $fail
