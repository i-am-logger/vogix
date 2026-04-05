# Help script generator
#
# Generates per-mode help scripts that display available keybindings.
# Each mode gets a script: vogix-modes-<mode>
# Pressing ? in any mode calls the script, which shows bindings via notify-send
# or a walker menu if available.
{ lib, pkgs }:

let
  inherit (lib)
    concatStringsSep
    mapAttrsToList
    filterAttrs
    ;

  # Format a key combo for display (strip modKey references, capitalize modifiers)
  formatKey = key:
    let
      cleaned = builtins.replaceStrings [ "modKey + " "super + " ] [ "Super+" "Super+" ] key;
    in
    lib.toUpper cleaned;

  # Generate a single mode's help text as aligned columns
  mkModeHelpText = _modeName: mode:
    let
      bindings = mode.bindings or { };
      # Filter out bindings without descriptions
      described = filterAttrs (_: b: (b.description or "") != "") bindings;
      lines = mapAttrsToList
        (_name: binding:
          let
            key = formatKey binding.key;
            desc = binding.description;
            # Pad key to 20 chars for alignment
            padded = key + builtins.substring 0 (20 - builtins.stringLength key) "                    ";
          in
          "${padded} ${desc}"
        )
        described;
      sorted = builtins.sort (a: b: a < b) lines;
    in
    concatStringsSep "\n" sorted;

  # Generate a help script for a mode
  mkHelpScript = modeName: mode:
    let
      helpText = mkModeHelpText modeName mode;
      title = lib.toUpper (builtins.substring 0 1 modeName) + builtins.substring 1 (builtins.stringLength modeName - 1) modeName;
    in
    pkgs.writeShellScriptBin "vogix-modes-${modeName}" ''
      # Vogix keybinding help: ${modeName} mode
      # Auto-generated — do not edit

      HELP_TEXT="${helpText}"

      # Try walker first (searchable), fall back to notify-send
      if command -v walker &>/dev/null; then
        echo "$HELP_TEXT" | walker --dmenu -p "${title} Mode Keybindings"
      else
        notify-send -t 10000 "${title} Mode" "$HELP_TEXT"
      fi
    '';

  # Generate all help scripts for all modes
  mkAllHelpScripts = modes:
    lib.mapAttrs mkHelpScript (filterAttrs (name: _: name != "normal") modes);

  # Generate help script for normal/global mode
  mkGlobalHelpScript = modes:
    mkHelpScript "global" (modes.normal or { bindings = { }; });

in
{
  inherit mkHelpScript mkAllHelpScripts mkGlobalHelpScript;
}
