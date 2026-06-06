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

  # Default vogix package — use pkgs.vogix from overlay if available, else build from source
  vogix = pkgs.vogix or (pkgs.callPackage ../../packages/vogix.nix { });

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

    appearance = {
      scheme = mkOption {
        type = types.str;
        default = "vogix16";
        description = "Color scheme to use (vogix16, base16, base24, ansi16).";
      };

      theme = mkOption {
        type = types.str;
        default = "yoga";
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
            yoga = ./themes/yoga.nix;
            synthwave = ./themes/synthwave.nix;
          }
        '';
        description = "Custom theme definitions.";
      };

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
          default = 0.5;
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

    autoRestoreSession = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Whether the daemon re-spawns the saved desktop session on boot/login
        into an empty desktop. Auto-SAVE is unaffected by this — the session is
        always written to 'autosave'; this only controls the boot-time re-spawn.
        Set to false (renders VOGIX_AUTO_RESTORE=0 on the daemon unit) to start
        with a clean desktop each boot; `vogix session restore` stays available
        manually. Re-spawning a whole layout can surprise you with a stale
        snapshot and re-launch many apps at once.
      '';
    };

    logLevel = mkOption {
      type = types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = ''
        Log verbosity for the vogix systemd user services (the theme daemon and
        the input engine). Rendered as `RUST_LOG=vogix=<level>` on each unit so
        the output lands in journald — systemd user services do not inherit a
        shell's `RUST_LOG`, so it must be set on the unit. Raise to `debug` to
        make every keybinding decision (key in → mode → binding match/miss →
        dispatch result/uinput emit) and the daemon's startup environment
        observable via `journalctl --user -u vogix-input -u vogix-daemon`.
        `trace` additionally logs every key (including passthrough typing).
      '';
    };

    colors = mkOption {
      type = types.attrsOf types.str;
      internal = true;
      description = "Semantic color API for the selected theme and variant. Used by application modules.";
    };

    themeApply = mkOption {
      type = types.attrsOf types.str;
      default = { };
      description = "Hardware theme apply commands. Keys are device names, values are shell commands with {{color}} placeholders resolved at runtime.";
    };

  }
  // appOptions
  // behaviorOptions;

  # Export for use by other modules
  inherit availableApps vogix;
}
