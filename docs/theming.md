# Theme Format and Structure

Vogix supports multiple color schemes and uses TOML-based theme definitions with application generators to ensure consistency across applications.

## Color Schemes

Vogix supports 4 color schemes, each with its own philosophy:

| Scheme | Philosophy | Source |
|--------|------------|--------|
| **vogix16** | Semantic design system - colors convey functional meaning | [vogix16-themes](https://github.com/i-am-logger/vogix16-themes) (TOML format) |
| **base16** | Syntax highlighting - 16 colors for code categories | [tinted-schemes](https://github.com/i-am-logger/tinted-schemes) |
| **base24** | Extended base16 with 8 additional bright colors | [tinted-schemes](https://github.com/i-am-logger/tinted-schemes) |
| **ansi16** | Terminal standard - traditional ANSI color mappings | [iTerm2-Color-Schemes](https://github.com/i-am-logger/iTerm2-Color-Schemes) |

For detailed information about the vogix16 design system, see the [vogix16-themes design system documentation](https://github.com/i-am-logger/vogix16-themes/blob/main/docs/design-system.md).

## Theme Definition Format

### vogix16 Theme Format (TOML)

Themes in the vogix16-themes repository use TOML format with one file per variant:

```
vogix16-themes/
├── yoga/           # Western convention: success green, danger red
│   ├── day.toml    # light variant
│   └── night.toml  # dark variant
├── aikido/         # Japanese convention: success red, danger green
│   ├── day.toml
│   └── night.toml
└── ...
```

Each variant file:

```toml
# vogix16-themes/yoga/night.toml — a Western theme (success green, danger red)
polarity = "dark"

[colors]
# Monochromatic ramp (background → foreground); single-hue by design
base00 = "#262626"  # background
base01 = "#333333"  # background_surface
base02 = "#3b3028"  # background_selection
base03 = "#54433a"  # foreground_comment
base04 = "#6c5d53"  # foreground_border
base05 = "#a29990"  # foreground_text
base06 = "#cbc3bc"  # foreground_heading
base07 = "#f6f5f0"  # foreground_bright

# Functional accents. The SLOT fixes the role (base08 is ALWAYS success); the
# HUE is the theme's editorial choice — Western themes paint success green and
# danger red, while a Japanese theme (aikido) does the reverse. Same roles,
# different hues. See the vogix16 design-system doc.
base08 = "#4d5645"  # success
base09 = "#835538"  # warning
base0A = "#bfa46f"  # notice
base0B = "#d7503c"  # danger
base0C = "#8694a8"  # active
base0D = "#658fbd"  # link
base0E = "#896ea4"  # highlight
base0F = "#7a5c42"  # special
```

### Theme Structure

| Field | Required | Description |
|-------|----------|-------------|
| `polarity` | Yes | `"dark"` or `"light"` |
| `[colors]` | Yes | Section containing base00-base0F color definitions |

### Automatic Variant Ordering

Variants are automatically ordered by luminance (lightest to darkest) based on the `base00` (background) color. This enables the `vogix -v darker` and `vogix -v lighter` navigation commands.

For catppuccin, the auto-derived order is: `latte → frappe → macchiato → mocha`

### Single-Variant Themes

Themes with only one variant (like dracula) work correctly:

```
themes/
└── dracula/
    └── dracula.toml
```

All navigation commands (`-v dark`, `-v light`, `-v darker`, `-v lighter`) resolve to the only available variant.

## Imported Themes

### base16 and base24

Themes are imported from YAML files in the [tinted-schemes](https://github.com/i-am-logger/tinted-schemes) repository:

```
tinted-schemes/
├── base16/
│   ├── catppuccin/
│   │   ├── latte.yaml
│   │   ├── frappe.yaml
│   │   ├── macchiato.yaml
│   │   └── mocha.yaml
│   ├── dracula/
│   │   └── dracula.yaml
│   └── ...
└── base24/
    └── ...
```

Directory name = theme name, file name = variant name.

### ansi16

Themes are imported from TOML files in the [iTerm2-Color-Schemes](https://github.com/i-am-logger/iTerm2-Color-Schemes) repository.

## Application Generators

Application generators convert theme colors into application-specific configuration files. They support all 4 schemes through pattern matching:

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
      # Semantic color usage
      colors.primary.background = colors.background;
      colors.primary.foreground = colors.foreground-text;
      colors.normal.red = colors.danger;
    };
    
    base16 = colors: {
      # Base16 standard mapping
      colors.primary.background = colors.base00;
      colors.primary.foreground = colors.base05;
      colors.normal.red = colors.base08;
    };
    
    base24 = colors: {
      # Base24 with bright colors
      colors.primary.background = colors.base00;
      colors.primary.foreground = colors.base05;
      colors.bright.red = colors.base12;
    };
    
    ansi16 = colors: {
      # ANSI standard mapping
      colors.primary.background = colors.background;
      colors.primary.foreground = colors.foreground;
      colors.normal.red = colors.red;
    };
  };
}
```

See [docs/app-module-template.nix](app-module-template.nix) for a complete application module template.

## Theme Processing (Build Time)

Theme processing happens entirely at **Nix build time**, not at runtime:

1. **Discovery**: 
   - Native themes from [vogix16-themes](https://github.com/i-am-logger/vogix16-themes) (TOML files)
   - Imported themes from forked repos (base16, base24, ansi16)

2. **Normalization**: All themes converted to internal format

3. **Generation**: For each (scheme, theme, variant, app) combination:
   - Load theme definition
   - Select the appropriate scheme generator
   - Apply generator to produce config file
   - Write to Nix store

4. **Symlinks**: Home-manager creates symlinks to `~/.local/share/vogix/themes/`

## Directory Structure

```
/nix/store/
├── xxxx-vogix-base16-catppuccin-mocha/
│   ├── alacritty/colors.toml
│   ├── btop/themes/vogix.theme
│   └── console/palette
├── yyyy-vogix-base16-catppuccin-latte/
├── zzzz-vogix-vogix16-yoga-dark/
└── ...

~/.local/share/vogix/
└── themes/
    ├── base16-catppuccin-mocha -> /nix/store/xxxx-...
    ├── base16-catppuccin-latte -> /nix/store/yyyy-...
    ├── vogix16-yoga-dark -> /nix/store/zzzz-...
    └── ...

~/.local/state/vogix/
├── current-theme -> ~/.local/share/vogix/themes/base16-catppuccin-mocha
└── state.toml

~/.config/
├── alacritty/colors.toml -> ~/.local/state/vogix/current-theme/alacritty/colors.toml
└── ...
```

## Adding New Themes

### Native vogix16 Theme

Themes are maintained in the [vogix16-themes](https://github.com/i-am-logger/vogix16-themes) repository.

1. **Clone the themes repo**:
   ```bash
   git clone https://github.com/i-am-logger/vogix16-themes
   cd vogix16-themes
   ```

2. **Create theme directory**:
   ```bash
   mkdir themes/my-theme
   ```

3. **Create variant files**:
   ```toml
   # themes/my-theme/dark.toml
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

   ```toml
   # themes/my-theme/light.toml
   polarity = "light"
   
   [colors]
   base00 = "#f8f8f8"
   # ... (inverted monochromatic scale)
   ```

4. **Validate your theme**:
   ```bash
   python scripts/validate-themes.py themes/my-theme
   ```

5. **Submit PR** to [vogix16-themes](https://github.com/i-am-logger/vogix16-themes)

### Contributing to Upstream

For base16/base24 themes, contribute to [tinted-schemes](https://github.com/tinted-theming/schemes).
For ansi16 themes, contribute to [iTerm2-Color-Schemes](https://github.com/mbadolato/iTerm2-Color-Schemes).

## Theme Validation

The system ensures at build time:

1. All required colors are defined (16 for base16/vogix16/ansi16, 24 for base24)
2. Color values are valid hex format (#RRGGBB)
3. Polarity is specified for each variant
4. Generators produce valid configs
