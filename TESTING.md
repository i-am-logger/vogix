# Vogix Automated Testing

## Overview

Vogix is tested in three layers, all run by `nix flake check` except the
first, which `devenv test` runs:

1. **Rust unit tests** (`cargo test`): the CLI parser and its reference
   (every example in [docs/cli.md](docs/cli.md) must parse, and every command
   must have one), `vogix desktop check`, the input engine, theme loading and
   template rendering.
2. **Build-sandbox checks**: pure-Nix evaluation tests, and the desktop
   shell's checks. These run the real QML under quickshell in a headless
   compositor, with no VM.
3. **NixOS VM suites**: a QEMU machine per suite, driving the real `vogix`
   binary as a user would.

## Running Tests

```bash
# Everything CI runs
devenv test          # formatting, clippy, cargo check, cargo test
nix flake check      # every check below, VM suites included

# One check at a time
nix build .#checks.x86_64-linux.smoke -L --no-link
nix build .#checks.x86_64-linux.desktop-smoke -L --no-link

# Evaluation only (fast; builds nothing)
nix flake check --no-build
```

`-L` prints the check's log as it runs; `--no-link` skips the `result`
symlink.

## What Gets Tested

### Build-sandbox checks

| Check | What it proves |
|---|---|
| `nix-unit` | The config generators: appearance, behavior, the Hyprland render in both config dialects (hyprlang and Lua), and the theme.json contract pinned to the same golden line as the Rust template tests, one of which renders it through praxis's semantic mapping. Runs at evaluation, so `--no-build` covers it. |
| `appearance-options` | `programs.vogix.appearance.*` reaches the rendered Hyprland config through the module system. |
| `login-profile` | For a home-manager profile with bash, zsh and fish enabled: no login-time shell text (`programs.bash.profileExtra`, `programs.zsh.profileExtra`/`loginExtra`, `programs.fish.loginShellInit`) names the vogix binary; `vogix-theme-restore` is a oneshot without `RemainAfterExit`, wanted by and ordered after `graphical-session.target`, running the profile's `vogix theme refresh` at `programs.vogix.logLevel`; `config.toml` carries the apply hooks, the greeter's included, as `[hooks."<name>"]` tables and has no `[hardware]` table. Runs at evaluation, so `--no-build` covers it. |
| `desktop-options` | The default `desktop.json` equals `nix/modules/desktop/desktop-json.pin.json` byte for byte; each bar's layout options take exactly the widget registry's names (`desktop/Bar/widgets/registry.json`) that render on that bar's orientation, plus `custom/<name>`, so a widget the registry confines to the other orientation (a title or the oscilloscope on a rail, a rail meter on a horizontal bar), an unknown name, an undefined `custom/<name>` placement or a bad cell name fails evaluation; a `desktop.json` that `vogix desktop check` rejects fails the home-manager build, and the check rejects a misplaced widget in a document that never went through the options; the shell's unit restarts on exit status 75, waits for PipeWire, and runs quickshell without detailed logs. |
| `desktop-runtime` | Every program the shell's QML starts by name is on the `vogix-desktop` unit's own `PATH` (or is a base-system tool or a client of a host daemon), and the NixOS module enables UPower and power-profiles-daemon exactly while a user runs the shell. |
| `desktop-qmllint` | qmllint over the shell's QML against the pinned quickshell, unused imports included, plus the shell's rules: square corners, no raw `Hyprland.dispatch()`, bar widgets open panels beside their bar, tray menus open through `QsMenuAnchor`. |
| `desktop-logic` | The shell's pure logic under Qt Quick Test: the parsers and policies in `desktop/Services/lib/` (block devices and swap, D-Bus service tracking, fan readings and titles, the GPU sample window, the mount gauges a df answer yields, screencast sessions, the tailnet connection record, the output VU's reference for a sink's volume), meter ballistics, bar leases and popup placement; and its sysfs probe scripts against fixture trees. |
| `desktop-smoke` | The real shell starts under a headless cage compositor exactly as its unit starts it, on the shipped default layout plus every registry widget it leaves out, with the verbs driven through the real `vogix desktop` CLI. It covers the bar, panel, power, launcher, gallery, reminder, notification, stats, privacy, custom-cell and keyboard verbs; custom cells on every trigger but a click, and not at all while their bar is parked; CAPS following the input engine's lock file; the window title and mode label; notification times across a restart; an MPRIS player; a hidden bar's samplers stopping; the tailnet record. A geometry probe lays every registry widget out on every bar edge the registry lets it sit on, at the default bar and font sizes, and requires each to fit across its bar, every canvas to measure at least 1x1, and the spectrums to keep their size before the first audio frame. Every widget is fed the widest realistic data it shows (`desktop-geometry-feed.nix`): two full laptop batteries and a mouse on a UPower, a connected headset on a BlueZ (python-dbusmock on a private system bus), an amdgpu card at 100 %, a CPU sensor and the tachometers of three chips in /sys, swap in use in /proc/meminfo, a pending reboot in /run and an impermanent host's mounts (bound in place by bwrap), a PipeWire with a desktop codec's sink and source and a program recording, Hyprland's two sockets (a screencast on the event socket; two monitors, ten workspaces and a special one, and their windows on the request socket), a Qt application's tray icon, a wttr.in answer in wttrbar's cache, do-not-disturb, night light, stay-awake and pending reminders on, and the input engine's mode table, a current mode and CAPS latched; a placement that never shows, or measures 0 on a side, fails, and so does a request quickshell makes of Hyprland that the socket does not expect, or any warning from quickshell's Hyprland IPC. A schema-1 `desktop.json` is refused without taking the shell down. Every log passes a gate for script errors, binding loops, failed components and failed spawns. Every step waits for an event (an answer from the shell, a state a verb reports, a file's content, a process's exit), never a fixed time; the run's overall timeout is its only bound, and a run it stops names the step it was waiting for. cage has no layer-shell, so the bars do not map here. |
| `desktop-taps` | The audio taps against a real PipeWire daemon: nothing runs while nothing plays, a playback stream starts the taps and the output VU, a source reads off, idle or waiting only once its tap process is gone (a tap that cannot exit yet reads `stopping`), the shell's own taps never light the privacy cell's microphone flag while another program's capture does, a dead tap is relaunched, taps wait for a default sink, a hidden bar's sources stop, a PipeWire restart is survived, and the unit writes no debug records. Every step waits for an event (a status, a process exit, the shell on the daemon), never a fixed time; the run's overall timeout is the only bound. |
| `desktop-backgrounds` | Every desktop theme variant ships its background set (the generated background, the aurora shader, merged extras), and the shell ships the shader precompiled. |
| `machine-contract` | The NixOS machine module, at evaluation (so `--no-build` covers it): `/etc/vogix/machine.json` for the dram-rgb, keychron-k2-he and kraken-elite modules is exactly the expected document (owner, drop zone, console, the OpenRGB endpoint with the server's port and `maxProtocol`, the three typed devices); `vogix-openrgb.service` (`BindsTo=`/`After=`/`Upheld by` `openrgb.service`, `stopIfChanged = false`, DynamicUser, loopback only), `vogix-machine.service` (root, `Type=notify`, before `systemd-user-sessions`, `CAP_SYS_TTY_CONFIG` only, closed device policy) and `vogix-machine-resume.service` (after every sleep target) are wired as declared, and each exists exactly when it has something to own; the drop zone belongs to the owner; the system build validates machine.json; the owner defaults to the first vogix user by name and `console.colors` follows the declared one. It refuses an OpenRGB server without readiness, a command whose `argv[0]` is not in the store, an owner that is not a vogix user, devices without `vogix.enable`, OpenRGB devices with `vogix.openrgb` off, bad device names, slots and USB ids, two providers on one device, and the removed `vogix.hardware.themeApply`; the same declarations without the defect are accepted. |
| `machine-contract-validate` | `vogix machine validate`, the owner units' own loader, accepts that rendered machine.json (owner, endpoint, owner units and each device as the loader reads them) and a palette the owner's CLI published in the `machine-release` VM (`tests/fixtures/machine/palette.json`), and refuses each with a field or entry its schema lacks. |

### VM suites

| Check | What it proves |
|---|---|
| `smoke` | Binary installs, `vogix theme status`/`theme list`, home-manager activation sets up the state directory and symlinks; `~/.profile` and `~/.bash_profile` do not name the vogix binary and a login shell (`su - vogix -c true`) leaves `current-theme` untouched (its inode, which every refresh replaces); `/etc/vogix/machine.json` names the owner and the drop zone, which belongs to the owner, `vogix-machine.service` is active, and nothing is published; `vogix-theme-restore.service` is a oneshot without `RemainAfterExit` wanted by `graphical-session.target`, and starting it refreshes the theme, logs `Applied:` to the user journal and leaves it inactive with `Result=success`, having published the owner's palette (`vogix`, mode 644) |
| `architecture` | `~/.config` symlinks point at vogix-managed themed configs; runtime dirs are created |
| `theme-switching` | `vogix theme set -t <name>` switches themes and rewrites app configs (alacritty, btop) with the right colors |
| `scheme-switching` | All 4 schemes (vogix16, base16, base24, ansi16) work; palette format validation |
| `navigation` | `vogix theme set -v darker/lighter/dark/light`; catppuccin multi-variant navigation |
| `cli` | Subcommand parsing, CLI flags, error handling, `--version` |
| `state` | The state file is created and persists changes |
| `session` | Session save/restore behavior |
| `runtime-size` | Runtime footprint / generated-config size bounds |
| `stress` | Rapid theme/variant switching |
| `templates` | Template architecture; templates bundled in the Nix package; for a real theme of each scheme (vogix16 in both polarities, base16, base24, ansi16) the `theme.json` `vogix theme set` renders into the cache is byte-identical to the one home-manager built into the theme package |
| `input-engine` | The evdev-grab → uinput re-emit / mock-compositor dispatch engine (below) |
| `desktop-hyprland` | The desktop shell in a real Hyprland 0.56 session (greetd, virtio-gpu) with PipeWire, NetworkManager and the input engine, with every gate on the Lua config provider and the six whose command path differs (the workspace click, both LANG gates and the three mode-border gates) also on hyprlang: the session's start running `vogix-theme-restore` once, after `graphical-session.target`, and succeeding; a workspace click, a tray icon's menu, notification and panel placement beside the bars, the focus brackets, the PRIVACY cell during a screencast, the LANG cell across a layout switch and a runtime layout change, the taps across a PipeWire restart, sampling stopped under the session lock, and the restart NetworkManager's arrival triggers, deferred while locked. The input engine paints the mode's border colour, read back from the compositor: at session start, when the engine starts while the compositor has not answered yet, and after a config reload. A −6 dBFS tone reads −6 dB on the output VU (at full and half sink volume, on a null sink whose monitor carries its volume and on one whose monitor does not) and, through a `pw-loopback` virtual source, on the MIC VU. A third node boots from a LUKS root, and the root gauge names the dm-N device /proc/diskstats counts it under |
| `openrgb-readiness` | `openrgb.service` as `vogix.openrgb` runs it, against the real OpenRGB server with Debug devices: it runs vogix's OpenRGB build as `Type=notify`/`NotifyAccess=main`, reaches active through `READY=1`, and is listening the moment a restart returns; the declared devices are the controllers it serves over the SDK, at protocol 6; `vogix.openrgb.settings` merges into `OpenRGB.json` recursively, keeping keys it does not set, and replaces a file that is not one JSON object. Stock nixpkgs OpenRGB under the same unit serves the SDK but never becomes active, and its start fails. At evaluation: the unit wiring, `qmkDevices` rendered as `QMKOpenRGBDevices`, and the readiness assertion rejecting a package without `passthru.vogixReadiness`. |
| `machine-release` | The machine surfaces as the NixOS module declares them, fed by the declared owner's real CLI (a second vogix user, the default owner by name, configured with desert, themes elsewhere): `vogix-machine.service` owns the kernel's VT palette and a command device (a recorder re-run on hidraw `0627:0001`), `vogix-openrgb.service` owns the real OpenRGB server's Debug DRAM and keyboard, DDP and Govee devices. With nothing published, and the owner's tty1 autologin shell having run its login profile, both owners wait, nothing is published and the VT keeps `console.colors`; `vogix theme set` publishes after the state commit and every surface follows, checked on sources vogix does not compute: `/sys/module/vt/parameters/default_{red,grn,blu}` against the theme package's `console/palette` (no VT switch), OpenRGB's own DDP datagrams and Govee commands, the recorder; every selected controller is confirmed at protocol 6, and `STATUS=`, both status files and `vogix machine status` agree. A theme change and five rapid ones land as the last; identical bytes are not republished; the second user, with no state yet, applies desert, their configured theme, on their first refresh; another user's apply does not reach the machine; a hot-added QEMU `usb-kbd` re-runs the command device; SIGHUP and `vogix-machine-resume.service` make both owners re-apply; `vogix machine inspect` lists the controllers and captures their raw payloads at protocols 6 and 5 (`$out/capture`, the source of `tests/fixtures/openrgb`; the published palette is `$out/published`, the source of `tests/fixtures/machine`); a palette the drop zone's owner did not write is rejected by both owners and `vogix machine status` exits 1 until the owner's refresh replaces it; a restarted openrgb stops `vogix-openrgb` cleanly and a new one re-confirms, a killed one makes it exit 75 and both come back, and the server never refuses; a switch to `maxProtocol` 5 restarts both owners, and `vogix-openrgb` speaks protocol 5; after a reboot the published palette is on the VT before `systemd-user-sessions`, so before any login, the devices follow it again, and the absent Keychron is warned about once, after OpenRGB's detection completes. |

The **input-engine** suite exercises, among others:

- **Plain-key re-emit** - an unbound key is re-emitted on the engine's virtual device (typing works, compositor-agnostic)
- **Super→Ctrl remap** - `Super+C/V` emit `Ctrl+C/V` at evdev; Super never leaks; numbers/excluded keys are not remapped (context-aware for terminal vs GUI)
- **CapsLock tap/hold** - caps-hold + bound key dispatches the WM command (bound key swallowed); caps-tap enters a sticky mode and exits cleanly
- **Sub-mode routing** - caps-hold → move/resize sub-modes, move↔resize switch, release returns to the app with no stuck mode
- **Esc safety-net** - Esc exits a catchall mode back to the app (typing resumes)
- **Single-instance guard** - a 2nd engine refuses with the lock message and never double-grabs; the 1st engine stays intact
- **Lock LEDs** - `input-locks.json` follows the grabbed keyboard's CapsLock/NumLock LEDs

## Live checks on a desktop session

Two properties of the desktop shell depend on real hardware and a real
session, so no check can settle them: its CPU cost, and the reference level
of its VU meters on a real output device (the VM suite checks the meters on a
virtual sink and a virtual microphone). Run these on the machine after
switching to the build under test. Steps marked *changes the session* hide
bars, lock the screen or change the volume; each says how to undo it.

### Desktop CPU budget

The shell's budget: **at most 0.5% of one core while nothing plays, and at
most 3% with music playing** and every meter running. With every bar hidden,
or the session locked, nothing should be sampled at all.

1. Confirm the shell runs from the build under test, without detailed logs:

   ```bash
   vogix desktop status
   systemctl --user show -p ExecStart vogix-desktop | grep -c -- --no-detailed-logs   # prints 1
   ```

2. Save the measuring script. It reports the unit's whole CPU (quickshell and
   every process it started, from the unit's cgroup), quickshell's own share,
   and quickshell's context switches per second, over a window (60 s by
   default):

   ```bash
   cat > /tmp/vogix-cpu.sh <<'EOF'
   #!/usr/bin/env bash
   set -eu
   secs=${1:-60}
   unit=vogix-desktop.service
   pid=$(systemctl --user show -p MainPID --value "$unit")
   [ "$pid" -gt 0 ] || { echo "$unit is not running"; exit 1; }
   hz=$(getconf CLK_TCK)
   cg() {
     v=$(systemctl --user show -p CPUUsageNSec --value "$unit")
     case $v in '' | *[!0-9]*) echo -1 ;; *) echo "$v" ;; esac
   }
   ticks() { awk '{ print $14 + $15 }' "/proc/$pid/stat"; }
   switches() { cat /proc/"$pid"/task/*/status | awk '/ctxt_switches/ { s += $2 } END { print s }'; }
   c0=$(cg); t0=$(ticks); w0=$(switches)
   sleep "$secs"
   c1=$(cg); t1=$(ticks); w1=$(switches)
   awk -v c0="$c0" -v c1="$c1" -v t="$((t1 - t0))" -v w="$((w1 - w0))" -v s="$secs" -v hz="$hz" 'BEGIN {
     if (c0 < 0 || c1 < 0) print "unit, all processes:    n/a (CPU accounting is off for user units)"
     else printf "unit, all processes:    %.2f%% of one core\n", (c1 - c0) / (s * 1e7)
     printf "quickshell alone:       %.2f%% of one core\n", t / (s * hz) * 100
     printf "quickshell wakeups:     %.1f context switches/s\n", w / s
   }'
   EOF
   chmod +x /tmp/vogix-cpu.sh
   ```

   The first line is the budget figure. When it reads `n/a`, CPU accounting
   is off for user units, and the second line is the figure to use.

