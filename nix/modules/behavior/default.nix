# Vogix behavior module
#
# Wires behavior options to generators (kanata, hyprland, help).
# Imported by the home-manager module.
{ lib, pkgs ? null }:

let
  defaults = import ./defaults.nix { };
  hyprlandGen = import ./generators/hyprland.nix { inherit lib; };
  kanataGen = import ./generators/kanata.nix { inherit lib; };
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
      modes = behaviorCfg.modes or { };
    in
    {
      modKey = kb.modKey or defaults.keybindings.modKey;
      modes = {
        normal = mergeOr (modes.app or { }) defaults.modes.app;
        desktop = mergeOr (modes.desktop or { }) defaults.modes.desktop;
        arrange = mergeOr (modes.arrange or { }) defaults.modes.arrange;
        theme = mergeOr (modes.theme or { }) defaults.modes.theme;
      };
      mouse = mergeOr (kb.mouse or { }) defaults.keybindings.mouse;
      layers = mergeOr (kb.layers or { }) defaults.keybindings.layers;
      universal = defaults._superCtrlRemaps;
      modeColors = modes.modeColors or { };
    };
in
{
  inherit optionsModule defaults mkGeneratorConfig;

  # Generate Hyprland config
  mkHyprlandConfig = behaviorCfg:
    hyprlandGen.generate (mkGeneratorConfig behaviorCfg);

  # Generate kanata config
  mkKanataConfig = behaviorCfg:
    kanataGen.generate (mkGeneratorConfig behaviorCfg);

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
}
