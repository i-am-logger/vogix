# NixOS module for Vogix
#
# Provides system-level integration:
# - Console colors (TTY) from vogix theme
# - Security wrappers for console theme switching (chvt, setvtrgb)
#
# NOTE: User configuration (config.toml, app configs) is handled by
# the home-manager module at ~/.local/state/vogix/
{ vogix16Themes
,
}:

{ config
, lib
, pkgs
, options
, ...
}:

let
  inherit (lib)
    mkIf
    mkMerge
    mkOption
    mkEnableOption
    types
    literalExpression
    attrNames
    filterAttrs
    ;

  cfg = config.vogix;

  # Behavior module for kanata config generation
  behaviorModule = import ./behavior { inherit lib; };
  kanataConfig = behaviorModule.mkKanataConfig { };

  # Import vogix16 themes for console colors
  vogix16Import = import ./vogix16-import.nix {
    inherit lib vogix16Themes;
  };

  # Find home-manager users with vogix enabled (for auto-detection)
  homeManagerUsers =
    if options ? home-manager then
      attrNames
        (
          filterAttrs (_name: userCfg: userCfg.programs.vogix.enable or false) (
            config.home-manager.users or { }
          )
        )
    else
      [ ];

  firstVogixUser = if homeManagerUsers != [ ] then builtins.head homeManagerUsers else null;

  # Get vogix config from first user for console colors auto-detection
  hmVogixCfg =
    if firstVogixUser != null then config.home-manager.users.${firstVogixUser}.programs.vogix else null;

  # Helper: Convert theme colors (base16 format) to console.colors array
  mkConsoleColors =
    themeColors:
    map (c: builtins.replaceStrings [ "#" ] [ "" ] c) [
      themeColors.base00 # black (ANSI 0)
      themeColors.base08 # red (ANSI 1)
      themeColors.base0B # green (ANSI 2)
      themeColors.base0A # yellow (ANSI 3)
      themeColors.base0D # blue (ANSI 4)
      themeColors.base0E # magenta (ANSI 5)
      themeColors.base0C # cyan (ANSI 6)
      themeColors.base05 # white (ANSI 7)
      themeColors.base03 # bright black (ANSI 8)
      themeColors.base08 # bright red (ANSI 9)
      themeColors.base0B # bright green (ANSI 10)
      themeColors.base0A # bright yellow (ANSI 11)
      themeColors.base0D # bright blue (ANSI 12)
      themeColors.base0E # bright magenta (ANSI 13)
      themeColors.base0C # bright cyan (ANSI 14)
      themeColors.base07 # bright white (ANSI 15)
    ];
in
{
  options.vogix = {
    enable = mkEnableOption "vogix theme management";

    autoFromHomeManager = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Automatically configure console colors from home-manager vogix configuration.
        When enabled, will use the theme from the first home-manager user
        with programs.vogix.enable = true.
      '';
    };

    theme = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to theme file for console colors (overrides auto-detection).";
      example = literalExpression "./themes/aikido.nix";
    };

    variant = mkOption {
      type = types.nullOr (
        types.enum [
          "dark"
          "light"
        ]
      );
      default = null;
      description = "Theme variant (dark or light) for console colors (overrides auto-detection).";
    };
  };

  config = mkIf cfg.enable (mkMerge [
    {
      # Make vogix CLI + dependencies available system-wide
      environment.systemPackages = [
        pkgs.vogix
        pkgs.tmux # Required for F12 system console
      ];

      # Add security wrappers for console theme switching
      security.wrappers = {
        chvt = {
          owner = "root";
          group = "root";
          capabilities = "cap_sys_tty_config+ep";
          source = "${pkgs.kbd}/bin/chvt";
        };
        setvtrgb = {
          owner = "root";
          group = "root";
          capabilities = "cap_sys_tty_config+ep";
          source = "${pkgs.kbd}/bin/setvtrgb";
        };
      };
    }

    # Auto-detect console colors from home-manager if enabled
    (mkIf (cfg.autoFromHomeManager && cfg.theme == null && cfg.variant == null) {
      console.colors =
        let
          selectedThemeName = if hmVogixCfg != null then hmVogixCfg.theme else null;
          selectedPolarity = if hmVogixCfg != null then hmVogixCfg.variant else null;

          loadedTheme =
            if selectedThemeName != null && vogix16Import.themes ? ${selectedThemeName} then
              vogix16Import.themes.${selectedThemeName}
            else
              null;

          selectedVariantName =
            if loadedTheme != null then loadedTheme.defaults.${selectedPolarity} or selectedPolarity else null;

          themeColors =
            if loadedTheme != null && selectedVariantName != null then
              loadedTheme.variants.${selectedVariantName}.colors or null
            else
              null;
        in
        mkIf (themeColors != null) (mkConsoleColors themeColors);
    })

    # Explicit theme/variant configuration for console
    (mkIf (cfg.theme != null && cfg.variant != null) {
      console.colors =
        let
          loadedTheme = import cfg.theme;
          variantName = loadedTheme.defaults.${cfg.variant} or cfg.variant;
          themeColors = loadedTheme.variants.${variantName}.colors;
        in
        mkConsoleColors themeColors;
    })

    # OpenRGB: SMBus access for DDR5 RGB, motherboard RGB, GPU RGB
    # Vogix uses OpenRGB as its peripheral RGB backend
    {
      boot.kernelModules = [ "i2c-dev" "i2c-piix4" ];
      boot.kernelParams = [ "acpi_enforce_resources=lax" ];
    }

    # Kanata service for behavior module (evdev key remapping)
    # System-wide: nav layer, Super→Ctrl, CapsLock toggle
    (mkIf (kanataConfig != null) {
      services.kanata = {
        enable = true;
        keyboards.default = {
          devices = [ ];
          config = kanataConfig;
          extraDefCfg = "process-unmapped-keys yes";
        };
      };
    })
  ]);
}