3. **Silent.** Close or stop every player (a paused stream an application
   keeps open counts as playing), then check that nothing plays and the audio
   taps are idle:

   ```bash
   pw-dump | grep -c '"media.class": "Stream/Output/Audio"'   # prints 0
   vogix desktop meters     # spectrum:idle scope:idle vu-out:idle vu-mic:on stats:…
   pgrep -x cava; pgrep -x pw-record                           # print nothing
   /tmp/vogix-cpu.sh
   ```

   Pass: the unit line is at most **0.50%**.

4. **Music.** Play something, then:

   ```bash
   vogix desktop meters     # spectrum:running scope:running vu-out:on vu-mic:on stats:…
   /tmp/vogix-cpu.sh
   ```

   Pass: the unit line is at most **3.00%**.

5. **Hidden** (*changes the session*: hides every bar). Stop the music, then:

   ```bash
   vogix desktop bar hide
   vogix desktop meters     # spectrum:off scope:off vu-out:off vu-mic:off stats:none
   /tmp/vogix-cpu.sh
   vogix desktop bar show   # undo
   ```

   Pass: `meters` reads as above, and the unit line is well below the silent
   figure, with far fewer wakeups than in step 3. What still runs while
   hidden is listed in [the desktop shell's docs](docs/desktop.md#what-runs-when).

6. **Locked** (*changes the session*: locks the screen). The measurement runs
   in the background while the lock is up; stay locked for at least 70
   seconds, then unlock and read the result:

   ```bash
   (sleep 5; vogix desktop meters; /tmp/vogix-cpu.sh) > /tmp/vogix-cpu-locked.txt 2>&1 &
   vogix desktop lock
   # …unlock after 70 s or more, then:
   cat /tmp/vogix-cpu-locked.txt
   ```

   Pass: the same as hidden.

7. If a budget fails, find the busy thread: `top -H -p "$(systemctl --user
   show -p MainPID --value vogix-desktop)"`.

8. The log stays small: note the size of quickshell's log, and again ten
   minutes later. It grows only by the shell's warnings, not by hundreds of
   records a second:

   ```bash
   ls -l "$XDG_RUNTIME_DIR"/quickshell/by-id/*/log.qslog
   ```

Record the four figures (silent, music, hidden, locked) with the change that
is being measured.

### VU meter calibration

The VU meters convert quickshell's PipeWire peaks to dBFS
(`desktop/Services/Peaks.qml`), and `vogix desktop vu` prints what they show.
quickshell reports a cube-rooted peak, and for a sink without a hardware
route (no `card.profile.device` property, or a pro-audio profile) it divides
that peak by the sink's volume. The division is right for a sink whose monitor
carries its volume (`monitor.channel-volumes = true`). By default a monitor
carries the signal before the volume, and there `Peaks.qml` multiplies the
division back out (`desktop/Services/lib/vu.js`). On a sink without a hardware
route the meter therefore shows the level applications send, before the
sink's volume. On a sink with one, quickshell leaves the peak alone, and what
the meter shows depends on where the device applies its volume.

**Automated.** `checks.desktop-hyprland` plays a 1 kHz tone peaking at
−6.00 dBFS and requires the meters to read −6 dB ± 0.5 dB for 1.5 s:

- the output meters, with the tone on each of two null sinks at 100% volume
  and again at 50%: the VM's, whose monitor carries its volume, and one made
  with PipeWire's default, whose monitor does not;
- the MIC meters, with the tone looped by `pw-loopback` into a virtual source
  that is the default input.

The tolerance follows from how the meter is defined; it is not fitted to
measurements. The level is published in steps of 1/40 of the `[floorDb, 0]`
window (`Ballistics.steps`), rounded to the nearest step, so a reading is at
most half a step from the level: 0.5 dB with the default −40 dB window. The
attack is instant, and the release and the peak cap act only when the input
falls, so a steady tone adds no ballistic error. The two 50% readings show
that on a sink without a hardware route the meter reads the level
applications send, whichever signal the sink's monitor carries.

**By hand.** The VM has no real output device. quickshell takes a device
sink's volume from its hardware route and leaves the peak alone, so there
the reference depends on where the device applies its volume. The steps
below measure it on the machine's own output (the rail's VU cell, and
`vu-out` wherever it is placed).

