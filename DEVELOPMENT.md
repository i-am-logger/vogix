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
- Nix formatting tools (nixpkgs-fmt)
- Required system dependencies (pkg-config, dbus)
- Pre-configured git hooks (rustfmt, clippy, nixpkgs-fmt)

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

### Integration Tests

```bash
# Full Nix flake check (includes all tests)
nix flake check

# Run specific integration test suites
nix build .#checks.x86_64-linux.smoke           # Quick sanity checks
nix build .#checks.x86_64-linux.architecture    # Symlinks, runtime dirs
nix build .#checks.x86_64-linux.theme-switching # Theme/variant switching
nix build .#checks.x86_64-linux.cli             # CLI flags, error handling
```

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
- `rustfmt` - Rust code formatting
- `clippy` - Rust linting
- `nixpkgs-fmt` - Nix code formatting

Manual checks before committing:
```bash
cargo fmt --check && \
cargo clippy -- -D warnings && \
cargo test && \
nixpkgs-fmt --check . && \
nix flake check --no-build
```

Or use devenv's test command:
```bash
devenv test  # Runs all git hooks
```

## Project Structure

```
vogix/
├── src/                        # Rust source code
│   ├── commands/               # Command handlers
│   │   ├── cache.rs            # Cache management
│   │   ├── completions.rs      # Shell completions
│   │   ├── daemon.rs           # Reload daemon
│   │   ├── input.rs            # Input engine / keybindings
│   │   ├── list.rs             # List themes
│   │   ├── modes.rs            # Paradigms and modes
│   │   ├── refresh.rs          # Refresh symlinks
│   │   ├── session.rs          # Session save/restore/undo
│   │   ├── shader.rs           # Hyprland screen shader
│   │   ├── status.rs           # Show status
│   │   └── theme_change.rs     # Theme/variant switching
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
│   │   │   └── vogix16.nix       # vogix16 helpers
│   │   ├── home-manager/       # Home-manager module (split)
│   │   │   ├── default.nix
│   │   │   ├── generators.nix
│   │   │   ├── options.nix
│   │   │   └── themes.nix
│   │   ├── applications/       # Application theme generators
│   │   │   ├── alacritty.nix
│   │   │   ├── btop.nix
│   │   │   └── ...
│   │   └── nixos.nix           # NixOS module
│   ├── packages/
│   │   └── vogix.nix           # Package definition
│   └── vm/
│       ├── tests/              # Integration tests (12 wired flake checks)
│       │   ├── lib.nix             # Shared test helpers
│       │   ├── smoke.nix           # Binary, status, list, systemd
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
│       │   └── input-engine.nix    # Input engine end-to-end
│       ├── test-vm.nix         # VM configuration
│       └── home.nix            # Test user config
│
├── docs/                       # Documentation
│   ├── architecture.md         # System architecture
│   ├── cli.md                  # CLI reference
│   ├── theming.md              # Theme format
│   ├── reload.md               # Reload mechanisms
│   └── app-module-template.nix # Template for new app modules
│
├── scripts/                    # Development scripts
│   └── demo.sh                 # Demo script
│
├── .github/
│   ├── workflows/              # CI/CD pipelines
│   │   ├── ci-and-release.yml  # Consolidated CI + release automation
│   │   └── release.yml         # Binary releases
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

### Debugging

#### Enable Rust Backtrace
```bash
RUST_BACKTRACE=1 cargo run -- status
RUST_BACKTRACE=full cargo run -- -t forest
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
1. CLI updates `current-theme` symlink (only this!)
2. Supports variant navigation (darker/lighter/dark/light)
3. Triggers application reloads per config
4. Persists state to `~/.local/state/vogix/`

**Key Principle**: Nix generates everything at build time. Rust CLI only manages symlinks.

### Directory Locations

| What | Path | Managed By |
|------|------|------------|
| Config manifest | `~/.local/state/vogix/config.toml` | home-manager |
| Theme packages | `~/.local/share/vogix/themes/` | home-manager |
| Current symlink | `~/.local/state/vogix/current-theme` | Rust CLI |
| User state | `~/.local/state/vogix/state.toml` | Rust CLI |
| App configs | `~/.config/{app}/` | home-manager symlinks |

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

### Consolidated Workflow (`.github/workflows/ci-and-release.yml`)

A single, efficient workflow handles both CI and releases with smart job dependencies:

**All CI jobs run in parallel for maximum speed:**

**Job 1: Fast Checks**
- `devenv-checks` - Runs `devenv test` which executes all git hooks:
  - Nix code formatting (nixpkgs-fmt)
  - Rust formatting (rustfmt)
  - Rust linting (clippy)

**Job 2: Nix Checks** (parallel with Job 1 & 3)
- `nix-checks`:
  - Runs `nix flake check` (includes Rust tests)
  - Builds Nix package

**Job 3: Integration Tests** (parallel with Job 1 & 2)
- `integration-tests`:
  - Runs integration tests
  - Tests VM-based functionality

**Job 4: Release** (depends on all CI passing)
- `release-please`:
  - Creates/updates release PRs from conventional commits
  - Creates Git tags when release PRs are merged
  - Only runs on push to master
  - Blocked if any CI checks fail

**Smart Optimizations:**
- Skips CI on release-please PRs (version bump only)
- Uses `devenv test` - same checks as local development
- Formatting/linting fails fast (10-30 seconds) before expensive builds
- All three CI jobs run in parallel for maximum speed
- Release job explicitly depends on all CI jobs passing
- Total: 4 jobs instead of original 6 (33% reduction)

### Binary Releases (`.github/workflows/release.yml`)
- Builds for x86_64-linux and aarch64-linux
- Uploads to GitHub Releases
- Uses GitHub Actions cache

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
