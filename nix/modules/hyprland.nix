# Vogix Hyprland module
#
# Generates the full Hyprland appearance + behavior config.
# Uses mkDefault so mynixos or the user can override any setting.
#
# Branches on `wayland.windowManager.hyprland.configType`: the hyprlang
# projection emits legacy sections/bind strings; the Lua projection emits the
# HM Lua shapes (`settings.config` → `hl.config`, `settings.bind` elements →
# `hl.bind(keys, hl.dsp.*, opts)`, passthrough submaps → `submaps.<name>`).
# Same resolved configuration either way — only the rendering differs.
{ config
, lib
, ...
}:

let
  inherit (lib) mkIf mkDefault mkAfter;

  cfg = config.programs.vogix;
  acfg = cfg.appearance;
  behaviorCfg = cfg.behavior;

  isLua = (config.wayland.windowManager.hyprland.configType or "hyprlang") == "lua";

  # Import modules
  appearanceModule = import ./appearance { inherit lib; };
  behaviorModule = import ./behavior { inherit lib; };

  # Generate configs — hyprlang returns { settings, extraConfig }, Lua
  # returns { settings, submaps }
  appearance =
    if isLua
    then appearanceModule.mkHyprlandLuaConfig acfg
    else appearanceModule.mkHyprlandConfig acfg;
  behavior =
    if isLua
    then behaviorModule.mkHyprlandLuaConfig behaviorCfg
    else behaviorModule.mkHyprlandConfig behaviorCfg;

  # Apply mkDefault to all leaf values so mynixos/user can override. Lists
  # (bind, curve, animation, window_rule, …) are leaves — a whole list is
  # overridden or kept; the attrset trees (hl.config values) stay
  # per-key overridable.
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
        submaps = lib.mapAttrs (_: mkDefault) (behavior.submaps or { });
      };
    })
  ];
}
