# Vogix Automated Testing

## Overview

Vogix includes automated integration tests using the NixOS testing framework. The tests verify features work correctly in an isolated VM environment.

## Running Tests

### Quick Test Run

```bash
# Run all integration tests
nix flake check

# Run specific test suites
nix build .#checks.x86_64-linux.smoke           # Quick sanity checks
nix build .#checks.x86_64-linux.architecture    # Symlinks, runtime dirs
nix build .#checks.x86_64-linux.theme-switching # Theme/variant switching
nix build .#checks.x86_64-linux.cli             # CLI flags, error handling
```

### What Gets Tested

The automated test suite is split into **12 wired suites** (one `nix build .#checks.<system>.<suite>` each):

1. **smoke** - Binary installs, `vogix theme status`/`theme list`, systemd daemon defined and starts
2. **architecture** - `~/.config` symlinks point to vogix-managed themed configs; runtime dirs created
3. **theme-switching** - `vogix theme set -t <name>` switches themes and regenerates app configs (alacritty, btop) with correct hex colors
4. **scheme-switching** - All 4 schemes (vogix16, base16, base24, ansi16) work; palette format validation
5. **navigation** - `vogix theme set -v darker/lighter/dark/light`; catppuccin multi-variant navigation
6. **cli** - Subcommand parsing, CLI flags, error handling, `--version`
7. **state** - State file created and persists changes
8. **session** - Session save/restore behavior
9. **runtime-size** - Runtime footprint / generated-config size bounds
10. **stress** - Rapid theme/variant switching
11. **templates** - Template architecture; templates bundled in the Nix package
12. **input-engine** - The evdev-grab → uinput re-emit / mock-compositor dispatch engine

The **input-engine** suite exercises, among others:

- **Plain-key re-emit** - an unbound key is re-emitted on the engine's virtual device (typing works, compositor-agnostic)
- **Super→Ctrl remap** - `Super+C/V` emit `Ctrl+C/V` at evdev; Super never leaks; numbers/excluded keys are not remapped (context-aware for terminal vs GUI)
- **CapsLock tap/hold** - caps-hold + bound key dispatches the WM command (bound key swallowed); caps-tap enters a sticky mode and exits cleanly
- **Sub-mode routing** - caps-hold → move/resize sub-modes, move↔resize switch, release returns to the app with no stuck mode
- **Esc safety-net** - Esc exits a catchall mode back to the app (typing resumes)
- **Single-instance guard** - a 2nd engine refuses with the lock message and never double-grabs; the 1st engine stays intact

## Test Output

When tests pass, you'll see:

```
=== Test 1: Vogix Binary Exists ===
✓ vogix binary found

=== Test 2: Check Status Command ===
✓ Status command works
Output: Current theme: yoga
Current variant: dark

=== Test 7: Variant Navigation ===
✓ Successfully navigated to darker variant
✓ Successfully navigated to lighter variant
✓ Successfully jumped to default dark variant

=== Test 8: Switch Theme and Verify Config Updates ===
✓ Successfully switched to catppuccin theme
✓ Alacritty config updated after theme switch

=== Test 13: Application Config Generation ===
✓ Alacritty config generated
✓ Alacritty config has color scheme
✓ Alacritty config contains hex colors
✓ Btop config generated
✓ Btop config contains hex colors

... (more tests) ...

============================================================
🎉 ALL TESTS PASSED!
============================================================

Test Summary:
✓ Binary installation
✓ CLI commands (status, list, -s, -t, -v)
✓ Configuration management
✓ State persistence
✓ Variant navigation (darker/lighter/dark/light)
✓ Theme switching with config updates
✓ Multi-scheme support (vogix16, base16, base24, ansi16)
✓ Symlink architecture verification
✓ Application config generation (alacritty, btop)
✓ Template bundling
✓ Systemd daemon service
✓ Shell completions
✓ Theme validation
✓ Error handling
✓ Version check
```

## Test Architecture

### NixOS Test Framework

The tests use `pkgs.nixosTest`, which:
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

**Test Scripts**: `nix/vm/tests/` (12 wired suites, plus `lib.nix` shared helpers)
- `smoke.nix` - Quick sanity checks (binary, status, list, systemd)
- `architecture.nix` - Symlinks, runtime directories
- `theme-switching.nix` - Theme and variant switching with config regeneration
- `scheme-switching.nix` - Cross-scheme tests, palette format validation
- `navigation.nix` - darker/lighter navigation, catppuccin multi-variant
- `cli.nix` - Subcommand parsing, CLI flags, error handling
- `state.nix` - State file creation and persistence
- `session.nix` - Session save/restore
- `runtime-size.nix` - Runtime footprint / generated-config size bounds
- `stress.nix` - Rapid switching
- `templates.nix` - Template architecture, bundling
- `input-engine.nix` - evdev-grab → uinput re-emit / mock-compositor dispatch engine

**Home Config**: `nix/vm/home.nix`
- User configuration for testing
- Themes installed
- Apps configured
- Daemon enabled

## Manual Testing

If you want to manually explore the test environment:

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

Add to your CI pipeline:

```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: cachix/install-nix-action@v22
        with:
          nix_path: nixpkgs=channel:nixos-unstable
      - run: nix flake check
```

## Test Development

### Adding New Tests

Create a new test file in `nix/vm/tests/` or add test cases to existing files:

```python
print("\n=== Test N: Your Test Name ===")
output = machine.succeed("su - vogix -c 'your command'")
assert "expected output" in output
print("✓ Your test passed")
```

### Test Helpers

- `machine.succeed(cmd)` - Run command, expect exit code 0
- `machine.fail(cmd)` - Run command, expect non-zero exit
- `machine.wait_for_unit(unit)` - Wait for systemd unit
- `machine.wait_for_file(path)` - Wait for file to exist
- `time.sleep(seconds)` - Wait for async operations

### Debugging Failed Tests

```bash
# Run test with more verbose output
nix build .#checks.x86_64-linux.smoke --print-build-logs

# Access the test VM interactively
nix run .#vogix-vm
```

## Performance

- **Test duration**: ~30-60 seconds per test suite
- **VM RAM**: 2GB
- **VM CPUs**: 2 cores
- **Storage**: Ephemeral (no persistence between runs)

## Coverage

The automated tests cover:

✅ All CLI commands
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

**Not covered** (requires real desktop environment):
- Actual live application reload (apps reading the configs)
- DBus reload signals in running applications
- Filesystem watching with running daemon
- Visual verification of colors in terminal

These require manual testing on a real system, but the core functionality - config generation and updates - is fully tested.

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

## Next Steps

After tests pass:

1. ✅ **CI Integration** - Add to GitHub Actions
2. ✅ **Release Testing** - Test on real NixOS system
3. ✅ **Documentation** - Update user docs with test results
4. ✅ **Benchmarks** - Add performance benchmarks (optional)

## Resources

- [NixOS Testing](https://nixos.org/manual/nixos/stable/#sec-nixos-tests)
- [VM Testing Examples](https://github.com/NixOS/nixpkgs/tree/master/nixos/tests)
- [Vogix Docs](../docs/)