1. **Protect your ears** (*changes the session*: the volume goes to 100%).
   Turn the speakers or amplifier down, or unplug the headphones: a −6 dBFS
   tone at full volume is loud. The meter reads the digital signal, so it
   works with nothing listening.

2. Make a 60-second, 1 kHz stereo tone peaking at exactly half of full scale
   (−6.02 dBFS), and confirm its peak:

   ```bash
   nix shell nixpkgs#sox -c sox -n -r 48000 -c 2 -b 16 /tmp/tone-6dbfs.wav synth 60 sine 1000 vol 0.5
   nix shell nixpkgs#sox -c sox /tmp/tone-6dbfs.wav -n stat 2>&1 | grep 'Maximum amplitude'   # 0.500
   ```

3. Note the current volume (to restore it), and whether the sink has a
   hardware route (a `card.profile.device` property: quickshell then takes
   the volume from the device and leaves the peak alone):

   ```bash
   wpctl get-volume @DEFAULT_AUDIO_SINK@
   wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -E 'node.name|card.profile.device'
   ```

4. At 100% volume, play the tone and read the meters while it plays:

   ```bash
   wpctl set-volume @DEFAULT_AUDIO_SINK@ 1.0
   pw-play /tmp/tone-6dbfs.wav &
   sleep 3; vogix desktop vu       # {"out":[-6,-6],...}
   ```

   Pass: both output columns read **−6 dB**; the VU cell (the left rail in
   the default layout) shows the same. The tone is 0.02 dB under −6 dBFS,
   well inside half a step.

