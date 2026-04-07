# Vogix Hyprland module
#
# Generates the full Hyprland appearance + behavior config.
# Uses mkDefault so mynixos or the user can override any setting.
{ config
, lib
, ...
}:

let
  inherit (lib) mkIf mkDefault mkAfter;

  cfg = config.programs.vogix;
  acfg = cfg.appearance;
  behaviorCfg = cfg.behavior;

  # Import modules
  appearanceModule = import ./appearance { inherit lib; };
  behaviorModule = import ./behavior { inherit lib; };

  # Generate configs — both return { settings, extraConfig }
  appearance = appearanceModule.mkHyprlandConfig acfg;
  behavior = behaviorModule.mkHyprlandConfig behaviorCfg;

  # Apply mkDefault recursively to all leaf values
  mkDefaultAttrs = attrs:
    lib.mapAttrsRecursive
      (_path: value:
        if builtins.isList value then mkDefault value
        else if builtins.isAttrs value then value
        else mkDefault value
      )
      attrs;

in
{
  config = mkIf (cfg.enable && (config.wayland.windowManager.hyprland.enable or false)) {
    wayland.windowManager.hyprland = {
      settings = lib.mkMerge [
        (mkDefaultAttrs appearance.settings)
        (mkDefaultAttrs behavior.settings)
      ];
      extraConfig = mkAfter (behavior.extraConfig or "");
    };
  };
}
