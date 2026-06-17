# Vogix CLI Tool

The Vogix CLI is the primary user interface for managing themes in the Vogix system.

## Commands

### Theme Selection

```bash
# Set scheme, theme, and variant
vogix theme set -s base16 -t catppuccin -v mocha

# Set just the theme (keeps current scheme)
vogix theme set -t dracula

# Set just the variant
vogix theme set -v dark
```

### Variant Navigation

Navigate through variants by polarity (light-to-dark ordering):

```bash
# Move to the next darker variant
vogix theme set -v darker

# Move to the next lighter variant
vogix theme set -v lighter

# Jump to the theme's default dark variant
vogix theme set -v dark

# Jump to the theme's default light variant
vogix theme set -v light
```

**Example with catppuccin** (variants ordered: latte → frappe → macchiato → mocha):
- From `mocha`: `-v lighter` → `macchiato`
- From `latte`: `-v darker` → `frappe`
- From any: `-v dark` → `mocha` (default dark)
- From any: `-v light` → `latte` (default light)

**Single-variant themes** (like dracula): polarity requests resolve to the only variant —
`-v dark` and `-v light` both select it. Step navigation does *not*: `-v darker` / `-v lighter`
go through `ThemeInfo::navigate`, which errors at the boundary ("Already at darkest/lightest
variant") since there is no adjacent variant to step to.

### Refresh

Reapply the current theme (re-render templates and trigger reloads) without changing the selection:

```bash
vogix theme refresh
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
#
# Pass --variants to append per-theme variant lists:
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

Toggle and tune the monochromatic screen shader (see [Shader](shader.md)):

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

Drive the ontology-driven input/keybinding engine (see [Input Engine](input.md)):

```bash
vogix input check            # validate the input schema's mode graph + engine invariants
vogix input run              # run the engine (grab evdev, drive modes, dispatch to Hyprland)
vogix input doctor           # read-only diagnostics for a running engine
vogix input doctor --watch   # repaint diagnostics continuously
vogix input keys             # show the resolved schema's keybindings (via walker/notify-send)
vogix input keys --print     # print the help text to stdout instead

# check / run / keys accept --config <path> to override ~/.local/state/vogix/input.json
```

### Session

Save and restore desktop sessions (window layouts):

```bash
vogix session save [name]            # save the current session (default name: "last")
vogix session restore [name]         # restore a named session
vogix session restore --json <path>  # restore from a JSON file instead of a named session
vogix session restore --dry-run      # validate and print the session without launching apps
vogix session list                   # list saved sessions
vogix session undo                   # undo the last window change (restore from autosave stack)
```

### Modes

Switch the active desktop mode, and inspect submap-mode telemetry captured by the daemon:

```bash
vogix mode <target>          # switch desktop mode (normal, focus, gaming, presentation, ...)
vogix modes recent           # show the most recent transitions from modes.log (-n to set count)
vogix modes stats            # per-mode dwell-time histogram across the whole log
vogix modes confusion        # re-entries within a short window (-t <ms> threshold)
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