5. At 50% volume (in wpctl's cubic scale, −18 dB), with the tone still
   playing:

   ```bash
   wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.5
   ```

   Read `vogix desktop vu` again and record which case applies:

   - **−6 dB again**: the meter shows the level applications send, whatever
     the sink's volume.
   - **about −24 dB**: the meter follows the sink's volume (it reads what the
     device receives).
   - **0 dB, or pegged at the top**: the sink's volume was divided out
     although the monitor never carried it. This one is a defect:
     `desktop/Services/lib/vu.js` misjudges this kind of sink. Record its
     properties (`wpctl inspect @DEFAULT_AUDIO_SINK@`).

6. Stop the tone and restore the volume noted in step 3:

   ```bash
   pkill -f 'pw-play /tmp/tone-6dbfs.wav'
   wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.40   # the value from step 3
   ```

Record the sink (its `node.name`, and whether it has `card.profile.device`)
with both readings.

## Test Architecture

### NixOS Test Framework

The VM suites use the NixOS testing framework, which:
- Spins up a lightweight QEMU VM
- Runs commands in the VM
- Asserts expected outcomes
- Tears down the VM automatically

### Test Configuration

**Test VM**: `nix/vm/test-vm.nix`
- Minimal NixOS system
- Terminal-only (no GUI)
- Pre-configured test user
- All vogix16 features enabled

