{ lib, ... }:

let
  # templates/<scheme>/console.palette.vogix, read as data: the one ANSI
  # mapping the runtime render, this generator and the NixOS console.colors
  # share.
  consolePalette = import ../lib/console-palette.nix { inherit lib; };

  # vogix16 reaches generators with kebab-case semantic names; the template
  # names them in snake_case, the runtime convention. Other schemes' names
  # carry no '-'.
  templateNames = lib.mapAttrs' (name: lib.nameValuePair (builtins.replaceStrings [ "-" ] [ "_" ] name));
in
{
  # Config file path relative to ~/.config/console/
  # Binary palette file for setvtrgb
  configFile = "palette";

  # Don't include metadata header (setvtrgb expects exactly 16 hex lines)
  includeHeader = false;

  # The reload writes the kernel's VT palette. Where the host's
  # vogix-machine owns that palette (programs.vogix.machineConsole), the app
  # is not reloaded; its palette file is still rendered.
  reloadWritesVtPalette = true;

  # Reload method: use setvtrgb command to load palette and switch VTs
  reloadMethod = {
    method = "command";
    # Use setvtrgb to load palette, then switch VTs to force refresh
    # Note: Requires security.wrappers from vogix NixOS module for non-root access
    # Only runs on actual VT consoles (not in PTY/SSH sessions)
    # Palette is at ~/.local/state/vogix/current-theme/console/palette
    command = "if [ -c /dev/console ] && fgconsole >/dev/null 2>&1; then setvtrgb \${XDG_STATE_HOME:-$HOME/.local/state}/vogix/current-theme/console/palette && { CURRENT_VT=$(fgconsole); NEXT_VT=$((CURRENT_VT % 6 + 1)); [ \"\$NEXT_VT\" = \"\$CURRENT_VT\" ] && NEXT_VT=1; chvt $NEXT_VT && sleep 0.05 && chvt $CURRENT_VT; }; fi";
  };

  # The palette for setvtrgb: 16 lines, ANSI 0-15, each a "#rrggbb" colour.
  schemes = lib.genAttrs (builtins.attrNames consolePalette.slots) (
    scheme: colors:
      lib.concatStringsSep "\n" (consolePalette.fromTemplateColors scheme (templateNames colors))
  );
}
