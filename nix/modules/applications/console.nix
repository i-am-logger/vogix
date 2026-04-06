{ lib, ... }:

{
  # Config file path relative to ~/.config/console/
  # Binary palette file for setvtrgb
  configFile = "palette";

  # Don't include metadata header (setvtrgb expects exactly 16 hex lines)
  includeHeader = false;

  # Reload method: use setvtrgb command to load palette and switch VTs
  reloadMethod = {
    method = "command";
    # Use setvtrgb to load palette, then switch VTs to force refresh
    # Note: Requires security.wrappers from vogix NixOS module for non-root access
    # Only runs on actual VT consoles (not in PTY/SSH sessions)
    # Palette is at ~/.local/state/vogix/current-theme/console/palette
    command = "if [ -c /dev/console ] && fgconsole >/dev/null 2>&1; then setvtrgb \${XDG_STATE_HOME:-$HOME/.local/state}/vogix/current-theme/console/palette && { CURRENT_VT=$(fgconsole); NEXT_VT=$((CURRENT_VT % 6 + 1)); [ \"\$NEXT_VT\" = \"\$CURRENT_VT\" ] && NEXT_VT=1; chvt $NEXT_VT && sleep 0.05 && chvt $CURRENT_VT; }; fi";
  };

  # Generators for each color scheme
  # Returns hexadecimal format palette for setvtrgb command
  # Format: 16 lines with hex colors (e.g., #000000)
  schemes = {
    # Vogix16: semantic color names mapped to ANSI
    vogix16 =
      colors:
      let
        palette = [
          colors.background # 0: Black
          colors.danger # 1: Red
          colors.success # 2: Green
          colors.warning # 3: Yellow
          colors.link # 4: Blue
          colors.highlight # 5: Magenta
          colors.active # 6: Cyan
          colors.foreground-text # 7: White
          colors.foreground-comment # 8: Bright Black
          colors.danger # 9: Bright Red
          colors.success # 10: Bright Green
          colors.warning # 11: Bright Yellow
          colors.link # 12: Bright Blue
          colors.highlight # 13: Bright Magenta
          colors.active # 14: Bright Cyan
          colors.foreground-bright # 15: Bright White
        ];
      in
      lib.concatStringsSep "\n" palette;

    # Base16: raw base00-base0F colors
    base16 =
      colors:
      let
        palette = [
          colors.base00 # 0: Black
          colors.base08 # 1: Red
          colors.base0B # 2: Green
          colors.base0A # 3: Yellow
          colors.base0D # 4: Blue
          colors.base0E # 5: Magenta
          colors.base0C # 6: Cyan
          colors.base05 # 7: White
          colors.base03 # 8: Bright Black
          colors.base08 # 9: Bright Red (same as normal in base16)
          colors.base0B # 10: Bright Green
          colors.base0A # 11: Bright Yellow
          colors.base0D # 12: Bright Blue
          colors.base0E # 13: Bright Magenta
          colors.base0C # 14: Bright Cyan
          colors.base07 # 15: Bright White
        ];
      in
      lib.concatStringsSep "\n" palette;

    # Base24: base00-base17 with true bright colors
    base24 =
      colors:
      let
        palette = [
          colors.base00 # 0: Black
          colors.base08 # 1: Red
          colors.base0B # 2: Green
          colors.base0A # 3: Yellow
          colors.base0D # 4: Blue
          colors.base0E # 5: Magenta
          colors.base0C # 6: Cyan
          colors.base05 # 7: White
          colors.base03 # 8: Bright Black
          colors.base12 # 9: Bright Red
          colors.base14 # 10: Bright Green
          colors.base13 # 11: Bright Yellow
          colors.base16 # 12: Bright Blue
          colors.base17 # 13: Bright Magenta
          colors.base15 # 14: Bright Cyan
          colors.base07 # 15: Bright White
        ];
      in
      lib.concatStringsSep "\n" palette;

    # ANSI16: direct terminal colors
    ansi16 =
      colors:
      let
        palette = [
          colors.color00 # 0: Black
          colors.color01 # 1: Red
          colors.color02 # 2: Green
          colors.color03 # 3: Yellow
          colors.color04 # 4: Blue
          colors.color05 # 5: Magenta
          colors.color06 # 6: Cyan
          colors.color07 # 7: White
          colors.color08 # 8: Bright Black
          colors.color09 # 9: Bright Red
          colors.color10 # 10: Bright Green
          colors.color11 # 11: Bright Yellow
          colors.color12 # 12: Bright Blue
          colors.color13 # 13: Bright Magenta
          colors.color14 # 14: Bright Cyan
          colors.color15 # 15: Bright White
        ];
      in
      lib.concatStringsSep "\n" palette;
  };
}
