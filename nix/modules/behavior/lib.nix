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

  # Recursively peel modifier prefixes (C-, S-, A-, M-) off a key name.
  # Returns { mods = [ "lctl" "lsft" ... ]; base = "h"; } where mods preserves prefix order.
  # Example:  "C-S-A-h"  →  { mods = [ "lctl" "lsft" "lalt" ]; base = "h"; }
  peelModifiers = key:
    let
      lower = lib.toLower key;
      ctrl = lib.hasPrefix "c-" lower;
      shift = lib.hasPrefix "s-" lower;
      alt = lib.hasPrefix "a-" lower;
      meta = lib.hasPrefix "m-" lower;
      hasMod = ctrl || shift || alt || meta;
      modName =
        if ctrl then "lctl"
        else if shift then "lsft"
        else if alt then "lalt"
        else "lmet";
      rest = builtins.substring 2 (builtins.stringLength lower - 2) lower;
      sub = peelModifiers rest;
    in
    if hasMod then { mods = [ modName ] ++ sub.mods; inherit (sub) base; }
    else { mods = [ ]; base = lower; };

  # Convert a key name to kanata format.
  # Single keys map via kanataKeyMap (e.g. "left" → "left", "capslock" → "caps").
  # Compound keys peel ALL modifier prefixes and emit a single (multi …) chord:
  #   "C-right"      → "(multi lctl rght)"
  #   "C-S-A-h"      → "(multi lctl lsft lalt h)"  ← synthetic-Hyper for the WM layer
  toKanataKey = key:
    let
      peeled = peelModifiers key;
      resolvedBase = kanataKeyMap.${peeled.base} or peeled.base;
    in
    if peeled.mods == [ ]
    then kanataKeyMap.${lib.toLower key} or (lib.toLower key)
    else "(multi ${concatStringsSep " " (peeled.mods ++ [ resolvedBase ])})";

in
{
  inherit
    parseKeyCombo
    toHyprlandBind
    toKanataKey
    modifierMap
    ;
}
