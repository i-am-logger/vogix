# Vogix behavior module
#
# Wires behavior options to the Hyprland generator (the engine-off fallback).
# Help is no longer generated here: it is an ENGINE view (`vogix input keys`,
# materialized from the resolved schema) — the single-materializer architecture
# means nothing re-encodes a per-mode help script in Nix.
# Imported by the home-manager module.
#
# `pkgs` is accepted (and ignored via `...`) for call-site compatibility — the
# help generator that needed it is gone; only `lib` is used now.
{ lib, ... }:

let
  defaults = import ./defaults.nix { };
  hyprlandGen = import ./generators/hyprland.nix { inherit lib; };
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
      effectiveKeybindings = mergeOr
        (behaviorCfg.keybindings or { })
        defaults.keybindings;
      # Emit only the paradigm SELECTION NAME + the user's OVERLAY modes. The
      # engine resolves the name into the WM-nav modes (the paradigm's BindingSet,
      # e.g. vogix_nav_preset) + the mode graph, and merges the overlay on top
      # (see src/input/{catalog,schema}.rs). Resolution lives in the engine — a
      # paradigm is loaded once and every view (dispatch, help, Hyprland fallback)
      # is materialized from it; nothing re-encodes the nav here. We deliberately
      # do NOT emit `modeGraph`: its absence is what tells the engine to resolve.
      paradigmName = effectiveKeybindings.paradigm or "vogix";
      # The overlay = the user's own bindings (launch/system/media), `defaults.modes`
      # merged with `behaviorCfg.modes`. The paradigm NAV is NOT here.
      overlayModes = lib.mapAttrs
        (name: dm: mergeOr (userModes.${name} or { }) dm)
        defaults.modes;
      # `paradigm` carries the selection name to the engine; the legacy `paradigms`
      # catalog is gone (the engine owns the catalog now).
      engineKeybindings =
        (builtins.removeAttrs effectiveKeybindings [ "paradigms" ])
        // { paradigm = paradigmName; };
    in
    builtins.toJSON {
      modes = overlayModes;
      keybindings = engineKeybindings;
      # Top-level for the Rust `Schema.terminal_classes` (context-aware remap).
      terminalClasses = effectiveKeybindings.terminalClasses or [ ];
      # Per-mode border colours for the mode-visibility surface (engine paints
      # the border on a mode change). Theme-derived; set by the home-manager
      # module's modeColors block.
      modeColors = userModes.modeColors or { };
      # Top-level for the Rust `Schema.device_filter` — which devices the engine
      # may grab. Empty here = the engine's baked-in safe baseline applies (Yubico
      # / audio HID excluded); user entries EXTEND that baseline.
      deviceFilter = effectiveKeybindings.deviceFilter or { };
    };
}
