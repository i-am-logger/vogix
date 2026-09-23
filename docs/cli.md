# Vogix CLI Tool

The `vogix` command switches themes, runs the input engine, and drives the
desktop shell. Every `vogix …` example on this page parses with the CLI it
describes, and every command has at least one example: a unit test
(`src/cli.rs`) checks both, so this page cannot fall behind the binary.

## Commands

### Theme Selection

```bash
# Set scheme, theme, and variant
vogix theme set -s base16 -t catppuccin -v mocha

# Set just the theme (keeps current scheme; the variant matches your current illumination)
vogix theme set -t dracula

# Set just the variant, by exact name
vogix theme set -v mocha
```

### Variant Navigation

Variants form an ordered **luminance ramp** (lightest → darkest). `light`/`lighter`
move **one step toward the lightest** variant; `dark`/`darker` move **one step toward
the darkest**. `-v light` is exactly `-v lighter`, and `-v dark` is exactly `-v darker`
— they step along the ramp, they do not jump to a polarity. (A theme may have several
variants of the same polarity, so stepping is the only way to reach the ones in between.)

```bash
# Step one variant lighter (toward the lightest)
vogix theme set -v lighter
vogix theme set -v light     # identical to -v lighter

# Step one variant darker (toward the darkest)
vogix theme set -v darker
vogix theme set -v dark      # identical to -v darker
```

**Example with catppuccin** (ordered: latte → frappe → macchiato → mocha):
- From `mocha`: `-v lighter` (or `-v light`) → `macchiato` → `frappe` → `latte`
- From `latte`: `-v lighter` → error (`Already at lightest variant`)

**Single-variant themes** (like dracula): there is nowhere to step, so *all* of
`-v light` / `-v dark` / `-v lighter` / `-v darker` resolve to the only variant.

**On a theme switch** (`-t <theme>`):
- With **no `-v`**, the new theme's variant is chosen to **match your current illumination**
  (closest luminance rank), so brightness carries across themes — for the usual two-variant
  theme this keeps dark↔dark / light↔light.
- With an explicit **`-v dark` / `-v light`**, you land on the new theme's **darkest** /
  **lightest** variant (there is no current position in the new theme to step from).

### Refresh

Reapply the current theme (re-render templates and trigger reloads) without changing the selection:

```bash
vogix theme refresh
vogix theme refresh -q       # errors only
```

### Undo and Redo

Step back and forth through the theme history:

```bash
vogix theme undo             # restore the selection before the last change
vogix theme redo             # re-apply the change undo stepped back from
```

### Listing

```bash
# List all schemes with theme counts
vogix theme list

# Output:
# vogix16  (19 themes)
# base16   (298 themes)
# base24   (178 themes)
# ansi16   (452 themes)

# List themes in a specific scheme
vogix theme list -s base16

# Output (default: bare theme names):
# catppuccin
# dracula
# gruvbox
# nord
# ...

# Append per-theme variant lists
vogix theme list -s base16 --variants
#   catppuccin [latte(light), frappe(dark), macchiato(dark), mocha(dark)]
```

### Status

```bash
vogix theme status

# Output:
# scheme:  base16 (16 slots)
# theme:   catppuccin
# variant: mocha
# mode:    normal
# shader:  off
# applied: 2026-06-16T12:34:56Z   # only when a last-applied timestamp exists
```

### Shell Completions

```bash
vogix completions bash > ~/.local/share/bash-completion/completions/vogix
vogix completions zsh > ~/.local/share/zsh/site-functions/_vogix
vogix completions fish > ~/.config/fish/completions/vogix.fish
vogix completions pwsh > vogix.ps1
vogix completions elvish > vogix.elv
```

## Other Commands

Beyond `theme`, the CLI exposes several top-level subcommands.

### Shader

