# NixOS module for Vogix
#
# Provides system-level integration:
# - Console colors (TTY) from the machine owner's vogix theme
# - Machine surfaces (machine.nix): the VT palette, LEDs and command devices
#   following the machine owner's published palette; where the VT palette
#   is not vogix-machine's, the chvt and setvtrgb wrappers the console app
#   reloads with
# - Hardware modules (Kraken Elite, Keychron, DRAM RGB, OpenRGB)
# - The desktop shell's system side (lock PAM service, lock handler, and
#   the UPower / power-profiles D-Bus services while a user runs the shell)
#
# NOTE: User configuration (config.toml, app configs) is handled by
# the home-manager module at ~/.local/state/vogix/
{ liquidctlSrc }:

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
    ;

  cfg = config.vogix;

  # The home-manager users with programs.vogix.enable.
  homeManagerUsers = import ./lib/vogix-users.nix { inherit config options lib; };

  # Whether any of those users runs the vogix desktop shell.
  desktopInUse = builtins.any
    (user: config.home-manager.users.${user}.programs.vogix.desktop.enable or false)
    homeManagerUsers;

  # The machine owner's vogix configuration: the source of the console
  # colours, the plymouth splash and the greeter palette.
  inherit (cfg.machine) owner;
  hmVogixCfg =
    if owner != null && builtins.elem owner homeManagerUsers
    then config.home-manager.users.${owner}.programs.vogix
    else null;

  # console.colors takes "rrggbb"; the console palette is "#rrggbb".
  toConsoleColors = map (lib.removePrefix "#");

  # The console palette mapping (templates/<scheme>/console.palette.vogix),
  # the one the owner's theme packages and the runtime render use.
  consolePalette = import ./lib/console-palette.nix { inherit lib; };
