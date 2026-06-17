# Vogix Testing VM

This directory contains a NixOS VM configuration for testing Vogix functionality in an isolated environment.

## Building the VM

```bash
# Build the VM
nix build .#nixosConfigurations.vogix-test-vm.config.system.build.vm

# Or use the shorthand
nix run .#vogix-vm
```

## Running the VM

```bash
# Run the built VM
./result/bin/run-vogix-test-vm-vm

# The VM will auto-login as user 'vogix' with password 'vogix'
```

## VM Configuration

- **User**: vogix / vogix
- **Hostname**: vogix-test
- **Memory**: 2GB
- **Cores**: 2
- **Display**: Terminal only (no GUI)

## Installed Applications

- `alacritty` - Terminal emulator (themed)
- `btop` - System monitor (themed)
- `tmux` - Terminal multiplexer
- `vim` - Text editor
- `git` - Version control

## Testing Vogix

Once logged in, you can test Vogix features:

### 1. Check Status
```bash
vogix theme status
```

### 2. List Available Themes
```bash
vogix theme list
# Should show: yoga, forest, etc.
```

### 3. Switch Themes
```bash
# Switch to forest theme
vogix theme set -t forest -s vogix16

# Check alacritty config was updated
cat ~/.config/alacritty/colors.toml

# Switch back
vogix theme set -t yoga -s vogix16
```

### 4. Switch Variants
```bash
# Switch to light variant
vogix theme set -v light

# Switch back to dark
vogix theme set -v dark
```

### 5. Test Daemon

The `vogix-daemon` service is **disabled in this VM** (`enableDaemon = false` in
`home.nix`), so `systemctl --user status vogix-daemon` reports the unit is not
found. The daemon needs a live Hyprland session, which this terminal-only VM
does not provide.

For reference, when enabled the daemon (`vogix daemon`):
- restores the last saved Hyprland session on start (into an empty desktop only),
- re-applies the current theme's screen shader on start and on `configreloaded`,
- auto-saves the session on window events (open/close/move/workspace change),
- records submap-mode dwell times to `~/.local/state/vogix/modes.log` for
  modal-keybinding ergonomics telemetry.

It does **not** regenerate themes — theme files are produced by home-manager at
build time, not watched at runtime.

```bash
# (No-op in this VM — the unit is not installed.)
systemctl --user status vogix-daemon

# View daemon logs (empty here; the daemon is not running)
journalctl --user -u vogix-daemon -f
```

### 6. Generate Shell Completions
```bash
# Generate bash completions
vogix completions bash > ~/.local/share/bash-completion/completions/vogix

# Source it
source ~/.local/share/bash-completion/completions/vogix

# Test tab completion
vogix <TAB>
```

### 7. Check Paths
```bash
# Config manifest
cat ~/.local/state/vogix/config.toml

# Theme packages
ls -la ~/.local/share/vogix/themes/

# Current theme symlink
ls -la ~/.local/state/vogix/current-theme

# User state
cat ~/.local/state/vogix/state.toml
```

## Cleanup

```bash
# Exit the VM
exit  # or Ctrl+D

# Remove the VM
rm -rf result
```

## Modifying the VM

Edit `test-vm.nix` or `home.nix` and rebuild:

```bash
nix build .#nixosConfigurations.vogix-test-vm.config.system.build.vm
```