**Test Scripts**: `nix/vm/tests/` (one file per VM suite, plus `lib.nix` shared helpers)

**Home Config**: `nix/vm/home.nix`
- User configuration for testing
- Themes installed
- Apps configured
- Daemon enabled

**Desktop shell checks**: `flake.nix` (`desktop-options`, `desktop-runtime`,
`desktop-qmllint`, `desktop-backgrounds`), `nix/checks/desktop-smoke.nix`
with its probes `nix/checks/desktop-geometry-probe.qml` (widget size and
fit, on the data `nix/checks/desktop-geometry-feed.nix` feeds) and
`nix/checks/desktop-state-probe.qml` (window title, mode label, card
texture), `nix/checks/desktop-taps.nix`, `nix/checks/pipewire-daemon.nix`
(the hardware-less PipeWire both run), `tests/desktop/`
(`desktop-logic`: one `tst_*.qml` per unit, `probes.sh` for the probe
scripts) and the VM suite `nix/vm/tests/desktop-hyprland.nix`.

## Manual Testing

To explore the test environment by hand:

```bash
# Launch the test VM
nix run .#vogix-vm

# Inside VM, run commands manually:
vogix theme status
vogix theme list
vogix theme list -s base16
vogix theme set -s base16 -t catppuccin -v mocha
vogix theme set -v darker
vogix theme set -v lighter
vogix theme set -v dark

# Check paths
ls -la ~/.local/share/vogix/themes/
ls -la ~/.local/state/vogix/
cat ~/.local/state/vogix/config.toml
```

