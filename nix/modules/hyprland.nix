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

  # Apply mkDefault to all leaf values so mynixos/user can override
  mkDefaultAttrs = attrs:
    lib.mapAttrsRecursive (_path: mkDefault) attrs;

in
{
  config = lib.mkMerge [
    # Warn if vogix is enabled but Hyprland is not
    (mkIf (cfg.enable && !(config.wayland.windowManager.hyprland.enable or false)) {
      warnings = [
        "programs.vogix is enabled but wayland.windowManager.hyprland is not — appearance/behavior settings will not be applied"
      ];
    })

    (mkIf (cfg.enable && (config.wayland.windowManager.hyprland.enable or false)) {
      wayland.windowManager.hyprland = {
        settings = lib.mkMerge [
          (mkDefaultAttrs appearance.settings)
          (mkDefaultAttrs behavior.settings)
        ];
        extraConfig = mkAfter (behavior.extraConfig or "");
      };
    })
  ];
}
