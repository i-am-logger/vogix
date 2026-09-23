# Application Configuration Modules

This directory contains Vogix theme configuration generators for various applications.

## Multi-Scheme Support

Application generators support all 4 color schemes through the `schemes` pattern:

```nix
# nix/modules/applications/alacritty.nix
_:
{
  configFile = "alacritty/colors.toml";
  format = "toml";
  settingsPath = "programs.alacritty.settings";
  reloadMethod = { method = "touch"; };
  
  schemes = {
    vogix16 = colors: {
      # Semantic color usage (vogix16 philosophy)
      colors.primary.background = colors.background;
      colors.primary.foreground = colors.foreground-text;
      colors.normal.red = colors.danger;
      colors.normal.green = colors.success;
    };
    
    base16 = colors: {
      # Base16 standard mapping
      colors.primary.background = colors.base00;
      colors.primary.foreground = colors.base05;
      colors.normal.red = colors.base08;
      colors.normal.green = colors.base0B;
    };
    
    base24 = colors: {
      # Base24 with additional bright colors
      colors.primary.background = colors.base00;
      colors.primary.foreground = colors.base05;
      colors.bright.red = colors.base12;
      colors.bright.green = colors.base14;
    };
    
    ansi16 = colors: {
      # ANSI standard slot mapping
      colors.primary.background = colors.background;
      colors.primary.foreground = colors.foreground;
      colors.normal.red = colors.red;
      colors.normal.green = colors.green;
    };
  };
}
```

Note: Use `_:` if the module doesn't need parameters, or `{ lib, ... }:` if it needs `lib`.

## Configuration Strategy

Vogix's minimalist philosophy ("functional colors for minimalist minds") applies to the **vogix16 scheme**. Other schemes follow their own conventions.

### vogix16 Scheme: Minimal ANSI by Default

For terminal emulators (alacritty, console), ANSI color slots are mapped minimally:
- **Monochromatic base** for most slots (blue, magenta, cyan → foreground colors)
- **Semantic colors only** for red/green/yellow (danger/success/warning)

This creates a minimal terminal by default.

### base16/base24/ansi16 Schemes: Standard Mappings

These schemes follow their respective standards, providing full color support as defined by their specifications.

### Application-Specific Configurations

**Examples**: btop, ls/eza, git, ripgrep, bat, vim, neovim, tmux, etc.

Each application gets scheme-aware configuration with appropriate color mappings:

- **vogix16**: Semantic colors for functional indicators only
- **base16/base24**: Standard syntax highlighting mappings
- **ansi16**: Traditional ANSI terminal colors

## Key Principle

**Scheme-appropriate coloring**: Each scheme generator respects the philosophy of that scheme. The vogix16 scheme is minimal by design; base16/base24/ansi16 provide full color support per their standards.

## Applications and Machine Surfaces

A generator themes a per-user file that `current-theme` switches, and its
`reloadMethod` tells running instances. Hardware that belongs to the machine
(RGB LEDs, cooler rings) is not an application: it is a device in the NixOS
option `vogix.hardware.devices`, applied by the machine owner units (see
[the machine surfaces](../../../docs/architecture.md#9-machine-surfaces)).

The `console` generator's `console/palette` is the palette the machine owner
publishes for the VT: the machine owner unit `vogix-machine` writes it into the
kernel for every VT. The generator's reload command acts only when the CLI runs
on a text VT: it loads the palette into the kernel (`setvtrgb`) and switches
away and back, so the VT it runs on shows the new colours at once. Its ANSI
mapping is `templates/<scheme>/console.palette.vogix`, read as data by
`nix/modules/lib/console-palette.nix`: the runtime renders the same template,
and the NixOS module builds `console.colors` from the same lines.

## Adding New Application Modules

When adding a new application:

1. **Create the generator file** in `nix/modules/applications/`
2. **Implement all 4 schemes** in the `schemes` attribute
3. **Follow each scheme's philosophy**:
   - vogix16: Only use functional colors for semantic meaning
   - base16/base24: Map to syntax highlighting categories
   - ansi16: Map to ANSI color slots
4. **Test with multiple schemes** to ensure correct output

### Generator Template

See [docs/app-module-template.nix](../../docs/app-module-template.nix) for a complete template.

```nix
_:
{
  configFile = "path/to/config";
  format = "toml";  # or "ini", "yaml", "text"
  settingsPath = "programs.app.settings";
  reloadMethod = { method = "touch"; };  # or "signal", "command", "none"
  
  schemes = {
    vogix16 = colors: {
      # Config using semantic names: colors.danger, colors.success, etc.
    };
    
    base16 = colors: {
      # Config using base16 names: colors.base00, colors.base08, etc.
    };
    
    base24 = colors: {
      # Config using base24 names: colors.base00-base17
    };
    
    ansi16 = colors: {
      # Config using ANSI names: colors.red, colors.green, etc.
    };
  };
}
```

### Determining Semantic vs. Monochromatic (vogix16 scheme)

Ask: "Does this color convey information the user needs to know?"

**YES - Use functional colors:**
- Error states, warnings, success indicators
- Resource utilization levels (CPU at 90% vs 10%)
- Temperature/status gradients (cool → warm → hot)
- Active/selected/focused items
- Added/removed/modified content (git diffs, file changes)
- Important notifications or alerts

**NO - Use monochromatic:**
- UI borders, dividers, structural elements
- File type differentiation (unless indicating errors)
- Syntax highlighting for aesthetics
- Category labels without status meaning
- Navigation elements
- Decorative accents

**Real Examples:**

✓ btop CPU gradient: low (comment) → moderate (warning) → high (danger)
  - Semantic: User needs to know resource utilization levels

✓ git diff: added (success) / removed (danger) / modified (warning)
  - Semantic: Indicates type of change

✗ btop box borders: all use foreground-border
  - Not semantic: Just organizational structure

✗ File listing colors by type: use monochromatic
  - Not semantic: Type differentiation is informational, not status