## Continuous Integration

`.github/workflows/ci-and-release.yml` runs on every pull request and push to
`master`: `devenv test` first, then `nix flake check --print-build-logs` on a
runner with KVM enabled for the VM suites.

## Test Development

### Adding New Tests

Create a new test file in `nix/vm/tests/` or add test cases to existing files:

```python
print("\n=== Test N: Your Test Name ===")
output = machine.succeed("su - vogix -c 'your command'")
assert "expected output" in output
print("✓ Your test passed")
```

A test of the desktop shell's pure logic goes in `tests/desktop/` as a
`tst_*.qml` Qt Quick Test case; `desktop-logic` runs every one.

### Test Helpers

- `machine.succeed(cmd)` - Run command, expect exit code 0
- `machine.fail(cmd)` - Run command, expect non-zero exit
- `machine.wait_for_unit(unit)` - Wait for systemd unit
- `machine.wait_for_file(path)` - Wait for file to exist
- `time.sleep(seconds)` - Wait for async operations

### Debugging Failed Tests

```bash
# Run a check with its log
nix build .#checks.x86_64-linux.smoke --print-build-logs

# Access the test VM interactively
nix run .#vogix-vm
```

## Performance

- **Test duration**: ~30-60 seconds per VM suite; the build-sandbox checks
  take seconds to a few minutes
