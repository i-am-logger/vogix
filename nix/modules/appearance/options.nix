# Appearance options for Vogix
#
# Defines programs.vogix.appearance.* options for visual UX
# (animations, decoration, blur, gaps, group).
#
# Returns the BARE option attrset — home-manager/options.nix merges it into
# the `appearance` group with `//`, exactly like the behavior fragment. It
# used to return `{ options = { … }; }`, and that wrapper is why none of
# these options was ever actually declared: the merge site saw one attr
# named `options` and the Hyprland visual values stayed pinned to
# defaults.nix whatever the user configured.
{ lib }:

let
  inherit (lib) mkOption types;
  defaults = import ./defaults.nix { };
in
{
  animations = {
    enable = mkOption {
      type = types.bool;
      default = defaults.animations.enable;
      description = "Enable window animations.";
    };

    bezier = mkOption {
      type = types.str;
      default = defaults.animations.bezier;
      description = "Bezier curve definition for animations.";
    };

    rules = mkOption {
      type = types.listOf types.str;
      default = defaults.animations.rules;
      description = "Animation rules (name, onoff, speed, curve, style).";
    };
  };

  decoration = {
    activeOpacity = mkOption {
      type = types.float;
      default = defaults.decoration.activeOpacity;
      description = "Opacity for active windows [0.0..1.0].";
    };

    inactiveOpacity = mkOption {
      type = types.float;
      default = defaults.decoration.inactiveOpacity;
      description = "Opacity for inactive windows [0.0..1.0].";
    };

    fullscreenOpacity = mkOption {
      type = types.float;
      default = defaults.decoration.fullscreenOpacity;
      description = "Opacity for fullscreen windows [0.0..1.0].";
    };

    rounding = mkOption {
      type = types.int;
      default = defaults.decoration.rounding;
      description = "Corner rounding radius in pixels.";
    };

    dimInactive = mkOption {
      type = types.bool;
      default = defaults.decoration.dimInactive;
      description = "Dim inactive windows.";
    };

    dimStrength = mkOption {
      type = types.float;
      default = defaults.decoration.dimStrength;
      description = "Dim strength for inactive windows [0.0..1.0].";
    };
  };

  blur = {
    enable = mkOption {
      type = types.bool;
      default = defaults.blur.enable;
      description = "Enable background blur.";
    };

    size = mkOption {
      type = types.int;
      default = defaults.blur.size;
      description = "Blur radius.";
    };

    brightness = mkOption {
      type = types.float;
      default = defaults.blur.brightness;
      description = "Blur brightness [0.0..2.0].";
    };
  };

  gaps = {
    inner = mkOption {
      type = types.int;
      default = defaults.gaps.inner;
      description = "Gap between windows in pixels.";
    };

    outer = mkOption {
      type = types.int;
      default = defaults.gaps.outer;
      description = "Gap between windows and screen edge in pixels.";
    };
  };

  borderSize = mkOption {
    type = types.int;
    default = defaults.borderSize;
    description = "Window border size in pixels.";
  };

  # Curated backgrounds merged onto each variant's always-present generated
  # one, keyed <theme>.<variant>. This is the drop-in point for the
  # vogix-backgrounds data repo (and for host-local additions); every entry
  # in that repo passes its SOURCES.toml licence gate before it can appear
  # here. Store-path entries only — the theme package symlinks them under
  # vogix-desktop/backgrounds/ and lists them in backgrounds.json.
  extraBackgrounds = mkOption {
    type = types.attrsOf (types.attrsOf (types.listOf (types.submodule {
      options = {
        kind = mkOption {
          type = types.enum [ "image" "video" "shader" "generated" ];
          description = "How the shell renders this entry.";
        };
        name = mkOption {
          type = types.str;
          description = "File name under vogix-desktop/backgrounds/ (unique per variant).";
        };
        path = mkOption {
          type = types.path;
          description = "The asset (a standalone derivation or data-repo file).";
        };
        attribution = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Verbatim credit line, for licences that require one.";
        };
      };
    })));
    default = { };
    description = "Extra backgrounds per theme and variant.";
  };

  group = {
    fontFamily = mkOption {
      type = types.str;
      default = defaults.group.fontFamily;
      description = "Font family for grouped window bar.";
    };

    fontSize = mkOption {
      type = types.int;
      default = defaults.group.fontSize;
      description = "Font size for grouped window bar.";
    };

    height = mkOption {
      type = types.int;
      default = defaults.group.height;
      description = "Height of grouped window bar in pixels.";
    };

    indicatorHeight = mkOption {
      type = types.int;
      default = defaults.group.indicatorHeight;
      description = "Height of active indicator in grouped window bar.";
    };
  };
}
