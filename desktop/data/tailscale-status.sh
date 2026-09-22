#!/bin/sh
# Tailnet state for Services/Tailscale.qml, parsed by lib/tailnet.js.
# Line 1 is tailscaled's start time as systemd reports it (`@<epoch s>`,
# empty when unknown). The rest is `absent` when there is no tailscale
# CLI, `down` when the CLI cannot reach tailscaled, or else the
# `tailscale status --json` document.
start=$(systemctl show tailscaled --property=ActiveEnterTimestamp --value \
  --timestamp=unix 2>/dev/null)
printf '%s\n' "$start"
command -v tailscale >/dev/null 2>&1 || {
  echo absent
  exit 0
}
tailscale status --json 2>/dev/null || echo down