- **VM RAM**: 2GB
- **VM CPUs**: 2 cores
- **Storage**: Ephemeral (no persistence between runs)

## Coverage

The automated tests cover:

✅ All CLI commands, and the CLI reference's examples
✅ Configuration management
✅ State persistence
✅ Theme and variant switching
✅ Variant navigation (darker/lighter)
✅ Multi-scheme support
✅ **Application config generation** (alacritty, btop)
✅ **Config updates on theme/variant changes**
✅ **Hex color validation in generated configs**
✅ Symlink architecture verification
✅ Template bundling
✅ Systemd integration
✅ Error cases
✅ Package installation
✅ The input engine end to end
✅ The desktop shell: its configuration contract, runtime dependencies, QML
   lint, pure logic, startup and verbs, audio taps against PipeWire, and its
   behavior under Hyprland

**Checked by hand** (see [Live checks](#live-checks-on-a-desktop-session)):
- The desktop shell's CPU cost, and its VU meters' reference level on a real
  output device
- Colors and layout as they look on a real display

## Troubleshooting

### Test fails with "vogix: command not found"

Check package installation in `test-vm.nix`:
```nix
vogix.enable = true;
```

### Test fails with "theme not found"

Check themes are installed in `home.nix`:
```nix
programs.vogix = {
  enable = true;
  # themes are discovered from vogix16-themes input
};
```

### Test VM won't start

```bash
# Check VM build
nix build .#nixosConfigurations.vogix-test-vm.config.system.build.toplevel

# Check for errors
nix flake check --print-build-logs
```

### Tests timeout

Increase timeout in the test file:
```python
machine.wait_for_unit("multi-user.target", timeout=120)
```

## Resources

- [NixOS Testing](https://nixos.org/manual/nixos/stable/#sec-nixos-tests)
- [VM Testing Examples](https://github.com/NixOS/nixpkgs/tree/master/nixos/tests)
- [Vogix Docs](docs/)
