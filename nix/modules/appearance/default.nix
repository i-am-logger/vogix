# Vogix appearance module
#
# Generates Hyprland appearance settings from programs.vogix.appearance.*
# Consumed by the home-manager module.
{ lib }:

let
  defaults = import ./defaults.nix { };
  optionsModule = import ./options.nix { inherit lib; };

  # Generate Hyprland config from appearance settings
  # Returns { settings, extraConfig } — same shape as behavior module
  mkHyprlandConfig = acfg: {
    settings = {
      animations = {
        enabled = acfg.animations.enable or defaults.animations.enable;
        bezier = acfg.animations.bezier or defaults.animations.bezier;
        animation = acfg.animations.rules or defaults.animations.rules;
      };

      general = {
        gaps_in = acfg.gaps.inner or defaults.gaps.inner;
        gaps_out = acfg.gaps.outer or defaults.gaps.outer;
        border_size = acfg.borderSize or defaults.borderSize;
      };

      decoration = {
        active_opacity = acfg.decoration.activeOpacity or defaults.decoration.activeOpacity;
        inactive_opacity = acfg.decoration.inactiveOpacity or defaults.decoration.inactiveOpacity;
        fullscreen_opacity = acfg.decoration.fullscreenOpacity or defaults.decoration.fullscreenOpacity;
        rounding = acfg.decoration.rounding or defaults.decoration.rounding;
        dim_inactive = acfg.decoration.dimInactive or defaults.decoration.dimInactive;
        dim_strength = acfg.decoration.dimStrength or defaults.decoration.dimStrength;

        blur = {
          enabled = acfg.blur.enable or defaults.blur.enable;
          size = acfg.blur.size or defaults.blur.size;
          brightness = acfg.blur.brightness or defaults.blur.brightness;
        };
      };

      group = {
        groupbar = {
          font_family = acfg.group.fontFamily or defaults.group.fontFamily;
          font_size = acfg.group.fontSize or defaults.group.fontSize;
          height = acfg.group.height or defaults.group.height;
          indicator_height = acfg.group.indicatorHeight or defaults.group.indicatorHeight;
        };
      };
    };

    extraConfig = "";
  };

in
{
  inherit optionsModule defaults mkHyprlandConfig;
}
