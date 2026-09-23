# The Desktop Shell (Flight Deck HUD)

vogix ships a desktop shell for Hyprland: four instrument bars on every
monitor, notifications, an on-screen display, panels, a launcher and menus,
a power menu, a polkit agent, the session lock, idle handling, the
wallpaper and focus brackets. It is rendered by
[quickshell](https://quickshell.org) and themed by vogix: every color comes
from the current theme, so `vogix theme set` recolors the whole shell live.

The look is the "Flight Deck" HUD: square hairline frames whose title breaks
the top border, uppercase labels, zero-padded readouts that never reflow the
bar, and meters that change color by threshold.

This page covers what the shell shows and how to configure it. The verbs
that drive it are in the [CLI reference](cli.md#desktop).

## Enabling it

The shell needs three pieces from the vogix flake: the overlay (it brings the
pinned quickshell 0.3.1 and the shell's QML package), the NixOS module (the
lock's PAM service, the lock handler, UPower and power-profiles-daemon), and
the home-manager option.

```nix
# NixOS configuration, with home-manager as a NixOS module
{
  nixpkgs.overlays = [ vogix.overlays.default ];
  imports = [
    vogix.nixosModules.default
    home-manager.nixosModules.home-manager
  ];
  vogix.enable = true;

  home-manager.users.me = {
    imports = [ vogix.homeManagerModules.default ];
    programs.vogix = {
      enable = true;
      desktop.enable = true;
    };
  };
}
```

With `desktop.enable` the home-manager module:

- writes `~/.local/state/vogix/desktop.json` from `programs.vogix.desktop.*`
- adds `theme.json` (the colors) to every theme package, served at
  `~/.config/vogix-desktop/theme.json` through `current-theme`
- runs the shell as `vogix-desktop.service` (part of `graphical-session.target`,
  ordered after PipeWire), and `vogix-lock.service`, which locks the session
  before `lock.target` and `sleep.target`
- puts the programs the shell starts (cava, wttrbar, qalc, cliphist, …, each
  only when the surface that uses it is on) on that unit's `PATH`

The NixOS module turns on UPower and power-profiles-daemon while any user
runs the shell (both `mkDefault`; power-profiles-daemon stays off where TLP,
auto-cpufreq, TuneD or system76-power already manages power).

What the shell reads from the host: Hyprland (workspaces, the window title,
focus brackets, the keyboard layout, screencast events), PipeWire (audio
devices, meters, the spectrum and scope), NetworkManager (network state),
UPower (batteries), BlueZ (bluetooth), and the vogix input engine
(`current-mode` for the MODE cell, `input-locks.json` for CapsLock). A
missing service hides or degrades the widgets that need it; the shell keeps
running.

### How changes reach the running shell

| Change | What happens |
|---|---|
| `vogix theme set …` | the theme switch runs `vogix desktop reload`: the shell re-reads `theme.json`, `desktop.json` and the theme's `backgrounds.json` and recolors in place |
| a rebuild that changes only `desktop.json` | systemd reloads the unit (`ExecReload` is the same verb); the process keeps running |
| a rebuild that changes the shell's QML | systemd restarts the unit |
| `vogix desktop restart` | restarts the unit; refused while the session is locked, because the lock lives in the shell |

`vogix desktop check` validates a `desktop.json` against everything the shell
expects; see the [CLI reference](cli.md#the-shell-and-its-configuration).

## The bars

There is one bar per screen edge, on every monitor: `top`, `bottom`, `left`
and `right`. Each has its own settings:

```nix
programs.vogix.desktop.bars.right = {
  enable = true;
  size = 114;                       # thickness in logical px
  layout = {
    start = [ "stat-cpu" "stat-gpu" "stat-mem" ];
    center = [ ];
    end = [ "network" "bluetooth" ];
  };
};
```

- `size` is the bar's thickness: its height on `top`/`bottom`, its width on
  `left`/`right`.
- A layout has three sections, laid along the bar: `start` (left or top),
  `center` (anchored to the bar's true center, whatever the other two hold)
  and `end`.
- The horizontal bars span the whole screen edge; the side rails fit between
  them.
- `window`, `media`, `weather` and `theme` read horizontally, so they are
  rejected on `left` and `right` at build time.
- The layout options accept only the names in the shell's widget registry
  (`desktop/Bar/widgets/registry.json`) and `custom/<name>`. A name that
  still reaches the shell unresolved renders as a magenta `?name` tile, with
  a journal warning saying why, so a mistake is visible rather than silently
  dropped.
- `bars.<edge>.enable = false` turns an edge off; `vogix desktop bar status`
  reports it as `off`.

`vogix desktop bar hide|show|toggle [EDGE]` parks and returns bars at runtime.
A parked bar slides off-screen but stays loaded, so it comes back in about
20 ms; while it is away, everything its widgets sample stops (see
[What runs when](#what-runs-when)).

### The default layout

| Edge | Size | start | center | end |
|---|---|---|---|---|
| top | 96 | `workspaces` `mode` | | `kbd` `privacy` `dnd` `indicators` `theme` `clock` |
| bottom | 96 | `spectrum-left` `media` | `oscilloscope` | `uptime` `weather` `update` `tray` `spectrum-right` |
| left | 128 | `mode` | `vu-rail` `audio-out-picker` `spacer` ×3 `mic-rail` `audio-in-picker` | `menu` `power-glyph` |
| right | 114 | `stat-cpu` `stat-gpu` `stat-temp` `stat-fans` `stat-mem` `stat-mounts` `graph-disk` `graph-net` | | `batteries` `battery` `audio` `network` `tailscale-glyph` `bluetooth` |

The base font size is 21 (`desktop.font.size`); every text size and spacing
unit scales from it. Bar thickness does not: it is each bar's `size`.

## Widgets

Cells are the framed Flight Deck boxes with a title in the top border;
glyphs are single icons. Where a widget is absent on a host (no battery, no
swap, no GPU it can measure), it takes no space at all rather than showing
an empty reading.

### Navigation and session

| Name | Shows | Interaction |
|---|---|---|
| `workspaces` | Hyprland's workspaces as square blocks: the focused one filled with the accent, an urgent one framed in the urgent color. A framed WS cell on a horizontal bar, a bare column on a rail. | Click focuses the workspace, under either of Hyprland's config engines (hyprlang or Lua). |
| `mode` | The input engine's current mode (`~/.local/state/vogix/current-mode`), labelled and colored from `programs.vogix.behavior.modes.modeColors`, the table the engine colors window borders with. A MODE cell on a horizontal bar, a one-letter box framed in the mode's color on a rail. | |
| `window` | The focused window's title, muted and elided. Horizontal bars only. | |
| `menu` | A menu glyph. | Click opens the root menu. |
| `power-glyph` | A power glyph. | Click opens the power menu. |
| `tray` | StatusNotifier icons. | Left click activates, middle click secondary-activates, right click opens the item's menu (it opens away from the bar's screen edge), the wheel scrolls. |
| `spacer` | Nothing: a fixed gap along the bar. | |

### Time, system and theme

| Name | Shows | Interaction and options |
|---|---|---|
| `clock` | The CLOCK cell: HH:mm:ss in bold; HH over mm (minute precision) on a rail. | Click opens the calendar panel. |
| `uptime` | The UP cell: how long the machine has been up (`2D4H`, `6H56M`, `12M`), read once a minute. | |
| `update` | A glyph, only while the booted system differs from the current one (a rebuild waits for a reboot). Checked every 5 minutes from `/run/booted-system` and `/run/current-system`. | Click shows both generations as a notification. |
| `weather` | The current weather from wttrbar, refreshed every 30 minutes and cached across restarts. Horizontal bars only. | Click opens the forecast panel. `desktop.weather.enable`, `desktop.weather.location` (empty uses wttr.in's geolocation). |
| `theme` | The THEME cell: `theme/variant` between two steppers. Horizontal bars only. | ◂ runs `vogix theme set -v darker`, ▸ runs `-v lighter`; the whole shell recolors. |
| `indicators` | Glyphs that appear only while active: night light, stay-awake, and the number of pending reminders. | Clicking night light or stay-awake turns it off. |
| `dnd` | A glyph while do-not-disturb is on. | Click turns do-not-disturb off. |

### Status glyphs

| Name | Shows | Interaction |
|---|---|---|
| `audio` | The default output's volume icon and percent. | Click opens the audio panel, middle click mutes, the wheel changes volume by 5%. |
| `mic` | The default input's mute state. | Click toggles mute. |
| `network` | Ethernet or Wi-Fi when connected, a muted glyph when not, and a warning-colored network-off glyph when the shell has no NetworkManager connection (see below). | Click opens the network panel. |
| `bluetooth` | Adapter state, accent-colored while a device is connected; absent without an adapter. | Click opens the bluetooth panel. |
| `battery` | Charge icon and percent; urgent at 15% or less on battery; absent without a battery. | Click opens the power panel. |
| `batteries` | The BAT cell: one row per battery UPower knows, internal and peripheral (model and percent); absent when there is none. | Click opens the power panel. |
| `tailscale-glyph` | The tailnet link: success-colored while connected, warning while connecting or offline, muted while stopped, logged out or without its daemon; absent without the tailscale CLI. | Click opens the tailnet panel. |
| `privacy` | See [Privacy indicators](#privacy-indicators). | |

quickshell connects to NetworkManager once per process. When the shell
starts before NetworkManager is on the bus, the network glyph shows the
warning state, and the shell waits for NetworkManager's bus name (an event,
not a poll). When the shell runs under systemd it then exits with status 75,
which `vogix-desktop.service` answers with a fresh start that connects. While
the session is locked, that restart waits for the unlock.

### Stat cells

A stat cell leads with a segmented gauge, then a zero-padded number (`042%`),
and colors the number by its thresholds: below `warn` in the normal text
color, from `warn` in the warning color, from `danger` in the danger color.
The gauge segments are colored by position in the same way.

| Name | Shows | Thresholds | Sampled |
|---|---|---|---|
| `stat-cpu` | CPU load, with its history trace inside the cell | `meters.thresholds.cpu` (50/90 %) | every `meters.sampleMs` (100 ms); trace at 1 Hz |
| `stat-gpu` | GPU busy, with its trace (see [GPU](#gpu)) | `meters.thresholds.gpu` (60/90 %) | 1 Hz, published as the mean of the last 5 samples |
| `stat-temp` | CPU temperature on a 0–100 °C gauge | `meters.thresholds.cpuTemp` (60/85 °C) | every 3 s |
| `stat-fans` | One cell per fan (see [Fans](#fans)) | `meters.thresholds.fan` (3000/4500 RPM) | every 3 s |
| `stat-mem` | Memory in use, with its trace | `meters.thresholds.memory` (60/90 %) | 1 Hz |
| `stat-swap` | Swap in use; absent without swap | `meters.thresholds.swap` (20/80 %) | 1 Hz |
| `stat-disk` | The root filesystem's usage | fixed 80/95 % | every 30 s |
| `stat-mounts` | One cell per `meters.mounts` entry (see below) | filesystems 80/95 %, swap `meters.thresholds.swap` | every 30 s |
| `stat-net` | NET: upload ▲ and download ▼ rates (B, K, M per second) | | every `meters.sampleMs` |
| `cpu`, `memory` | Plain glyph-and-percent gauges, urgent from 90% | | as above |

`stat-temp` reads the first hwmon sensor it finds by driver: `k10temp`,
`zenpower`, `coretemp`, `cpu_thermal`, `acpitz`. A board with none shows no
TEMP cell.

`stat-mounts` takes its gauges from `meters.mounts`, in order: absolute mount
points plus the literal `swap`. The default is
`[ "/" "/nix" "/persist" "/boot" "swap" "/tmp" ]`. Each cell is titled with
the path's last segment (`/` is ROOT, four characters on a rail), and carries
the filesystem's own I/O rate (⇅) where a block device backs it, LUKS and LVM
volumes included. A path the host does not mount has no cell, and `/` has
none while the root is in memory (tmpfs or ramfs, as under impermanence),
since the storage is then on the other mounts.

### Graphs

History graphs keep `meters.history` samples (64 by default), one a second.

| Name | Shows |
|---|---|
| `graph-cpu`, `graph-mem` | CPU and memory history, untitled, meant to sit beside their stat cells |
| `graph-gpu` | The GPU busy history; absent without a GPU source |
| `graph-net` | NET: download solid, upload dashed, scaled to the window's own peak, with the live download rate |
| `graph-disk` | I/O: the combined throughput of the whole physical disks, scaled to the window's peak, with the live rate |

### GPU

The GPU cell and graph measure one device, chosen in this order:

1. an NVIDIA GPU, through one long-running `nvidia-smi` stream (the host's
   `nvidia-smi`, which has to match its driver);
2. amdgpu's `gpu_busy_percent`;
3. Intel i915 or xe idle residency: busy is the share of time the GPU was not
   idle, an upper bound on engine load. These are the only Intel counters
   readable without `CAP_PERFMON`.

A GPU that runtime power management may switch off, and that is not the boot
display, is never sampled: reading it every second would keep a hybrid
laptop's idle discrete GPU awake. A busy counter swings from one read to the
next, so the cell shows the mean of the last 5 one-second samples. With no
usable source the GPU cells are absent. Thresholds:
`meters.thresholds.gpu = { warn = 60; danger = 90; }`.

### Fans

`stat-fans` shows one cell per hwmon fan tachometer (`fanN_input`) that has
spun at least once this session; boards expose a tachometer per header
whether or not a fan is plugged in, and an empty header reads 0 forever. Each
cell shows RPM, and a gauge only where the chip reports the fan's maximum
(`fanN_max`). The title is the chip's label for the header, else `FAN<n>`
(cut to four characters on a rail, `F<n>` past FAN9). When the fans come from
more than one chip, each cell also names its chip. Thresholds are in RPM:
`meters.thresholds.fan = { warn = 3000; danger = 4500; }`.

A fan or pump that the kernel exposes no hwmon tachometer for (for example an
AIO cooler driven only through liquidctl) does not appear.

### Audio instruments

| Name | Shows | Interaction |
|---|---|---|
| `vu-out` | The OUT cell: the default output's level on a segmented meter with a peak cap, and its dB. | |
| `vu-mic` | The MIC cell: the same for the default input; the title turns urgent while an application is recording. | |
| `vu-rail` | The rail's VU cell: left and right output columns and the output dB. | |
| `mic-rail` | The rail's MIC column and dB; the title turns urgent while an application is recording. | |
| `audio-out-picker` | The OUT cell: the output's mute glyph and device name. | The glyph toggles mute; the rest opens the output device list. |
| `audio-in-picker` | The IN cell: the input's mute glyph and device name; the title turns urgent while an application is recording. | The glyph toggles mute; the rest opens the input device list. |
| `spectrum-mini` | The whole stereo spectrum as thin bars (for a top bar). | |
| `spectrum-left`, `spectrum-right` | One channel each, bass at the outer edge, so the pair mirrors across the bar. | |
| `spectrum-rail` | The spectrum with one band per row, for a rail. | |
| `oscilloscope` | The SCOPE cell: the output's waveform around a zero line. | |
| `media` | See [Media transport](#media-transport). | |

The VU meters read quickshell's PipeWire peak monitors and show dBFS over a
window from `meters.vu.floorDb` (−40 by default) to 0. The meters rise
instantly and fall at 3.0 full-scale per second; the peak cap holds 0.53 s,
then falls at 0.75 full-scale per second. The spectrum shares that
ballistics, so the bars and the meters fall together. The level is shown in
steps of 1/40 of the window: 1 dB with the default window.
[TESTING.md](../TESTING.md#vu-meter-calibration) has the procedure that checks
the meters' reference level against a test tone.

The spectrum is cava at 25 frames a second, `meters.spectrum.bars` bands per
stereo channel (48 by default); `meters.spectrum.enable = false` removes it.
A spectrum's size follows that band count, not the latest frame, so it
keeps its size while nothing plays.
The scope reads the output's monitor with `pw-record` at 8 kHz, one update
per 256 samples (about 31 a second). Both run only while their widget's bar
is on screen and something is playing, and restart on their own after an
unexpected exit (after 1, 2, 4, 8 and 16 s; after the sixth quick failure
they wait for PipeWire to reconnect, the default output to change or the
widget to come back). `vogix desktop meters` reports their state.

### Media transport

`media` is the MEDIA cell: previous, play/pause and next for the active MPRIS
player (the one playing, else the first that can be controlled). There is no
track text. The MEDIA title lights while something plays; a control the
player cannot perform right now is unlit and ignores clicks; the mouse wheel
over the controls skips tracks (a mouse wheel only, so a touchpad swipe does
not skip a run of tracks). The cell is absent while no player exists. It is
horizontal-only.

### Tailnet

`tailscale-glyph` (above) shows the link state; the `tailscale` TS cell shows
peers online and total and how long the current connection has lasted, as in
`4/7 12D`. A connection the shell found already up is marked `≥`, because its
true start is earlier. The state is sampled every 30 seconds with the
tailscale CLI, which offers no stable event stream. The connection record is
kept in the runtime directory, so it survives a shell restart but not a new
login.

### Custom cells

A custom cell shows the output of a command, in the stat-cell shape, on any
bar, horizontal or vertical. Define it under `desktop.custom` and place it as
`custom/<name>`:

```nix
programs.vogix.desktop = {
  custom = {
    # System generations on disk, re-counted hourly.
    gens = {
      title = "GENS";
      command = "ls -d /nix/var/nix/profiles/system-*-link | wc -l";
      interval = 3600;
      widest = "999";
    };
    # The 1-minute load average, colored by its own thresholds.
    load = {
      command = ''awk '{ s = $1 > 8 ? "danger" : $1 > 4 ? "warning" : "normal"; printf "{\"text\":\"%s\",\"state\":\"%s\"}\n", $1, s }' /proc/loadavg'';
      output = "json";
      interval = 5;
    };
    # A state file another program writes: re-read whenever it changes.
    vpn = {
      command = "cat /run/user/1000/vpn-state";
      watch = [ "/run/user/1000/vpn-state" ];
    };
  };
  # A layout list replaces the edge's default section.
  bars.top.layout.center = [ "custom/gens" "custom/load" "custom/vpn" ];
};
```

| Field | Meaning |
|---|---|
| `command` | Run with `sh -c`; its output is the value. A non-zero exit shows ERR in the danger color until a run succeeds. |
| `title` | The cell's title (uppercase; four characters on a rail). Defaults to the name. |
| `output` | `text`: the first non-empty line. `json`: one object `{ "text", "state": "normal" \| "warning" \| "danger", "meter": 0.0–1.0 }`; `state` colors the value like a threshold, `meter` adds a gauge. |
| `interval` | Seconds between runs; `null` (the default) runs only on the triggers below. |
| `watch` | Absolute paths whose change (a write, a replacement, a creation) re-runs the command. |
| `stream` | The command keeps running and every line it prints (every JSON object, with `json`) replaces the value. |
| `onClick` | A command run on click; the cell re-runs its own command after it. Without one, a click re-runs the command. |
| `widest` | A sample of the widest value; its width is reserved so the bar never reflows. |

One command serves every bar and screen that places the cell, and it runs
only while one of those bars is live (see [What runs when](#what-runs-when)):
nothing runs while every bar carrying the cell is hidden, the session is
locked or the screens are off. While live, the command runs on its interval,
on a watched file's change, after a click, and on
`vogix desktop custom refresh <name>`; a trigger during a run queues exactly
one more run. When the cell comes back on screen it runs only if its result
is older than its interval, or a watched file changed, a refresh arrived or a
run was cut short while it was away; otherwise it waits out the rest of its
interval. A refresh while every bar carrying the cell is hidden answers
`queued: custom/<name> runs once its bar is on screen`. A `stream` command
stays up while the cell is live and starts again each time it returns.
`vogix desktop custom status <name>` prints what the cell shows. The build
rejects a placement that names no defined cell, and a cell name other than
letters, digits, `-` and `_`.

## Notifications

The shell is the session's notification server (the
`org.freedesktop.Notifications` owner, so no other notification daemon runs
beside it). Popups are 440 px Flight Deck cards in the top-right corner of
the space the bars leave free, on the focused monitor:

- a normal card has the app name in its title and a faint hairline frame; a
  critical card is titled `ALERT :: APPNAME` on a solid danger frame
- the header carries the arrival time (HH:mm:ss); the body shows up to six
  lines
- a drain bar along the bottom shows the card's remaining lifetime
- a click dismisses a card
- past `maxVisible` cards, the rest queue behind a `+N QUEUED` line; when the
  column is taller than the free space, the oldest cards leave the top first

| Option | Default | Meaning |
|---|---|---|
| `notifications.enable` | `true` | Run the notification server |
| `notifications.defaultTimeout` | `5000` | Lifetime in ms of a normal notification that sets none |
| `notifications.maxVisible` | `5` | Cards shown at once |
| `notifications.appRules.<app>` | yubikey-touch-detector: 15 s, danger accent, bypasses DND | Per-app `timeout` (ms), `accent` (a semantic slot) and `bypassDnd` |

Critical notifications never expire. Do-not-disturb (`vogix desktop notify dnd
on`, or the menu) hides everything but critical and rule-exempt popups. Live
cards are saved to disk, so a shell restart in the middle of a YubiKey prompt
brings them back with their arrival times; `vogix desktop notify history`
reads the kept history without a running shell. With
`background.scanlines = true` the cards carry the same scanline texture as the
bars.

## Privacy indicators

The `privacy` widget appears only while something records:

- a microphone glyph while an application holds a PipeWire capture stream.
  The shell's own meters and taps also capture; they are excluded by name, so
  a meter never lights its own indicator.
- a screen glyph while a screencast runs, counted from Hyprland's
  `screencast` events, so overlapping casts keep it lit until the last one
  ends. Screenshots use the same mechanism and light it for the moment they
  take. A cast already running when the shell starts goes unseen until it
  ends, because Hyprland offers no query for live sessions.

The MIC and IN titles turn urgent on the same microphone signal.

## LANG and CapsLock

`kbd` is the LANG cell: every configured layout, with the active one bright
and bold and the rest dim; a click switches to the next layout. It follows the
input engine's `vogix-input` keyboard (what applications actually receive),
else Hyprland's main keyboard, and reads that keyboard's layout list and
active layout index from `hyprctl -j devices` on every `activelayout` event,
so a layout added by a rebuild appears without a restart.

The layouts are `programs.vogix.behavior.input.kbLayout` (default `"us,il"`);
`behavior.input.kbOptions` defaults to `grp:alt_caps_toggle`, so Alt+CapsLock
cycles the layouts while CapsLock alone still toggles capitals.

CAPS sits in the same cell, bright while CapsLock is on and dim while off. The
input engine reads the lock LEDs of the keyboards it grabs and publishes them
in `~/.local/state/vogix/input-locks.json`; the cell watches that file, so it
changes the moment the LED does. CAPS is hidden while the state is unknown:
no input engine running, or no grabbed keyboard with a CapsLock LED.
`vogix desktop keyboard` prints the same state.

## Panels and popups

The glyphs and cells open panels: `audio` (volume and mutes), `audio-out`
and `audio-in` (device lists), `network` (Wi-Fi scan and connect, wired
state), `bluetooth` (adapter switch, devices), `power` (batteries, power
profile, system line), `monitor` (backlight and displays), `tailscale`
(connection time, this node, peers), `calendar`, `weather` (forecast) and
`agents` (Claude Code sessions and output tokens today, per account). A panel
opened from a widget sits beside that widget's bar, centred on the widget,
on that bar's screen; one opened with `vogix desktop panel NAME` sits under
the top bar's end on the focused monitor. Escape closes a panel.

The power panel reports only what its daemons confirm: without UPower the
battery state reads as unknown, and the profile row appears once
power-profiles-daemon has answered.

## Launcher, menus and dialogs

- **Launcher** (`vogix desktop launcher`): one overlay with the modes `apps`,
  `files` (fd), `calc` (qalc), `emoji`, `ssh` (hosts from `~/.ssh/config`),
  `clipboard` (cliphist), `theme` and `background` (pickers). Each mode is
  switched by `desktop.launcher.modes.<mode>.enable`.
- **Root menu** (`vogix desktop menu`, the `menu` glyph): the entries in
  `desktop.launcher.menu` — `{ id, icon, label, action | submenu, when }`,
  where `when` is a command that must exit 0 for the entry to show. The
  default menu holds keybindings, a 10-minute reminder, the theme picker, the
  next background, do-not-disturb, Claude usage, lock and power.
  `vogix desktop check` verifies that every entry's command parses.
- **dmenu mode**: `vogix desktop select` and `vogix desktop input` show the
  launcher as a picker for scripts; the `vogix-launcher` package wraps them in
  a walker-compatible `--dmenu` form.
- **Power menu** (`vogix desktop power`, the `power-glyph`): lock, log out,
  suspend, reboot, power off. Keyboard-first; Escape closes.
- **Polkit agent**: the shell answers polkit authentication requests with its
  own dialog (`desktop.polkit.enable`).
- **On-screen display**: volume changes on the default output flash it
  automatically; `vogix desktop osd` flashes it from scripts. It sits above
  the bottom bar for `desktop.osd.timeout` ms (1500).

## Lock and idle

The lock is the shell's own: one Wayland session lock covering every output,
authenticated through the PAM service `desktop.lock.pamService` (`vogix-lock`,
which vogix's NixOS module declares, so password, U2F and fingerprint follow
the host's PAM settings). The shell refuses to lock when that PAM service is
missing, so it can never produce a screen nobody can unlock. A wrong password
shakes the box; there is no cancel. `vogix desktop lock --wait-secure 4`
fails unless the compositor confirms every output covered, and
`vogix-lock.service` uses it so a suspend cannot run ahead of the lock.

Idle stages, in seconds of inactivity (`null` disables a stage):

| Option | Default | Stage |
|---|---|---|
| `idle.screensaver` | `null` | A full-screen drift of the theme's colors with a wandering clock |
| `idle.dim` | `300` | A dimming veil |
| `idle.lock` | `600` | The session lock |
| `idle.screenOff` | `660` | Displays off (DPMS) |
| `idle.suspend` | `null` | `systemctl suspend` |

Idle inhibitors (a playing video, for example) hold every stage, and so does
`vogix desktop stay-awake on`.

## Wallpaper, scanlines and focus brackets

The wallpaper layer shows the current theme's background set, listed in the
theme package's `backgrounds.json`: a background generated from the theme's
own palette first, the built-in `aurora` shader (the palette as its colors)
second, then any extra images or videos from
`programs.vogix.appearance.extraBackgrounds.<theme>.<variant>`. A theme switch
cross-fades to the new set. `vogix desktop background next|set|clear` changes
it at runtime (an override is kept across restarts).

| Option | Default | Meaning |
|---|---|---|
| `background.enable` | `true` | Render the wallpaper layer |
| `background.animate` | `"on-ac"` | When shaders and videos may move: `always`, `on-ac` or `never`; every setting pauses them while the dim stage is up |
| `background.scanlines` | `false` | A static CRT scanline texture over the bars and notification cards |
| `decorations.focusBrackets` | `true` | Four accent corner brackets on the focused window, drawn by a click-through overlay and hidden on fullscreen |

## Colors and sizes

Every color the shell draws is a token: a slot from the theme's 16 semantic
colors plus an alpha, `{ slot = "background"; alpha = 0.92; }` or just
`"danger"`. Tokens are grouped by surface: `bar`, `meter`, `popup`,
`notification`, `osd`, `polkit`, `lock`, `launcher`, `power`. Override any
of them:

```nix
programs.vogix.desktop.surfaces = {
  bar.background = { slot = "background"; alpha = 0.85; };
  meter.high = "danger";
};
```

A token whose slot the theme does not resolve draws in magenta, so the
mistake shows; `vogix desktop check` catches it before that.

`desktop.font.family` and `desktop.font.size` set the shell's font; every
text size and spacing unit is a multiple of `font.size`. Bar thickness is
each bar's `size`, and the floating surfaces (notification cards, panels,
launcher, OSD, lock box) have fixed widths.

`vogix desktop gallery` opens a window with every surface's tokens rendered
as swatches, and the HUD building blocks in the current theme.

## What runs when

A widget samples only while its bar is **live**: the bar is enabled and not
hidden, and the screen can be seen (the session is not locked, the
screensaver is not up, and the displays are not off). A hidden bar's widgets
stay loaded, so it returns instantly, but they hold nothing:

- **Audio.** The spectrum (cava) and scope (`pw-record`) processes, and the
  output VU monitor, run only while a live widget wants them and a playback
  stream exists; with nothing playing no tap process runs and the output can
  suspend. A stream an application keeps open while paused counts as
  playing. The microphone VU monitor runs while its widget is live. The
  meters' animation stops once every level has settled.
- **System stats.** Each sampler (cpu, memory, net, disk, gpu, uptime, temp,
  fans, mounts) runs only while a live widget reads it. When a stat stops,
  its graph history is cleared, so a returning graph never joins two periods
  of time.
- **The clock** ticks seconds only while its bar is live.
- **Custom cells** run their commands only while a bar carrying them is live
  (see [Custom cells](#custom-cells)).

Some sources do not depend on a bar:

- the tailnet state is sampled every 30 seconds
- the weather refreshes every 30 minutes, the reboot check runs every 5
  minutes, and pending reminders are checked every 15 seconds
- everything else is event-driven: Hyprland's events (workspaces, the focused
  window, keyboard layouts, screencasts), PipeWire's graph, D-Bus services,
  and file watches (the mode and lock-state files)

`vogix desktop meters` shows what is running now, for example
`spectrum:idle scope:idle vu-out:idle vu-mic:on stats:cpu,memory,net,disk,gpu,uptime,temp,fans,mounts`.
[TESTING.md](../TESTING.md#desktop-cpu-budget) has the procedure that
measures the shell's CPU cost.

The shell runs quickshell with `--no-detailed-logs`, so quickshell records
only the messages it shows (the shell's warnings and errors, which reach
`journalctl --user -u vogix-desktop`), not every internal debug record.
`desktop.detailedLogs = true` turns the detailed log back on for debugging;
it records hundreds of entries a second while the meters run.

## Diagnostics

```bash
vogix desktop status                     # is the shell running, and the bar state
vogix desktop check                      # does desktop.json match what the shell expects
vogix desktop meters                     # which taps, monitors and samplers run
vogix desktop keyboard                   # the keyboard, layouts and CapsLock the LANG cell shows
vogix desktop stats                      # the stat cells' readings, as JSON
vogix desktop privacy                    # what the PRIVACY cell shows: mic:off screencast:off
vogix desktop bar geometry               # where each placed widget sits on its screen
journalctl --user -u vogix-desktop -b    # the shell's warnings
```