in
{
  imports = [
    ./hardware
    ./openrgb.nix
    ./machine.nix
  ];

  options.vogix = {
    enable = mkEnableOption "vogix theme management";

    autoFromHomeManager = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Automatically configure console colors from home-manager vogix configuration.
        When enabled, uses the theme of the machine owner
        (vogix.machine.owner).
      '';
    };

    theme = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to theme file for console colors (overrides auto-detection).";
      example = literalExpression "./themes/yoga.nix";
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

    plymouth = {
      enable = mkEnableOption "the vogix plymouth boot splash (text-only script theme from the machine owner's palette)";
    };

    greeter = {
      enable = mkEnableOption "the vogix SDDM greeter (SDDM under a Hyprland Lua compositor, themed from the machine owner's palette)";

      compositor = mkOption {
        type = types.enum [ "hyprland" "weston" ];
        default = "hyprland";
        description = ''
          The Wayland compositor SDDM hosts its greeter on. Hyprland (with a
          minimal Lua config whose background is the theme's base00, so boot,
          greeter and session are color-continuous) on real hosts; weston —
          nixpkgs' default, no GL required — for VM tests.
        '';
      };
    };
  };

  config = mkIf cfg.enable (mkMerge [
    {
      # Make vogix CLI + dependencies available system-wide
      environment.systemPackages = [
        pkgs.vogix
        pkgs.tmux # Required for F12 system console
      ];

      # The shell's session lock authenticates against THIS service; the
      # shell refuses to lock when the file is absent (never an unlockable
      # screen), so the PAM declaration travels with the program that needs
      # it. unix/u2f/fprintd follow the host's global PAM settings; the fail
      # delay slows brute force on the lock screen.
      security.pam.services.vogix-lock = {
        failDelay = {
          enable = true;
          delay = 2000000;
        };
      };

      # Bridge logind's lock/sleep signals to user-session targets, so
      # `loginctl lock-session` and suspend reach the shell's lock through
      # the vogix-lock.service unit (declared in the home-manager module).
      services.systemd-lock-handler.enable = true;
    }

    # The system D-Bus services the desktop shell reads: UPower (battery
    # cells, low-battery alerts, the power panel) and power-profiles-daemon
    # (the power panel's profile row). Both travel with the program that
    # needs them, as the lock's PAM service does; mkDefault, so a host keeps
    # the last word. power-profiles-daemon stays off where another power
    # manager already owns that role, since it conflicts with each of them.
    (mkIf desktopInUse {
      services.upower.enable = lib.mkDefault true;
      services.power-profiles-daemon.enable = lib.mkDefault (
        !(config.services.tlp.enable
          || config.services.auto-cpufreq.enable
          || config.services.tuned.enable
          || config.hardware.system76.power-daemon.enable)
      );
    })

    # The machine owner's console palette, the one their theme package ships
    # and vogix-machine applies once a palette is published: the VT colours
    # from early boot on.
    (mkIf (cfg.autoFromHomeManager && cfg.theme == null && cfg.variant == null && hmVogixCfg != null) {
      console.colors = toConsoleColors hmVogixCfg.consolePalette;
    })

    # Explicit theme/variant configuration for console
    (mkIf (cfg.theme != null && cfg.variant != null) {
      console.colors =
        let
          loadedTheme = import cfg.theme;
          variantName = loadedTheme.defaults.${cfg.variant} or cfg.variant;
          themeColors = loadedTheme.variants.${variantName}.colors;
        in
        toConsoleColors (consolePalette.palette (loadedTheme.scheme or "vogix16") themeColors);
    })

    # Liquidctl overlay (patched fork with Kraken 2024 Elite RGB ring support)
    (mkIf cfg.hardware.kraken-elite.enable {
      nixpkgs.overlays = [
        (_final: prev: {
          liquidctl = prev.liquidctl.overridePythonAttrs (_old: {
            src = liquidctlSrc;
          });
        })
      ];
    })

    # The boot splash: a text-only plymouth script theme from the same
    # owner palette the console and greeter use — boot, login and
    # session are color-continuous with no recolored bitmaps anywhere.
    # Build-time like console.colors (a theme switch reaches it at the
    # next rebuild); mkDefault everywhere so a host keeps the last word.
    (mkIf cfg.plymouth.enable (
      let
        semantic =
          if hmVogixCfg != null then
            lib.mapAttrs'
              (n: lib.nameValuePair (builtins.replaceStrings [ "-" ] [ "_" ] n))
              hmVogixCfg.colors
          else
            { };
        theme = pkgs.callPackage ../packages/vogix-plymouth.nix { colors = semantic; };
      in
      {
        boot.plymouth = {
          themePackages = [ theme ];
          theme = lib.mkDefault "vogix";
        };
      }
    ))

    # The greeter surface: SDDM with the vogix QML theme, its [General]
    # palette rendered from the machine owner's semantic colors — the
    # console.colors precedent, so every scheme (not just vogix16) reaches
    # the greeter; never a re-render from theme files at this layer. The
    # opt-in runtime follow (`vogix greeter sync`, programs.vogix.greeter
    # .sync) copies the live theme.json into /var/lib/vogix/greeter, which
    # the QML prefers over the build-time palette.
    (mkIf cfg.greeter.enable (
      let
        semantic =
          if hmVogixCfg != null then
            lib.mapAttrs'
              (n: lib.nameValuePair (builtins.replaceStrings [ "-" ] [ "_" ] n))
              hmVogixCfg.colors
          else
            { };
        fontFamily =
          if hmVogixCfg != null then hmVogixCfg.desktop.font.family else "monospace";
        sddmTheme = pkgs.callPackage ../packages/vogix-sddm-theme.nix {
          conf = semantic // { font = fontFamily; };
        };
        bg6 = builtins.replaceStrings [ "#" ] [ "" ] (semantic.background or "181818");
      in
      {
        services.displayManager.sddm = {
          enable = lib.mkDefault true;
          wayland.enable = true;
          theme = "vogix";
          extraPackages = [ pkgs.qt6.qtsvg ];
          # The QML's runtime-follow reads /var/lib/vogix/greeter over
          # XMLHttpRequest on file:// — Qt gates that behind this env var.
          settings.General.GreeterEnvironment = "QML_XHR_ALLOW_FILE_READ=1";
          wayland.compositorCommand = mkIf (cfg.greeter.compositor == "hyprland") "${pkgs.hyprland}/bin/start-hyprland -- --config /etc/vogix/greeter/hyprland.lua";
        };

        environment.systemPackages = [ sddmTheme ];

        # The greeter compositor's config is Lua FROM DAY ONE (valid on
        # 0.56.2 and 0.57): logo/splash off, no animations, first frame
        # painted in the theme's base00.
        environment.etc."vogix/greeter/hyprland.lua".text = ''
          hl.config({
            misc = {
              disable_hyprland_logo = true,
              disable_splash_rendering = true,
              force_default_wallpaper = 0,
              background_color = 0xff${bg6},
            },
            animations = { enabled = false },
          })
        '';

        # Runtime-follow drop zone: group-writable so `vogix greeter sync`
        # (run as a vogix user on theme switch) can copy the live palette.
        # Theme + wallpaper are not secrets.
        systemd.tmpfiles.rules = [ "d /var/lib/vogix/greeter 2775 root vogix -" ];
        users.groups.vogix = { };
        users.users = lib.genAttrs homeManagerUsers (_user: {
          extraGroups = [ "vogix" ];
        });
      }
    ))

    # Input engine: uinput + group membership wiring.
    # hardware.uinput exposes /dev/uinput; the vogix input daemon needs to open
    # it RW to create the virtual keyboard. Membership in the `input` group lets
    # it open /dev/input/event* for the real keyboard grab; membership in
    # `uinput` lets it open /dev/uinput for emit. Without root either of those
    # would EACCES — that's the whole reason we list the groups here.
    {
      hardware.uinput.enable = true;
      users.users = lib.genAttrs homeManagerUsers (_user: {
        extraGroups = [ "input" "uinput" ];
      });
    }
  ]);
}
