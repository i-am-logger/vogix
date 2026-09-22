#!/bin/sh
# Fan enumeration for the shell's fan cells: one tab-separated line per
# hwmon tachometer, carrying what Services/lib/fans.js names and scales
# it by:
#
#   input-path chip index label max
#
# label is fanN_label and max is fanN_max in RPM; absent values are "-".
# $1 is the sysfs root (default /sys), so the logic check runs this
# against a fixture tree.
sys=${1:-/sys}

for input in "$sys"/class/hwmon/hwmon*/fan*_input; do
  [ -r "$input" ] || continue
  dir=${input%/*}
  n=${input##*/fan}
  n=${n%_input}
  chip=$(cat "$dir/name" 2>/dev/null)
  label=$(cat "$dir/fan${n}_label" 2>/dev/null)
  max=$(cat "$dir/fan${n}_max" 2>/dev/null)
  printf '%s\t%s\t%s\t%s\t%s\n' "$input" "${chip:--}" "$n" "${label:--}" "${max:--}"
done
