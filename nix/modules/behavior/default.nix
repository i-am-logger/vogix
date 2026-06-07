# Vogix behavior module
#
# Wires behavior options to generators (hyprland, help).
# Imported by the home-manager module.
{ lib, pkgs ? null }:

let
  defaults = import ./defaults.nix { };
  hyprlandGen = import ./generators/hyprland.nix { inherit lib; };
  helpGen =
    if pkgs != null
    then import ./generators/help.nix { inherit lib pkgs; }
    else null;
  optionsModule = import ./options.nix { inherit lib; };

  # Build the flat config that generators expect from the new nested structure
  # Merge with defaults: {} means "use default", not "empty"
  mergeOr = user: default:
    if user == { } || user == null then default
    else lib.recursiveUpdate default user;

  mkGeneratorConfig = behaviorCfg:
    let
      kb = behaviorCfg.keybindings or { };
      userModes = behaviorCfg.modes or { };
      modeGraph = behaviorCfg.modeGraph or defaults.modeGraph;
    in
    {
      modKey = kb.modKey or defaults.keybindings.modKey;
      inherit modeGraph;
      modes = lib.mapAttrs
        (name: _:
          mergeOr (userModes.${name} or { }) (defaults.modes.${name} or { })
        )
        modeGraph.modes;
      mouse = mergeOr (kb.mouse or { }) defaults.keybindings.mouse;
      layers = mergeOr (kb.layers or { }) defaults.keybindings.layers;
      modeColors = userModes.modeColors or { };
      input = mergeOr (behaviorCfg.input or { }) defaults.input;
      touchpad = mergeOr (behaviorCfg.touchpad or { }) defaults.touchpad;
      layout = behaviorCfg.layout or defaults.layout;
      layouts = mergeOr (behaviorCfg.layouts or { }) defaults.layouts;
      misc = mergeOr (behaviorCfg.misc or { }) defaults.misc;
      gestures = mergeOr (behaviorCfg.gestures or { }) defaults.gestures;
    };
in
{
  inherit optionsModule defaults mkGeneratorConfig;

  # Generate Hyprland config
  mkHyprlandConfig = behaviorCfg:
    hyprlandGen.generate (mkGeneratorConfig behaviorCfg);

  # Generate per-mode help scripts
  mkHelpScripts = behaviorCfg:
    if helpGen != null then
      helpGen.mkAllHelpScripts ((mkGeneratorConfig behaviorCfg).modes or { })
    else
      { };

  mkGlobalHelpScript = behaviorCfg:
    if helpGen != null then
      helpGen.mkGlobalHelpScript ((mkGeneratorConfig behaviorCfg).modes or { })
    else
      null;

  # Render the behavior config to the JSON shape `src/input/schema.rs` expects.
  # The Rust side reads this via `Schema::load()` from
  # `~/.local/state/vogix/input.json`. The top-level keys mirror defaults.nix
  # 1:1 (`modeGraph`, `modes`, `keybindings`) — the Rust struct's
  # `#[serde(rename)]` lines were written against that layout. The Super→Ctrl
  # remap set is selected by `keybindings.paradigm` (a praxis preset), not a
  # listed table.
  #
  # We iterate `modeGraph.modes` (not `behaviorCfg.modes`) so every declared
  # mode lands in the schema even if only some are exposed as user options.
  mkSchemaJSON = behaviorCfg:
    let
      userModes = behaviorCfg.modes or { };
      modeGraph = behaviorCfg.modeGraph or defaults.modeGraph;
      effectiveKeybindings = mergeOr
        (behaviorCfg.keybindings or { })
        defaults.keybindings;
      # Resolve the selected interaction PARADIGM (whole-WM flavour) into the
      # engine's two inputs: the per-mode base bindings and the Super remap name.
      # The user's `behavior.modes` overlays on top of the paradigm's modes.
      paradigmName = effectiveKeybindings.paradigm or "vim";
      paradigms = effectiveKeybindings.paradigms or { };
      # Fail loudly on an unknown paradigm — a silent fallback would ship an empty
      # binding set (no desktop nav), the exact footgun the seam must avoid.
      selected = paradigms.${paradigmName} or (throw
        "vogix: unknown keybindings.paradigm \"${paradigmName}\" — known paradigms: ${toString (builtins.attrNames paradigms)}");
      paradigmModes = selected.modes or { };
      effectiveModes = lib.mapAttrs
        (name: _:
          mergeOr (userModes.${name} or { }) (paradigmModes.${name} or { })
        )
        modeGraph.modes;
      # The engine reads `keybindings.paradigm` as the REMAP name; the flavour's
      # remap fills it. `paradigms`/`paradigm` themselves aren't engine inputs.
      engineKeybindings =
        (builtins.removeAttrs effectiveKeybindings [ "paradigms" "paradigm" ])
        // { paradigm = selected.remap or "macos"; };
    in
    builtins.toJSON {
      inherit modeGraph;
      modes = effectiveModes;
      keybindings = engineKeybindings;
      # Top-level for the Rust `Schema.terminal_classes` (context-aware remap).
      terminalClasses = effectiveKeybindings.terminalClasses or [ ];
      # Per-mode border colours for the mode-visibility surface (engine paints
      # the border on a mode change). Theme-derived; set by the home-manager
      # module's modeColors block.
      modeColors = userModes.modeColors or { };
    };
}
