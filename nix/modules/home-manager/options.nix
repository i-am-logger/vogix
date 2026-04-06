# Module options for Vogix Home Manager module
#
# Defines all programs.vogix.* options
{ lib, pkgs }:

let
  inherit (lib)
    mkOption
    mkEnableOption
    types
    literalExpression
    ;

  # Import the vogix package
  vogix = pkgs.callPackage ../../packages/vogix.nix { };

  # Import shared application discovery
  appDiscovery = import ../lib/applications.nix { inherit lib; };
  inherit (appDiscovery) availableApps;

  # Import behavior options
  behaviorOptions = import ../behavior/options.nix { inherit lib; };

  # Per-app options (dynamically generated)
  appOptions = lib.listToAttrs (
    map
      (appName: {
        name = appName;
        value = {
          enable = mkOption {
            type = types.bool;
            default = true;
            description = "Enable vogix theming for ${appName}";
          };

          theme = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Theme to use for ${appName} (overrides global theme)";
          };

          variant = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Variant to use for ${appName} (overrides global variant)";
          };
        };
      })
      availableApps
  );

in
{
  options.programs.vogix = {
    enable = mkEnableOption "vogix runtime theme management";

    package = mkOption {
      type = types.package;
      default = vogix;
      defaultText = literalExpression "pkgs.vogix";
      description = "The vogix package to use.";
    };

    scheme = mkOption {
      type = types.str;
      default = "vogix16";
      description = "Color scheme to use (vogix16, base16, base24, ansi16).";
    };

    theme = mkOption {
      type = types.str;
      default = "aikido";
      description = "Theme to use.";
    };

    variant = mkOption {
      type = types.str;
      default = "night";
      description = "Variant name (e.g., night, day, dark, light, moon, dawn).";
    };

    themes = mkOption {
      type = types.attrsOf types.path;
      default = { };
      example = literalExpression ''
        {
          aikido = ./themes/aikido.nix;
          synthwave = ./themes/synthwave.nix;
        }
      '';
      description = "Custom theme definitions.";
    };

    appearance = {
      shader = {
        enable = mkOption {
          type = types.bool;
          default = false;
          description = ''
            Enable monochromatic screen shader. Auto-generates a Hyprland
            screen_shader from the theme's base00-07 palette hue.
            Applied automatically on every theme change.
          '';
        };

        intensity = mkOption {
          type = types.float;
          default = 0.7;
          description = "Blend intensity between original and monochrome [0.0..1.0].";
        };

        brightness = mkOption {
          type = types.float;
          default = 1.0;
          description = "Output brightness multiplier [0.1..2.0].";
        };

        saturation = mkOption {
          type = types.float;
          default = 1.0;
          description = "Color saturation adjustment [0.0..2.0].";
        };
      };
    };

    enableDaemon = mkOption {
      type = types.bool;
      default = false;
      description = "Enable the vogix daemon for auto-regeneration.";
    };

    colors = mkOption {
      type = types.attrsOf types.str;
      internal = true;
      description = "Semantic color API for the selected theme and variant. Used by application modules.";
    };
  }
  // appOptions
  // behaviorOptions;

  # Export for use by other modules
  inherit availableApps vogix;
}
