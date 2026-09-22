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
| `nix-unit` | The config generators: appearance, behavior, the Hyprland render in both config dialects (hyprlang and Lua), and the theme.json contract pinned to the same golden line as the Rust template test. Runs at evaluation, so `--no-build` covers it. |
| `appearance-options` | `programs.vogix.appearance.*` reaches the rendered Hyprland config through the module system. |
| `desktop-options` | The default `desktop.json` equals `nix/modules/desktop/desktop-json.pin.json` byte for byte; horizontal-only widgets on a side bar, undefined `custom/<name>` placements and bad cell names fail evaluation; the shell's unit restarts on exit status 75, waits for PipeWire, and runs quickshell without detailed logs. |
| `desktop-runtime` | Every program the shell's QML starts by name is on the `vogix-desktop` unit's own `PATH` (or is a base-system tool or a client of a host daemon), and the NixOS module enables UPower and power-profiles-daemon exactly while a user runs the shell. |
| `desktop-qmllint` | qmllint over the shell's QML against the pinned quickshell, unused imports included, plus the shell's rules: square corners, no raw `Hyprland.dispatch()`, bar widgets open panels beside their bar, tray menus open through `QsMenuAnchor`. |
| `desktop-logic` | The shell's pure logic under Qt Quick Test (the parsers and policies in `desktop/Services/lib/`, meter ballistics, bar leases, popup placement) and its sysfs probe scripts against fixture trees. |
| `desktop-smoke` | The real shell starts under a headless cage compositor against fixture contract files: the bar, panel, power, launcher, gallery, reminder, custom-cell and keyboard verbs answer; custom cells run on every trigger but the timer; CAPS follows the input engine's lock file; the oscilloscope's canvas never lays out collapsed; a schema-1 `desktop.json` is refused without taking the shell down. cage has no layer-shell, so the bars do not map here. |
| `desktop-taps` | The audio taps against a real PipeWire daemon: nothing runs while nothing plays, a playback stream starts the taps and the output VU, a dead tap is relaunched, taps wait for a default sink, a hidden bar's sources stop, a PipeWire restart is survived, and the unit writes no debug records. |
| `desktop-backgrounds` | Every desktop theme variant ships its background set (the generated background, the aurora shader, merged extras), and the shell ships the shader precompiled. |

### VM suites

| Check | What it proves |
|---|---|
| `smoke` | Binary installs, `vogix theme status`/`theme list`, the systemd daemon is defined and starts |
| `architecture` | `~/.config` symlinks point at vogix-managed themed configs; runtime dirs are created |
| `theme-switching` | `vogix theme set -t <name>` switches themes and rewrites app configs (alacritty, btop) with the right colors |
| `scheme-switching` | All 4 schemes (vogix16, base16, base24, ansi16) work; palette format validation |
| `navigation` | `vogix theme set -v darker/lighter/dark/light`; catppuccin multi-variant navigation |
| `cli` | Subcommand parsing, CLI flags, error handling, `--version` |
| `state` | The state file is created and persists changes |
| `session` | Session save/restore behavior |
| `runtime-size` | Runtime footprint / generated-config size bounds |
| `stress` | Rapid theme/variant switching |
| `templates` | Template architecture; templates bundled in the Nix package |
| `input-engine` | The evdev-grab → uinput re-emit / mock-compositor dispatch engine (below) |
| `desktop-hyprland` | The desktop shell in a real Hyprland session with PipeWire: what cage cannot host |

The **input-engine** suite exercises, among others:

- **Plain-key re-emit** - an unbound key is re-emitted on the engine's virtual device (typing works, compositor-agnostic)
- **Super→Ctrl remap** - `Super+C/V` emit `Ctrl+C/V` at evdev; Super never leaks; numbers/excluded keys are not remapped (context-aware for terminal vs GUI)
- **CapsLock tap/hold** - caps-hold + bound key dispatches the WM command (bound key swallowed); caps-tap enters a sticky mode and exits cleanly
- **Sub-mode routing** - caps-hold → move/resize sub-modes, move↔resize switch, release returns to the app with no stuck mode
- **Esc safety-net** - Esc exits a catchall mode back to the app (typing resumes)
- **Single-instance guard** - a 2nd engine refuses with the lock message and never double-grabs; the 1st engine stays intact
- **Lock LEDs** - `input-locks.json` follows the grabbed keyboard's CapsLock/NumLock LEDs

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
`desktop-qmllint`, `desktop-smoke`, `desktop-backgrounds`),
`nix/checks/desktop-taps.nix`, `nix/checks/desktop-geometry-probe.qml` (the
smoke's canvas probe) and `tests/desktop/` (`desktop-logic`: one
`tst_*.qml` per unit, `probes.sh` for the probe scripts).

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

**Checked by hand**:
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
