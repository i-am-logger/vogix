# Vogix
[![CI](https://github.com/i-am-logger/vogix/actions/workflows/ci-and-release.yml/badge.svg?branch=master)](https://github.com/i-am-logger/vogix/actions/workflows/ci-and-release.yml)
[![License: CC BY-NC-SA 4.0](https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-nc-sa/4.0/)
[![NixOS](https://img.shields.io/badge/NixOS-5277C3?logo=nixos&logoColor=white)](https://nixos.org)
[![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)

> A NixOS UX subsystem for unified desktop appearance and behavior.

Vogix is a declarative UX layer for NixOS that unifies desktop configuration - define your appearance (colors, fonts, transparency, animations) and behavior (keybindings, window rules, gestures) once, and vogix generates configs for all your applications.

**Currently implemented:**
- **Appearance** - runtime color theme switching across 4 schemes, no system rebuilds.
- **Behavior** - an ontology-driven keyboard input engine: modal + chorded keybindings, optional dual-role CapsLock, and selectable interaction *paradigms* (vogix / i3 / cua / emacs / vim / windows / macos / linux).
- **Desktop shell** - a Hyprland desktop shell themed by the current palette: four instrument bars (the Flight Deck HUD), notifications, lock, idle, launcher, panels, OSD and wallpaper. See [the desktop shell](docs/desktop.md).

**Vision:** Full desktop UX management - see [roadmap](#roadmap).

> [!WARNING]  
> right now this runs and working in a vm.
> ```bash
> nix run .#vogix-vm
> ```
> vogix is alpha, however it is "battlefield tested" as it is integrated to [mynixos](https://github.com/i-am-logger/mynixos) and my [system](https://github.com/i-am-logger/flake). 

## Roadmap

Vogix is evolving from a color theming tool to a full NixOS UX subsystem. See [#149](https://github.com/i-am-logger/vogix/issues/149) for the complete vision.

### Appearance (how things look)
- [x] Colors - runtime theme switching
- [ ] [Typography](https://github.com/i-am-logger/vogix/issues/141) - fonts, sizes, weights
- [ ] [Transparency](https://github.com/i-am-logger/vogix/issues/142) - opacity, blur
- [x] [Backgrounds](https://github.com/i-am-logger/vogix/issues/143) - per-theme wallpaper sets in the desktop shell (a background generated from the palette, a live shader, curated extras)
- [ ] [Window chrome](https://github.com/i-am-logger/vogix/issues/144) - borders, gaps, radius
- [ ] [Animations](https://github.com/i-am-logger/vogix/issues/151) - duration, easing
- [ ] [Cursors](https://github.com/i-am-logger/vogix/issues/146) - theme, size
- [ ] [Icons](https://github.com/i-am-logger/vogix/issues/152) - icon themes
- [x] [Notifications](https://github.com/i-am-logger/vogix/issues/150) - the desktop shell's own notification server, themed live (mako and dunst are not styled)
- [ ] [GTK/Qt](https://github.com/i-am-logger/vogix/issues/148) - toolkit theming
- [ ] [HiDPI](https://github.com/i-am-logger/vogix/issues/153) - scaling
- [ ] [Shaders](https://github.com/i-am-logger/vogix/issues/145) - *future* CRT / bloom post-processing effects (the monochromatic screen shader already ships — see Features)

### Behavior (how things act)
- [x] [Keybindings](https://github.com/i-am-logger/vogix/issues/154) - ontology-driven input engine: modes, optional dual-role CapsLock, interaction paradigms (vogix/i3/cua/emacs/vim/windows/macos/linux)
- [ ] [Window rules](https://github.com/i-am-logger/vogix/issues/155) - floating, positioning
- [ ] [Focus](https://github.com/i-am-logger/vogix/issues/156) - follow mouse, click-to-focus
- [ ] [Gestures](https://github.com/i-am-logger/vogix/issues/157) - touchpad, touchscreen

## Philosophy

**Declarative.** Define your desktop experience once in a single config. Vogix generates the app-specific configs.

**Reproducible.** Built on Nix. Same inputs = same outputs. Templates are immutable in the Nix store, rendering is deterministic.

**Compositor-aware.** Theme switching works under any compositor. The input engine re-emits keys below the compositor, so typing works everywhere; its window actions, the screen shader, session restore and the desktop shell target Hyprland, under either of its config engines (hyprlang or Lua).

## Color Schemes

Vogix supports 4 color schemes, each with its own philosophy:

| Scheme | Themes | Philosophy |
|--------|--------|------------|
| **vogix16** | 19 | Semantic design - colors convey functional meaning. See [design system](https://github.com/i-am-logger/vogix16-themes/blob/main/docs/design-system.md). |
| **base16** | ~300 | Minimal palette standard for syntax highlighting |
| **base24** | ~180 | Extended base16 with extra accents |
| **ansi16** | ~450 | Traditional ANSI terminal color mappings |

## Features

- **Multi-Scheme Support**: 4 color schemes (vogix16, ansi16, base16, base24)
  - **vogix16** (default) - Semantic design system focused on functional colors (19 native themes)
  - **ansi16** - Terminal ANSI standard (~450 themes)
  - **base16** - Minimal palette standard, widely used for UI and syntax highlighting (~300 themes)
  - **base24** - Expanded base16 palette with extra accents (~180 themes)
- **Runtime Theme Switching**: Change themes without NixOS rebuilds
- **Multi-Variant Themes**: Themes can have multiple variants (e.g., catppuccin: latte, frappe, macchiato, mocha)
- **Polarity Navigation**: Switch between lighter/darker variants with `vogix theme set -v lighter` / `vogix theme set -v darker`
- **Application-Specific Configs**: Direct integration for [supported applications](https://github.com/i-am-logger/vogix/tree/master/nix/modules/applications)
- **Monochromatic Screen Shader**: A palette-derived Hyprland screen shader ships today — it desaturates the screen to a light/dark blend of the active theme's ramp while preserving the functional colors, applied/cleared via `hyprctl` and driven by the full `vogix shader` CLI (`on`/`off`/`toggle`/`status`, with intensity/brightness params)
- **Multiple Reload Methods**: Unix signals, command, filesystem watching (`touch`), or none
- **Nix-Based Theme Generation**: All theme configurations pre-generated at build time
- **NixOS Integration**: Home Manager module with systemd service
- **Shell Completions**: Support for Bash, Zsh, Fish, and Elvish

### Input Engine

A single ontology-driven input daemon (evdev grab → uinput re-emit + compositor IPC) that owns the keyboard — no kanata, no compositor submaps — so the same keybindings work even without a running compositor.

- **Modes** *(optional)*: the engine can model a typed mode statechart (e.g. app / desktop / move / resize / console). Modes and their bindings are loaded as *data*, not hard-coded; "stuck in a mode" is unrepresentable (machine-checked against the [praxis](https://github.com/i-am-logger/pr4xis) HMI ontology — Raskin/Norman/Harel). The shipped `vogix` default is a single flat `app` mode with no submodes; the named multi-mode statechart (desktop / move / console) currently appears only in test fixtures. Paradigms that need a submode add their own (e.g. `i3` adds a `resize` submap, `vim` an `insert` submode).
- **Dual-role CapsLock** *(optional)*: an engine capability — tap = sticky (locked) mode, hold = momentary (reverts on release); a forgotten sticky mode idle-reverts. Esc is an always-available safety-net exit; the active mode is shown by the window border. It is NOT enabled by the `vogix` default (CapsLock stays CapsLock); you opt in by authoring layers.
- **Interaction paradigms**: select a whole keymap *flavour* as data and layer your own bindings on top —
  - `vogix` *(default)* - the house WM-navigation layout (Super-chorded hjkl + arrows) in a single `app` mode
  - `i3` / `vim` - modal flavours that add a submode (`resize` / `insert`)
  - `cua` - the IBM/Windows Ctrl-shortcut standard
  - `windows` / `macos` / `linux` - desktop chorded navigation (each cited to the platform's keyboard-shortcut docs)
  - `emacs` - Ctrl/Meta motion as a single passthrough `app` mode (multi-key `C-x` prefixes modelled as transient modes are *future work* — not yet in the preset)
- **Context-aware macOS-Command remap**: Super behaves like ⌘ (Super+C → Ctrl+C), retargeted to Ctrl+Shift in terminals so it can't SIGINT a running job.

### Desktop Shell

A quickshell-rendered shell for Hyprland, enabled with `programs.vogix.desktop.enable` and colored entirely by the current theme, so `vogix theme set` recolors it live. See [the desktop shell](docs/desktop.md).

- **Four bars on every monitor** (the Flight Deck HUD): workspaces, the input engine's mode, clock, keyboard layout with CapsLock, CPU/GPU/memory/temperature/fan/disk cells with history traces, network and disk I/O graphs, stereo VU meters with dB, a spectrum and an oscilloscope, media transport, audio device pickers, and status glyphs. Where each widget sits and the meters' thresholds are configuration, and custom cells show the output of your own commands.
- **Notifications**: the session's notification server; critical notifications never expire, per-app rules set timeouts, colors and do-not-disturb exemptions.
- **Lock and idle**: a PAM-backed session lock that refuses to engage without its PAM service, and staged idle (screensaver, dim, lock, screens off, suspend).
- **Launcher and menus**: apps, files, calc, emoji, ssh, clipboard, theme and background pickers, a root menu from your configuration, a power menu, and dmenu mode for scripts.
- **Panels, OSD, polkit agent, wallpaper and focus brackets**, all driven by `vogix desktop …` verbs.
- **Quiet out of sight**: the audio taps run only while something plays and their widget is on screen, and the system samplers stop while their bar is hidden, the session is locked or the screens are off (custom cells and a few slow polls keep running).

## Quick Start

### Installation (NixOS with Flakes)

Add to your `flake.nix`:

```nix
{
  inputs.vogix.url = "github:i-am-logger/vogix";

  outputs = { nixpkgs, home-manager, vogix, ... }: {
    homeConfigurations.youruser = home-manager.lib.homeManagerConfiguration {
      modules = [
        vogix.homeManagerModules.default
        {
          # Enable applications you want to theme
          programs.alacritty.enable = true;
          programs.btop.enable = true;

          # Configure vogix
          programs.vogix = {
            enable = true;
            appearance = {
              scheme = "vogix16";
              theme = "yoga";
              variant = "dark";
            };
          };
        }
      ];
    };
  };
}
```

### Usage

```bash
# Show current theme state
vogix theme status

# List all schemes with theme counts
vogix theme list

# List themes in a specific scheme
vogix theme list -s base16

# Set scheme, theme, and variant
vogix theme set -s base16 -t catppuccin -v mocha

# Navigate to a darker variant
vogix theme set -v darker

# Navigate to a lighter variant
vogix theme set -v lighter

# `dark`/`light` are aliases of `darker`/`lighter` — one step toward dark/light
vogix theme set -v dark
vogix theme set -v light

# Generate shell completions
vogix completions bash > ~/.local/share/bash-completion/completions/vogix
```

## Testing

Vogix includes automated integration tests:

```bash
# Run all tests
./test.sh

# Or use nix directly
nix flake check
```

See [TESTING.md](TESTING.md) for detailed testing documentation.

## Documentation

- [Architecture](docs/architecture.md) - System architecture and integration
- [CLI Reference](docs/cli.md) - Command-line interface guide
- [Desktop Shell](docs/desktop.md) - The Flight Deck HUD: bars, widgets, notifications, lock, launcher
- [Hyprland IPC](docs/hyprland-lua-ipc.md) - Talking to Hyprland under both config engines
- [Theming Guide](docs/theming.md) - Creating and customizing themes
- [Reload Mechanisms](docs/reload.md) - Application reload methods
- [Vogix16 Design System](https://github.com/i-am-logger/vogix16-themes/blob/main/docs/design-system.md) - Default scheme philosophy and formats

## Defaults

Vogix ships with the vogix16 scheme as the default, using the `yoga` theme in `dark` mode unless configured otherwise.

## Example Themes

Vogix supports themes from multiple sources:

- **vogix16**: Native themes from [vogix16-themes](https://github.com/i-am-logger/vogix16-themes) (yoga, forest, etc.)
- **ansi16**: Imported from [iTerm2-Color-Schemes](https://github.com/i-am-logger/iTerm2-Color-Schemes)
- **base16/base24**: Imported from [tinted-schemes](https://github.com/i-am-logger/tinted-schemes) (catppuccin, dracula, gruvbox, nord, etc.)

Create custom vogix16 themes by following the [vogix16-themes contribution guide](https://github.com/i-am-logger/vogix16-themes#contributing).


## Requirements

- NixOS (for full integration) or any Linux distribution (for standalone binary)
- Rust Edition 2024
- Hyprland (for the screen shader, the input engine's window actions, session restore, and the desktop shell)
- For the desktop shell: PipeWire, NetworkManager, UPower, BlueZ and Tailscale where you want their widgets (a missing service hides or degrades the widgets that need it)
- App reloads use Unix signals, a command, or a `touch` of the theme file — no DBus/IPC dependency

## License

Creative Commons Attribution-NonCommercial-ShareAlike (CC BY-NC-SA) 4.0 International

See [LICENSE](LICENSE) for details.

## Contributing

- [Contributing Guide](CONTRIBUTING.md) - How to contribute to Vogix
- [Development Guide](DEVELOPMENT.md) - Setting up development environment
- [Testing Guide](TESTING.md) - Automated testing documentation

## Acknowledgments

Vogix is inspired by projects in the theme ecosystem and incorporates scheme data from upstream sources:

- [tinted-theming/schemes](https://github.com/tinted-theming/schemes) - Source for base16/base24 schemes (via [fork](https://github.com/i-am-logger/tinted-schemes))
- [iTerm2-Color-Schemes](https://github.com/mbadolato/iTerm2-Color-Schemes) - Source for ansi16 schemes (via [fork](https://github.com/i-am-logger/iTerm2-Color-Schemes))
- [Base16](https://github.com/chriskempson/base16) - Palette standard that informed scheme conventions
- [Stylix](https://github.com/nix-community/stylix) - NixOS theming inspiration
- [Omarchy](https://github.com/basecamp/omarchy) - Runtime theme switching inspiration
