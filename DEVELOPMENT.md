# Development Guide

This guide covers setting up a development environment and contributing to Vogix.

## Prerequisites

- Nix with flakes enabled
- Rust Edition 2024 (provided by Nix)
- devenv (automatically available via flake)

## Quick Start

### Clone the Repository

```bash
git clone https://github.com/i-am-logger/vogix
cd vogix
```

### Enter Development Environment

```bash
# Using devenv (recommended)
devenv shell

# Note: 'nix develop --impure' has known issues with platform-specific dependencies
# Use 'devenv shell' for the full development experience
```

This provides:
- Rust toolchain (rustc, cargo, rustfmt, clippy, rust-analyzer)
- treefmt with nixpkgs-fmt, deadnix, statix, rustfmt, shellcheck and shfmt
- Required system dependencies (pkg-config, dbus)
- Pre-configured git hooks (treefmt, clippy)

## Building

### Rust Binary (Development)

```bash
# Development build
cargo build

# Release build
cargo build --release

# Check without building
cargo check
```

### Nix Package (Production)

```bash
# Build the package (recommended)
nix build .#vogix

# Build for specific architecture
nix build .#packages.x86_64-linux.vogix
nix build .#packages.aarch64-linux.vogix
```

The package is built with nixpkgs' standard `rustPlatform.buildRustPackage` (`nix/packages/vogix.nix`) for a reproducible Rust build.

## Testing

### Unit Tests

```bash
# Run Rust unit tests
cargo test

# Run with output
cargo test -- --nocapture
```

The unit tests include the CLI reference: every `vogix …` example in
[docs/cli.md](docs/cli.md) must parse, and every command must have one, so a
CLI change that leaves the reference behind fails `cargo test`.

### Integration Tests

```bash
# Full Nix flake check (includes all tests)
nix flake check

# Run specific integration test suites
nix build .#checks.x86_64-linux.smoke           # Quick sanity checks
nix build .#checks.x86_64-linux.architecture    # Symlinks, runtime dirs
nix build .#checks.x86_64-linux.theme-switching # Theme/variant switching
nix build .#checks.x86_64-linux.cli             # CLI flags, error handling

# The desktop shell (no VM; seconds to minutes each)
nix build .#checks.x86_64-linux.desktop-qmllint -L --no-link     # lint the QML, plus the shell's own rules
nix build .#checks.x86_64-linux.desktop-logic -L --no-link       # Qt Quick Test over the pure logic, probe scripts on fixtures
nix build .#checks.x86_64-linux.desktop-smoke -L --no-link       # the real shell under a headless compositor, widget fit included
nix build .#checks.x86_64-linux.desktop-options -L --no-link     # the desktop.json pin, the registry-typed layouts, module assertions
nix build .#checks.x86_64-linux.desktop-runtime -L --no-link     # every program the shell starts is on its unit's PATH
nix build .#checks.x86_64-linux.desktop-taps -L --no-link        # the audio taps against a real PipeWire
nix build .#checks.x86_64-linux.desktop-backgrounds -L --no-link # every theme variant ships its backgrounds

# The desktop shell on a real Hyprland session (a VM; about two minutes of test time)
nix build .#checks.x86_64-linux.desktop-hyprland -L --no-link
```

