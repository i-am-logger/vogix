# Shared keybinding utilities
#
# Helpers for parsing key combos and resolving modKey references
{ lib }:

let
  inherit (lib)
    concatStringsSep
    splitString
    toUpper
    trim
    ;

  # Map friendly modifier names to Hyprland modifier names
  modifierMap = {
    "super" = "SUPER";
    "alt" = "ALT";
    "ctrl" = "CTRL";
    "shift" = "SHIFT";
    "meta" = "SUPER";
    "modkey" = null; # resolved dynamically
  };

  # Parse a key combo string like "modKey + shift + h" into { mods, key }
  # Returns: { mods = "SUPER SHIFT"; key = "h"; }
  parseKeyCombo = modKey: combo:
    let
      parts = map trim (splitString "+" combo);
      resolved = map
        (part:
          let lower = lib.toLower part;
          in
          if lower == "modkey" then toUpper modKey
          else modifierMap.${lower} or null  # not a modifier, this is the key
        )
        parts;
      mods = builtins.filter (x: x != null) (lib.init resolved);
      key = lib.last parts;
      # Check if the last part is actually a modifier (shouldn't be, but handle gracefully)
      keyResolved =
        let lower = lib.toLower key;
        in if modifierMap ? ${lower} && lower != "modkey" then toUpper key else key;
    in
    {
      mods = concatStringsSep " " mods;
      key = trim keyResolved;
    };

  # Convert a parsed key combo to Hyprland bind format: "MODS, key"
  toHyprlandBind = modKey: combo:
    let parsed = parseKeyCombo modKey combo;
    in "${parsed.mods}, ${parsed.key}";

  # Map kanata key names to their kanata config representation
  kanataKeyMap = {
    "capslock" = "caps";
    "escape" = "esc";
    "left" = "left";
    "right" = "rght";
    "up" = "up";
    "down" = "down";
    "pageup" = "pgup";
    "pagedown" = "pgdn";
    "home" = "home";
    "end" = "end";
    "insert" = "ins";
    "delete" = "del";
    "backspace" = "bspc";
    "tab" = "tab";
    "enter" = "ret";
    "space" = "spc";
  };

  # Convert a key name to kanata format
  # Handles compound keys like "C-right" → "(multi lctl rght)"
  toKanataKey = key:
    let
      lower = lib.toLower key;
      # Check for modifier prefix (C-, S-, A-, M-)
      hasCtrl = lib.hasPrefix "c-" lower;
      hasShift = lib.hasPrefix "s-" lower;
      hasAlt = lib.hasPrefix "a-" lower;
      hasMeta = lib.hasPrefix "m-" lower;
      hasModifier = hasCtrl || hasShift || hasAlt || hasMeta;
      modifier =
        if hasCtrl then "lctl"
        else if hasShift then "lsft"
        else if hasAlt then "lalt"
        else if hasMeta then "lmet"
        else "";
      baseKey = if hasModifier then builtins.substring 2 (builtins.stringLength lower - 2) lower else lower;
      resolvedBase = kanataKeyMap.${baseKey} or baseKey;
    in
    if hasModifier then "(multi ${modifier} ${resolvedBase})"
    else kanataKeyMap.${lower} or lower;

in
{
  inherit
    parseKeyCombo
    toHyprlandBind
    toKanataKey
    modifierMap
    ;
}