Toggle and tune the monochromatic screen shader (a Hyprland screen shader
derived from the current theme's palette):

```bash
vogix shader on            # apply the current theme's monochromatic tint
vogix shader off           # clear the shader
vogix shader toggle        # flip on/off
vogix shader status        # show shader state and current parameters

# Tune parameters (also accepted as flags on `shader on`):
vogix shader intensity 0.5    # blend intensity   [0.0..1.0]  (-i on `shader on`)
vogix shader brightness 1.2   # brightness mult.  [0.1..2.0]  (-b on `shader on`)
vogix shader saturation 1.0   # color saturation  [0.0..2.0]  (-s on `shader on`)

vogix shader on -i 0.5 -b 1.2 -s 1.0
```

### Input

Drive the ontology-driven input/keybinding engine (see
[the input engine](architecture.md#7-input-engine)):

```bash
vogix input check            # validate the input schema's mode graph + engine invariants
vogix input run              # run the engine (grab evdev, drive modes, dispatch to Hyprland)
vogix input doctor           # read-only diagnostics for a running engine
vogix input doctor --watch   # repaint diagnostics continuously
vogix input keys             # show the resolved schema's keybindings (via $LAUNCHER/notify-send)
vogix input keys --print     # print the help text to stdout instead

# check / run / keys accept --config <path> to override ~/.local/state/vogix/input.json
vogix input check --config ./input.json
```

### Session

Save and restore desktop sessions (window layouts):

```bash
vogix session save                   # save the current session as "last"
vogix session save work              # …or under a name
vogix session restore                # restore "last"
vogix session restore work           # restore a named session
vogix session restore --json ./session.json   # restore from a JSON file instead of a named session
vogix session restore --dry-run      # validate and print the session without launching apps
vogix session list                   # list saved sessions
vogix session undo                   # undo the last window change (restore from autosave stack)
```

### Desktop

These verbs drive the vogix desktop shell (see [the desktop shell](desktop.md)).
Keybindings, the root menu, custom cells and scripts call them rather than the
shell's own transport (`qs ipc`). Without a running shell most verbs print
`no responsive shell instance` (a few stay silent) and exit 0; `lock`,
`power lock`, `select` and `input` exit non-zero instead, because a lock that
did not lock or a picker that never opened must not pass for success.

#### The shell and its configuration

```bash
vogix desktop status                     # "shell: running" plus the bar line, or "shell: not running"
vogix desktop reload                     # re-read theme.json, desktop.json and backgrounds.json (exits 0 with no shell)
vogix desktop check                      # validate ~/.local/state/vogix/desktop.json
vogix desktop check --config ./desktop.json
vogix desktop restart                    # restart vogix-desktop.service (REFUSED while the session is locked)
```

`reload` runs on every theme switch, so it never fails one. `check` is to
desktop.json what `vogix input check` is to input.json, and home-manager runs
it on the desktop.json it builds, so a document it rejects fails that build.
It fails on:

- a `schema` other than 2
- a surface token whose slot is not one of the 16 semantic keys, whose alpha
  is outside [0,1], or whose slot the current theme does not resolve
- a bar widget missing from the shell's widget registry
  (`desktop/Bar/widgets/registry.json`), a `custom/<name>` for a cell
  `custom` does not define, or a widget the registry confines to one bar
  orientation on a bar of the other (a horizontal-only `window` on the left
  bar, a vertical-only `vu-rail` on the top one)
- a malformed custom cell
- a launcher-menu `action`/`when` or a custom `command`/`onClick` with broken
  shell quoting, or one that runs `vogix` with arguments this CLI rejects

#### Bars

```bash
vogix desktop bar status                 # top:shown bottom:shown left:hidden right:off
vogix desktop bar hide left              # park one bar: top, bottom, left, right or all (the default)
vogix desktop bar show                   # slide every bar back
vogix desktop bar toggle right
vogix desktop bar toggle                 # all bars, following the top bar's state
vogix desktop bar geometry               # one line per placed widget: DP-1 top clock 3712 26 112 48
```

A hidden bar is parked off-screen, not unmapped: it slides back in about
20 ms, and its widgets stay loaded, but nothing they sample runs while it is
away. `off` is an edge desktop.json disables. `show`, `hide` and `toggle`
print the resulting status line. `geometry` prints, for each placed widget,
its screen, bar edge and name, then `x y width height` on that screen in
logical pixels as when its bar is shown — a rectangle `grim -g` or a click can
aim at.

#### HUD state

```bash
vogix desktop meters                     # spectrum:idle scope:idle vu-out:idle vu-mic:on stats:cpu,memory,net,...
vogix desktop keyboard                   # device:vogix-input layouts:us,il active:us caps:off
vogix desktop stats                      # {"cpu":0.12,"memory":0.41,"swap":null,"gpu":null,...}
vogix desktop privacy                    # mic:off screencast:off
vogix desktop vu                         # {"out":[-6,-6],"mic":-12,"stepDb":1}
```

`meters` reports each data source the HUD can run. The spectrum and scope taps
are `off` (no visible widget), `idle` (nothing playing), `waiting` (for
PipeWire or a default sink), `running`, `retrying` (after an exit) or
`parked` (after repeated failures). The VU monitors are `off`, `idle` or `on`.
`stats` lists the running samplers (cpu, memory, net, disk, gpu, uptime, temp,
fans, mounts), or `none`.

`keyboard` is what the LANG cell shows: the keyboard it follows (the input
engine's `vogix-input` device, else Hyprland's main keyboard), that keyboard's
layouts, the active one, and CapsLock as `on`, `off` or `unknown` (no input
engine running, or no grabbed keyboard with a CapsLock LED).

`stats` prints the readings behind the stat cells as one JSON object: `cpu`,
`memory`, `swap` and `gpu` as fractions (0..1), `cpuTempC`, and `fanRpm` per
fan, each `null` (or empty) while its stat is not sampled or has no sample
yet; `mounts` (the df capacity per watched mount), `gauges` (the mounts the
mounts cell shows), `rootInMemory` (a tmpfs or ramfs root, which gets no
gauge), `gaugeDevice` (the kernel device behind each gauge) and `hasGpu`.

`privacy` is what the PRIVACY cell shows: `mic:on` while another program
captures a microphone (the shell's own audio taps never count), and
`screencast:on` while a screen capture session runs.

`vu` is what the VU cells show, in dBFS: `out` holds the output meter's left
and right columns and `mic` the microphone meter, each `null` while its
monitor is not capturing (the output one captures only while something plays).
`stepDb` is the dB one step of the meter spans, so every reading is a multiple
of it (1 dB with the default window).

#### Notifications

```bash
vogix desktop notify dismiss             # the newest popup
vogix desktop notify dismiss --all
vogix desktop notify dnd toggle          # prints "dnd: on" or "dnd: off"
vogix desktop notify dnd on              # critical and rule-exempt popups still show
vogix desktop notify dnd off
vogix desktop notify dnd status          # answered from the saved state when no shell runs
vogix desktop notify history             # the last 20, newest last, read from disk (no shell needed)
vogix desktop notify history -n 50
```

#### Lock and power

```bash
vogix desktop lock                       # engage the session lock; fails LOUDLY when it cannot
vogix desktop lock --wait-secure 4       # and fail unless the compositor reports every output covered within 4 s
vogix desktop lock status                # unlocked | locked | secure
vogix desktop power                      # toggle the power menu
vogix desktop power lock                 # the same loud lock as `desktop lock`
vogix desktop power logout               # end the Hyprland session
vogix desktop power suspend              # systemctl suspend (through sleep.target, so the lock engages first)
vogix desktop power reboot               # systemctl reboot
vogix desktop power poweroff             # systemctl poweroff
```

#### Panels, launcher and menus

```bash
vogix desktop panel audio-out            # toggle a panel; prints its name, or "closed"
vogix desktop panel                      # the open panel, or "closed"
vogix desktop panel --close
vogix desktop launcher                   # the apps mode
vogix desktop launcher --mode calc --query '2^10'
vogix desktop menu                       # the root menu desktop.json defines
vogix desktop menu --summon theme        # one entry by id: its submenu, or its action run directly
vogix desktop gallery                    # the dev gallery: every surface's tokens as swatches
vogix desktop gallery --close
```

Panels: `audio` (volume and mutes), `audio-out` and `audio-in` (the device
lists), `network`, `bluetooth`, `power` (battery and power profile),
`monitor` (brightness and displays), `tailscale`, `calendar`, `weather` and
`agents` (Claude Code usage). A panel a bar widget opens sits beside that
bar; one opened by this verb sits under the top bar's end on the focused
monitor.

Launcher modes: `apps`, `files`, `calc`, `emoji`, `ssh`, `clipboard`, `theme`
and `background`, each one switched by `programs.vogix.desktop.launcher.modes`.

#### Wallpaper, OSD and switches

```bash
vogix desktop background status          # what the wallpaper shows: "<kind> <path>", or "none"
vogix desktop background next            # the next background in the theme's set (clears an override)
vogix desktop background set ~/Pictures/dunes.jpg   # override with an image file (kept across restarts)
vogix desktop background clear           # drop the override: back to the theme's first background
vogix desktop osd volume --value 40      # flash the on-screen display: a label and a 40% gauge
vogix desktop osd mic --muted            # the muted style, no gauge
vogix desktop osd caps --message 'CAPS ON'   # free text instead of the derived label
vogix desktop nightlight toggle          # hyprsunset at desktop.nightlight.temperature
vogix desktop nightlight on
vogix desktop nightlight off
vogix desktop nightlight status
vogix desktop stay-awake toggle          # hold every idle stage open (screensaver, dim, lock, screen-off, suspend)
vogix desktop stay-awake on
vogix desktop stay-awake off
vogix desktop stay-awake status
```

#### Reminders and custom cells

```bash
vogix desktop remind add 'Stretch' --in 10m   # also 45s, 1h30m, 2d; a bare number is minutes
vogix desktop remind list
vogix desktop remind clear
vogix desktop custom status updates      # what custom/updates shows: its value, "failed: …", "pending" or "inactive"
vogix desktop custom refresh updates     # run the cell's command now, or once its bar is back on screen
```

Reminders fire through the shell's own notification server and survive a
shell restart. A custom cell is one `programs.vogix.desktop.custom.<name>`
entry; `custom` exits non-zero for a name desktop.json does not define. Its
command runs only while a bar showing the cell is on screen, so a `refresh`
while every such bar is parked answers
`queued: custom/<name> runs once its bar is on screen`, and one for a cell no
bar places answers `inactive: no bar shows custom/<name>`.

#### dmenu mode

```bash
vogix desktop select -p 'Pick one' < choices.txt   # items on stdin; the chosen line on stdout, exit 1 on cancel
vogix desktop input -p 'Name'            # free text on stdout, exit 1 on cancel
```

### Hyprland IPC

Talk to Hyprland in whichever config dialect it runs (see
[Hyprland IPC under the two config engines](hyprland-lua-ipc.md)), so a bound
command does not hardcode `hyprctl dispatch`/`keyword` legacy syntax:

```bash
vogix hypr dispatch 'movefocus, l'       # a dispatcher in the legacy action form, translated for the Lua engine
vogix hypr keyword general:gaps_out 5 general:gaps_in 6   # KEY VALUE pairs, written in one atomic request
```

### Greeter

```bash
vogix greeter sync   # copy the live theme into /var/lib/vogix/greeter (the SDDM
                     # greeter's runtime follow; wired as a themeApply hook by
                     # programs.vogix.greeter.sync)
```

### Modes

Switch the active desktop mode, and inspect submap-mode telemetry captured by the daemon:

```bash
vogix mode gaming            # switch desktop mode (normal, focus, gaming, presentation, ...)
vogix modes recent           # the most recent transitions from modes.log
vogix modes recent -n 50
vogix modes stats            # per-mode dwell-time histogram across the whole log
vogix modes confusion        # re-entries within a short window (1000 ms)
vogix modes confusion -t 500
```

### Daemon & Cache

```bash
vogix daemon                 # run the vogix daemon (session auto-save, event monitoring)
vogix cache clean            # remove stale cache entries from old template versions
```

## CLI Flags Reference

The only top-level flag is `--log-level` (global). The scheme/theme/variant flags are
arguments of `theme set` (and `-s` of `theme list`), not top-level flags.

| Flag | Long | Scope | Description |
|------|------|-------|-------------|
|      | `--log-level` | top-level (global) | Log verbosity: `error`, `warn`, `info`, `debug`, `trace` |
| `-s` | `--scheme` | `theme set`, `theme list` | Color scheme (vogix16, base16, base24, ansi16) |
| `-t` | `--theme` | `theme set` | Theme within the current/specified scheme |
| `-v` | `--variant` | `theme set` | Set or navigate variants (name, dark, light, darker, lighter) |

## Configuration

The CLI is configured via the home-manager module. The scheme/theme/variant selection
nests under `appearance`:

```nix
programs.vogix = {
  enable = true;
  appearance = {
    scheme = "vogix16";
    theme = "yoga";
    variant = "dark";
  };
};
```

### Configuration Paths

| Path | Description |
|------|-------------|
| `~/.local/state/vogix/config.toml` | User configuration manifest (generated by the home-manager module) |
| `~/.local/state/vogix/state.toml` | User state (current theme selection) |
| `~/.local/state/vogix/current-theme` | Symlink to active theme directory |
| `~/.local/share/vogix/themes/` | All available theme packages |
| `~/.local/state/vogix/input.json` | The input engine's schema (home-manager) |
| `~/.local/state/vogix/current-mode`, `input-locks.json`, `input-health.json` | Published by the running input engine |
| `~/.local/state/vogix/desktop.json` | The desktop shell's configuration (home-manager) |
| `~/.config/vogix-desktop/theme.json` | The current theme's colors for the desktop shell, through `current-theme` |
| `~/.local/state/vogix/desktop/` | The desktop shell's own state: notifications and their history, do-not-disturb, reminders, stay-awake, night light, the background override, the weather cache |

## System Integration

The CLI tool handles runtime theme management:

1. **Symlink Management**: Updates the `current-theme` symlink to switch between pre-generated theme configurations
2. **State Persistence**: Saves the current scheme, theme, and variant selection
3. **Theme Validation**: Verifies that requested scheme-theme-variant combinations exist
4. **Application Reloading**: Triggers applications to reload their configurations

Note: Pre-generated `/nix/store` themes are built by Nix at build time — for those the CLI
only manages symlinks and triggers reloads, not generation. But when `[templates]` is configured
(the home-manager module emits it unconditionally), the CLI also renders the Tera templates to
`~/.cache/vogix/{templates-hash}/{scheme}/{theme}/{variant}/` at runtime on `theme set` / `theme refresh`.
So the "does not generate configs" claim applies only to the pre-generated store themes, not to
template rendering.

## Implementation Details

The vogix CLI is implemented in Rust and provides:

- Fast, efficient theme switching via symlink updates
- Polarity-based variant navigation (darker/lighter)
- Variant ordering by a persisted per-variant `order` integer (luminance is computed at Nix
  **build** time and written into the config manifest as `order = N`; the CLI sorts by that
  integer via `ThemeInfo::variants_by_order` and never recomputes luminance)
- Command completion for all major shells
- Error handling for missing themes or applications
- Theme discovery from `~/.local/share/vogix/themes/`
- State management at `~/.local/state/vogix/`

For details on how applications are reloaded, see [Reload Mechanism](reload.md).