[TESTING.md](TESTING.md#what-gets-tested) says what each check covers.

### VM Testing

```bash
# Launch test VM
nix run .#vogix-vm

# Inside the VM, test commands:
vogix theme status
vogix theme list
vogix theme list -s base16
vogix theme set -s base16 -t catppuccin -v mocha
vogix theme set -v darker
vogix theme set -v lighter
```

See [TESTING.md](TESTING.md) for testing documentation.

## Code Quality

### Formatting

```bash
# Check formatting
cargo fmt --check

# Auto-format code
cargo fmt
```

### Linting

```bash
# Run Clippy
cargo clippy

# Clippy with all warnings as errors
cargo clippy -- -D warnings
```

### Pre-commit Checks

Git hooks are automatically configured when you enter `devenv shell`. They run:
- `treefmt` - nixpkgs-fmt, deadnix, statix, rustfmt, shellcheck, shfmt
- `clippy` - Rust linting

Before committing, run what CI runs first:
```bash
devenv test               # treefmt --fail-on-change, clippy -D warnings, cargo check, cargo test
nix flake check --no-build
```

## Project Structure

```
vogix/
├── src/                        # Rust source code
│   ├── commands/               # Command handlers
│   │   ├── cache.rs            # Cache management
│   │   ├── completions.rs      # Shell completions
│   │   ├── daemon.rs           # Reload daemon
│   │   ├── desktop.rs          # `vogix desktop` verbs and `desktop check`
│   │   ├── greeter.rs          # SDDM greeter theme sync
│   │   ├── hypr.rs             # Dialect-aware Hyprland IPC
│   │   ├── input.rs            # Input engine / keybindings
│   │   ├── list.rs             # List themes
│   │   ├── machine.rs          # `vogix machine` owners, status, validate, inspect
│   │   ├── modes.rs            # Paradigms and modes
│   │   ├── refresh.rs          # Refresh symlinks
│   │   ├── session.rs          # Session save/restore/undo
│   │   ├── shader.rs           # Hyprland screen shader
│   │   ├── status.rs           # Show status
│   │   └── theme_change.rs     # Theme/variant switching
│   ├── input/                  # The input engine (router, evdev/uinput loop, schema, lock LEDs, …)
│   ├── machine/                # Machine surfaces: config and palette loaders, publish, reactor,
│   │                           #   command runner, VT palette, status; openrgb/ is the SDK client
│   ├── cache/                  # Theme cache module
│   │   ├── paths.rs            # Cache path management
│   │   ├── renderer.rs         # Config rendering
│   │   └── tests.rs
│   ├── config/                 # Configuration
│   │   ├── types.rs            # Config types
│   │   └── tests.rs
│   ├── template/               # Tera template rendering
│   │   ├── filters.rs          # Custom filters
│   │   ├── render.rs           # Render logic
│   │   └── tests.rs
│   ├── theme/                  # Theme management
│   │   ├── discovery.rs        # Theme discovery
│   │   ├── loader/             # Theme loaders by scheme
│   │   ├── query.rs            # Theme queries
│   │   └── types.rs            # Theme types
│   ├── cli.rs                  # CLI definition (clap)
│   ├── errors.rs               # Error handling
│   ├── fsutil.rs               # Atomic writes of files other processes read
│   ├── main.rs                 # Entry point
│   ├── reload.rs               # Application reload mechanisms
│   ├── scheme.rs               # Color scheme types
│   ├── state.rs                # State persistence
│   └── symlink.rs              # Symlink management
│
├── nix/
│   ├── modules/
│   │   ├── lib/                # Shared libraries
│   │   │   ├── applications.nix  # App discovery
│   │   │   ├── colors.nix        # Color utilities
│   │   │   ├── vogix16.nix       # vogix16 helpers
│   │   │   └── vogix-users.nix   # The home-manager users with programs.vogix.enable
│   │   ├── home-manager/       # Home-manager module (split)
│   │   │   ├── default.nix
│   │   │   ├── generators.nix
│   │   │   ├── options.nix
│   │   │   └── themes.nix
│   │   ├── applications/       # Application theme generators
│   │   │   ├── alacritty.nix
│   │   │   ├── btop.nix
│   │   │   ├── vogix-desktop.nix # The desktop shell's theme.json
│   │   │   └── ...
│   │   ├── hardware/           # vogix.hardware.* modules (DRAM, Keychron, Kraken) and their devices
│   │   ├── desktop/            # programs.vogix.desktop.* options, defaults,
│   │   │                       #   and the pinned default desktop.json
│   │   ├── machine.nix         # vogix.machine: machine.json, the drop zone and the machine owner units
│   │   ├── openrgb.nix         # vogix.openrgb: the OpenRGB SDK server and its settings
│   │   └── nixos.nix           # NixOS module
│   ├── checks/                 # Checks too large for flake.nix
│   │   ├── machine-contract.nix # The machine module at evaluation and through the owners' loader
│   │   ├── desktop-smoke.nix   # The real shell under a headless compositor
│   │   ├── desktop-taps.nix    # The audio taps against a real PipeWire
│   │   ├── desktop-geometry-probe.qml # desktop-smoke's widget size and fit probe
│   │   ├── desktop-geometry-feed.nix  # the data every widget shows for that probe
│   │   ├── desktop-state-probe.qml    # desktop-smoke's window-title, mode and card probe
│   │   └── pipewire-daemon.nix # A hardware-less PipeWire for the checks
│   ├── packages/
│   │   ├── vogix.nix           # Package definition
│   │   ├── openrgb.nix         # vogix's OpenRGB build (notifies systemd when listening)
│   │   └── vogix-desktop-qml.nix # The desktop shell's QML tree
│   └── vm/
│       ├── tests/              # NixOS VM suites (one flake check each)
│       │   ├── lib.nix             # Shared test helpers
│       │   ├── smoke.nix           # Binary, status, list, login shells, vogix-machine, the session restore unit
│       │   ├── architecture.nix    # Symlinks, runtime dirs
│       │   ├── theme-switching.nix # Theme/variant switching
│       │   ├── scheme-switching.nix # Cross-scheme, palette format
│       │   ├── navigation.nix      # Darker/lighter navigation
│       │   ├── cli.nix             # CLI flags, error handling
│       │   ├── state.nix           # State persistence
│       │   ├── session.nix         # Session save/restore/undo
│       │   ├── runtime-size.nix    # Runtime size inspection
│       │   ├── stress.nix          # Rapid switching
│       │   ├── templates.nix       # Template architecture
│       │   ├── input-engine.nix    # Input engine end-to-end
│       │   ├── desktop-hyprland.nix # The desktop shell under Hyprland
│       │   ├── openrgb-readiness.nix # openrgb.service readiness and settings
│       │   └── machine-release.nix # Both machine owners as the module declares them, against the real OpenRGB server
│       ├── test-vm.nix         # VM configuration
│       └── home.nix            # Test user config
│
├── desktop/                    # The desktop shell (quickshell QML)
│   ├── shell.qml               # Entry point and the IPC targets behind `vogix desktop`
│   ├── Vogix/                  # Contract readers (desktop.json, theme.json, current-mode) and design tables
│   ├── Services/               # One singleton per source (audio, stats, notifications, lock, idle, …)
│   │   └── lib/                # Pure parsing and policy (JavaScript), unit-tested
│   ├── Bar/                    # The four bars, the section registry, and every widget
│   ├── Components/             # Meters, sparklines, readouts, ballistics
│   ├── Geometry/               # Placement of floating surfaces
│   ├── Panels/  Notifications/  Launcher/  Lock/  Idle/  Osd/  Power/  Polkit/
│   ├── Background/  Decorations/  DevGallery/
│   ├── Greeter/                # The SDDM greeter theme (its own package)
│   └── data/                   # Shaders and the sysfs probe scripts
├── tests/desktop/              # desktop-logic: Qt Quick Test cases and probe fixtures
├── tests/fixtures/             # Captured OpenRGB payloads, kernel uevents, theme and input fixtures
├── tests/machine_reactor_signals.rs # The reactor's signalfd, outside the libtest harness
│
├── docs/                       # Documentation
│   ├── architecture.md         # System architecture
│   ├── cli.md                  # CLI reference
│   ├── desktop.md              # The desktop shell
│   ├── hyprland-lua-ipc.md     # Hyprland IPC under both config engines
│   ├── theming.md              # Theme format
│   ├── reload.md               # Reload mechanisms
│   └── app-module-template.nix # Template for new app modules
│
├── scripts/                    # Development scripts
│   └── demo.sh                 # Demo script
│
├── .github/
│   ├── workflows/              # CI/CD pipeline
│   │   └── ci-and-release.yml  # CI and release automation
│   └── ISSUE_TEMPLATE/         # Issue templates
│
├── Cargo.toml                  # Rust dependencies (version source of truth)
├── flake.nix                   # Nix flake definition
├── CONTRIBUTING.md             # Contribution guidelines
├── CHANGELOG.md                # Version history
└── README.md                   # Project overview
```

## Common Development Tasks

### Adding a New Theme

Themes are now maintained in the separate [vogix16-themes](https://github.com/i-am-logger/vogix16-themes) repository.

1. Clone the themes repo:
   ```bash
   git clone https://github.com/i-am-logger/vogix16-themes
   cd vogix16-themes
   ```

2. Create theme files in TOML format:
   ```bash
   mkdir themes/mytheme
   ```

   ```toml
   # themes/mytheme/dark.toml
   polarity = "dark"
   
   [colors]
   base00 = "#1a1a1a"
   base01 = "#282828"
   base02 = "#383838"
   base03 = "#585858"
   base04 = "#b8b8b8"
   base05 = "#d8d8d8"
   base06 = "#e8e8e8"
   base07 = "#f8f8f8"
   base08 = "#ab4642"
   base09 = "#dc9656"
   base0A = "#f7ca88"
   base0B = "#a1b56c"
   base0C = "#86c1b9"
   base0D = "#7cafc2"
   base0E = "#ba8baf"
   base0F = "#a16946"
   ```

3. Validate your theme:
   ```bash
   python scripts/validate-themes.py themes/mytheme
   ```

4. Submit a PR to vogix16-themes

See the [vogix16-themes README](https://github.com/i-am-logger/vogix16-themes) for detailed guidelines.

### Adding Application Support

1. Create generator in `nix/modules/applications/`:

   ```nix
   # nix/modules/applications/myapp.nix
   _:
   {
     configFile = "myapp/config.conf";
     format = "toml";  # or "ini", "yaml", "text"
     settingsPath = "programs.myapp.settings";
     reloadMethod = { method = "touch"; };  # or "signal", "command", "none"
     
     schemes = {
       vogix16 = colors: {
         background = colors.background;
         foreground = colors.foreground-text;
         error = colors.danger;
       };
       
       base16 = colors: {
         background = colors.base00;
         foreground = colors.base05;
         red = colors.base08;
       };
       
       base24 = colors: {
         background = colors.base00;
         foreground = colors.base05;
         bright-red = colors.base12;
       };
       
       ansi16 = colors: {
         background = colors.background;
         foreground = colors.foreground;
         red = colors.red;
       };
     };
   }
   ```

   Note: Use `_:` if the module doesn't need parameters, or `{ lib, ... }:` if it needs `lib`.

2. Test integration:
   ```bash
   nix flake check
   ```

See [docs/app-module-template.nix](docs/app-module-template.nix) for a complete template.

### Working on the Desktop Shell

The shell is the QML tree in `desktop/` (see
[the architecture](docs/architecture.md#8-desktop-shell) and
[the user docs](docs/desktop.md)). Its options and defaults are in
`nix/modules/desktop/`, the unit and `desktop.json` rendering in
`nix/modules/home-manager/default.nix`, and its verbs in
`src/commands/desktop.rs`.

- **Lint and test without a session**: `desktop-qmllint`, `desktop-logic`,
  `desktop-smoke` and `desktop-taps` (commands above). Logic that can be a
  pure function goes in `desktop/Services/lib/*.js` with a `tst_*.qml` case in
  `tests/desktop/`; a sysfs probe goes in `desktop/data/*.sh` with a fixture
  case in `tests/desktop/probes.sh`.
- **What needs a compositor** (clicks, layer-shell placement, the session
  lock, screencasts, keyboard layouts, NetworkManager): `desktop-hyprland`
  runs the shell on a real Hyprland session in a VM, on both config
  providers.
- **A default changed**: `desktop-options` fails until
  `nix/modules/desktop/desktop-json.pin.json` matches the new rendering, so
  every change to what the shell receives is a reviewed edit of that file.
- **A new widget**: add its QML under `desktop/Bar/widgets/` (declare
  `property BarAxis axis` to receive the bar's context, and hold any data
  source through a `Lease` on `axis.live`), give it an entry in
  `desktop/Bar/widgets/registry.json` (its name, its component, and
  `"orientation": "horizontal"` or `"vertical"` when it renders on one bar
  orientation only), and list it in `docs/desktop.md`. The registry is the
  only list of widget names: the shell resolves names through it, the layout
  options are typed from it per bar orientation, and `vogix desktop check`
  compiles it in. `desktop-smoke` places every registry widget, and its
  geometry probe lays each one out on every bar edge the registry lets it sit
  on: a widget that overflows a bar it is allowed on fails the check until its
  `orientation` says so, or it fits. A new widget is loaded and measured
  without editing the check when it shows with the data the probe is fed
  (`nix/checks/desktop-geometry-feed.nix`: UPower, BlueZ, sysfs, PipeWire,
  Hyprland's events, wttrbar). One that needs a source the feed lacks never
  shows and fails the check until the feed gives it the widest realistic
  data it shows.
- **A new verb**: add the subcommand in `src/cli.rs`, its relay in
  `src/commands/desktop.rs` (an `IpcCall` through the `Shell` seam, with a
  case in the `relayed_verbs` table its tests drive), the IPC function in
  `desktop/shell.qml`, and an example in `docs/cli.md` (the unit tests require
  one).
- **On a real session**, a home-manager switch that changes the QML package
  restarts `vogix-desktop.service`; one that changes only `desktop.json`
  reloads it in place. `vogix desktop check`, `status`, `meters`, `stats`,
  `privacy`, `keyboard` and `bar geometry` show what the running shell sees,
  and warnings go to `journalctl --user -u vogix-desktop`.

### Debugging

#### Enable Rust Backtrace
```bash
RUST_BACKTRACE=1 cargo run -- theme status
RUST_BACKTRACE=full cargo run -- theme set -t forest
```

#### Check Generated Configs
```bash
# After home-manager switch
ls -la ~/.local/share/vogix/themes/
cat ~/.local/state/vogix/config.toml

# Check symlinks
ls -la ~/.config/alacritty/colors.toml
readlink ~/.config/alacritty/colors.toml

# Check state
cat ~/.local/state/vogix/state.toml
```

#### Nix Debugging
```bash
# Show flake outputs
nix flake show

# Evaluate specific attribute
nix eval .#packages.x86_64-linux.vogix.version

# Build with verbose output
nix build --print-build-logs

# Show trace on errors
nix flake check --show-trace
```

## Known Issues

### Nix Eval Cache During Development

**Problem**: When modifying application modules (e.g., `nix/modules/applications/btop.nix`), Nix's evaluation cache may return stale results even though files are git-tracked and the flake detects a dirty tree.

**Symptoms**:
- You modify an application module file
- Run `git add` to track the change
- Build the VM or run flake check
- Generated configs still have OLD content

**Root Cause**: This is a known limitation of Nix flakes evaluation cache for local repositories under active development. The eval cache has race conditions/bugs with dirty git trees. See: [NixOS/nix#12102](https://github.com/NixOS/nix/pull/12102)

**Workarounds**:

1. **Use development helpers** (recommended):
   ```bash
   # VM launcher (automatically disables eval cache)
   nix run .#vogix-vm

   # Check flake without eval cache
   nix run .#dev-check

   # From inside devenv shell
   nix-build-dev         # Build VM without eval cache
   nix-check-dev         # Check flake without eval cache
   ```

2. **Manual flag** (for other nix commands):
   ```bash
   nix build --option eval-cache false
   nix flake check --option eval-cache false
   ```

3. **Force cache invalidation** (make trivial edit to `flake.nix`):
   ```bash
   # Add/remove a comment in flake.nix
   # This changes the flake fingerprint → cache invalidates
   ```

**Status**: Waiting for upstream Nix to implement automatic eval cache disabling for local repos. Track progress in issue [#101](https://github.com/i-am-logger/vogix/issues/101).

## Architecture Overview

### Build Time (Nix)
1. Home-manager module discovers themes:
   - Native vogix16 themes from [vogix16-themes](https://github.com/i-am-logger/vogix16-themes) repo (TOML format)
   - Imported base16/base24 from tinted-schemes fork
   - Imported ansi16 from iTerm2-Color-Schemes fork
2. Discovers application generators from `nix/modules/applications/`
3. For each (scheme × theme × variant × app) combination, generates configs
4. Stores generated configs in `/nix/store` (immutable)
5. Symlinks configs to `~/.local/share/vogix/themes/`

### Runtime (Rust CLI)
1. CLI updates the `current-theme` symlink
2. Supports variant navigation (darker/lighter/dark/light)
3. Renders the Tera templates into its cache (`~/.cache/vogix/`) on `theme set` and `theme refresh`
4. Triggers application reloads per config (the desktop shell's is `vogix desktop reload`)
5. Persists state to `~/.local/state/vogix/`
6. Runs the input engine (`vogix input run`) and relays the desktop shell's verbs (`vogix desktop …`)

**Key Principle**: Nix generates the theme packages and every configuration file at build time; at runtime the CLI flips the `current-theme` symlink, renders the templates into its cache, and signals the programs that read them.

### Directory Locations

| What | Path | Managed By |
|------|------|------------|
| Config manifest | `~/.local/state/vogix/config.toml` | home-manager |
| Theme packages | `~/.local/share/vogix/themes/` | home-manager |
| Current symlink | `~/.local/state/vogix/current-theme` | Rust CLI |
| User state | `~/.local/state/vogix/state.toml` | Rust CLI |
| App configs | `~/.config/{app}/` | home-manager symlinks |
| Input engine schema | `~/.local/state/vogix/input.json` | home-manager |
| Engine state (`current-mode`, `input-locks.json`, `input-health.json`) | `~/.local/state/vogix/` | input engine |
| Desktop shell configuration | `~/.local/state/vogix/desktop.json` | home-manager |
| Desktop shell colors | `~/.config/vogix-desktop/theme.json` | home-manager symlink through `current-theme` |
| Desktop shell state | `~/.local/state/vogix/desktop/` | the desktop shell |

## Version Management

**Single Source of Truth**: `Cargo.toml` (version field)

All components derive from here:
- CLI: Uses `env!("CARGO_PKG_VERSION")` at compile time
- Nix package: Reads Cargo.toml via `builtins.fromTOML`
- release-please: Updates Cargo.toml automatically

Users pin versions via Git tags (release tags are `vogix-` prefixed):
```nix
inputs.vogix.url = "github:i-am-logger/vogix/vogix-v0.7.0";
```

## Conventional Commits

We use conventional commits for automated changelog and versioning:

```
feat(themes): add nord theme
fix(cli): resolve symlink race condition
docs(architecture): clarify Nix generation
chore(deps): update rust dependencies
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`

Breaking changes:
```
feat(cli): change switch command to auto-toggle

BREAKING CHANGE: vogix switch no longer takes arguments
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for complete guidelines.

## CI/CD Pipeline

### Workflow (`.github/workflows/ci-and-release.yml`)

One workflow runs on pull requests and pushes to `master`, in three jobs:

1. **Lint/UT** (`devenv-checks`): `devenv test`, the same checks as local
   development — treefmt, clippy, `cargo check`, `cargo test`.
2. **Integration** (`nix-checks`, after Lint/UT): `nix flake check
   --print-build-logs` on a runner with KVM, so every flake check runs: the
   pure-Nix tests, the desktop shell's sandbox checks, and the VM suites.
3. **Release** (`release-please`, pushes to `master` only, after both pass):
   creates or updates the release PR from conventional commits, and tags the
   release when that PR merges.

CI is skipped for release-please's own PRs and release merges, which only
bump versions.

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Clap Documentation](https://docs.rs/clap/)
- [Nix Manual](https://nixos.org/manual/nix/stable/)
- [Home Manager Manual](https://nix-community.github.io/home-manager/)
- [NixOS Wiki](https://nixos.wiki/)
- [Conventional Commits](https://www.conventionalcommits.org/)

## Getting Help

- **Documentation**: Check [docs/](docs/) directory
- **Issues**: Search or create on GitHub
- **Contributing**: See [CONTRIBUTING.md](CONTRIBUTING.md)

---

Happy hacking!
