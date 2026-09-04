# Desktop shell options for Vogix
#
# Defines programs.vogix.desktop.* — the vogix desktop shell (bar,
# notifications, lock, launcher, …; quickshell-rendered in v1, the
# vogix-desktop Rust program in v2; the contract both consume is
# `theme.json` + `desktop.json`).
#
# Bare fragment in the behavior-options shape, merged into
# `options.programs.vogix` by home-manager/options.nix. The per-surface
# trees grow here increment by increment as the plan's steps land.
{ lib }:

let
  inherit (lib) mkOption mkEnableOption types;
  defaults = import ./defaults.nix { };
  v16 = import ../lib/vogix16.nix { inherit lib; };

  # A surface color token: a semantic SLOT (one of the 16 praxis keys) plus
  # an alpha. `background = "background"` coerces to `{ slot; alpha = 1.0; }`
  # — the praxis projection is SlotMapping { slot, target_key, Rgba { alpha } }.
  tokenType = types.coercedTo types.str (slot: { inherit slot; }) (types.submodule {
    options = {
      slot = mkOption {
        type = types.enum v16.semanticKeys;
        description = "Semantic slot this token resolves through the current theme.";
      };
      alpha = mkOption {
        type = types.numbers.between 0.0 1.0;
        default = 1.0;
        description = "Opacity applied to the resolved color.";
      };
    };
  });

  widgetNames = types.listOf types.str;

  # A launcher menu entry. `action` is a shell command; `submenu` nests one
  # level; `when` guards visibility (entry shown
  # only while the command exits 0, re-evaluated on menu open).
  menuLeafOptions = {
    id = mkOption {
      type = types.str;
      description = "Stable identifier (used by `vogix desktop menu --summon <id>`).";
    };
    icon = mkOption {
      type = types.str;
      default = "";
      description = "Glyph shown before the label (a Nerd Font icon, or empty).";
    };
    label = mkOption {
      type = types.str;
      description = "Text shown in the menu.";
    };
    action = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Shell command run on selection (null for a pure submenu entry).";
    };
    when = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Guard command; the entry is shown only while it exits 0.";
    };
  };
  menuEntryType = types.submodule {
    options = menuLeafOptions // {
      submenu = mkOption {
        type = types.listOf (types.submodule { options = menuLeafOptions; });
        default = [ ];
        description = "Nested entries opened in place of running an action.";
      };
    };
  };

  launcherModeNames = [ "apps" "files" "calc" "emoji" "ssh" "clipboard" "theme" "background" ];

  barEdges = [ "top" "bottom" "left" "right" ];

  # warn/danger pair for one meter; units differ per metric (percent for
  # cpu/memory/swap, °C for cpuTemp) — the description carries them.
  thresholdPair = metric: unit: {
    warn = mkOption {
      type = types.ints.positive;
      default = defaults.meters.thresholds.${metric}.warn;
      description = "${metric} level (${unit}) at which the meter turns warning-colored.";
    };
    danger = mkOption {
      type = types.ints.positive;
      default = defaults.meters.thresholds.${metric}.danger;
      description = "${metric} level (${unit}) at which the meter turns danger-colored.";
    };
  };

in
{
  desktop = mkOption {
    description = "The vogix desktop shell.";
    default = { };
    type = types.submodule {
      options = {
        enable = mkEnableOption "the vogix desktop shell (bar, notifications, lock, launcher)";

        font = {
          family = mkOption {
            type = types.str;
            default = defaults.font.family;
            description = "Shell UI font family.";
          };
          size = mkOption {
            type = types.ints.positive;
            default = defaults.font.size;
            description = "Base font size (logical px); shell type scale derives from it.";
          };
        };

        # One bar per screen edge, each on every monitor. `size` is the
        # thickness: height for top/bottom, width for left/right. Sections
        # are start/center/end along the bar's axis (start = left on a
        # horizontal bar, top on a vertical one). Some widgets are
        # horizontal-only (window, media, weather, theme) — an assertion in
        # the home-manager module rejects them on left/right.
        bars = lib.genAttrs barEdges (edge: {
          enable = mkOption {
            type = types.bool;
            default = defaults.bars.${edge}.enable;
            description = "Render the ${edge} bar (one per monitor).";
          };
          size = mkOption {
            type = types.ints.positive;
            default = defaults.bars.${edge}.size;
            description = "Bar thickness in logical px (height for a horizontal bar, width for a vertical one).";
          };
          layout = {
            start = mkOption {
              type = widgetNames;
              default = defaults.bars.${edge}.layout.start;
              description = "Widgets in the ${edge} bar's start section, in order.";
            };
            center = mkOption {
              type = widgetNames;
              default = defaults.bars.${edge}.layout.center;
              description = "Widgets in the ${edge} bar's center section, in order.";
            };
            end = mkOption {
              type = widgetNames;
              default = defaults.bars.${edge}.layout.end;
              description = "Widgets in the ${edge} bar's end section, in order.";
            };
          };
        });

        # The HUD's data-density knobs: the audio meters, the history ring
        # buffers behind the graphs, and the stat thresholds that drive
        # positional meter coloring (via surfaces.meter).
        meters = {
          spectrum = {
            enable = mkOption {
              type = types.bool;
              default = defaults.meters.spectrum.enable;
              description = "Run the cava audio spectrum (subprocess starts only while a spectrum widget is visible).";
            };
            bars = mkOption {
              type = types.ints.positive;
              default = defaults.meters.spectrum.bars;
              description = "Number of spectrum bars.";
            };
          };
          vu = {
            floorDb = mkOption {
              type = types.ints.between (-90) (-10);
              default = defaults.meters.vu.floorDb;
              description = "Bottom of the VU meters' dB window (0 dBFS is the top).";
            };
          };
          history = mkOption {
            type = types.ints.positive;
            default = defaults.meters.history;
            description = "Samples kept per stat graph (ring buffer length).";
          };
          sampleMs = mkOption {
            type = types.ints.between 50 5000;
            default = defaults.meters.sampleMs;
            description = "cpu/mem/net sample period in ms (the graphs' liveliness; temperature and disk stay on slow ticks).";
          };
          mounts = mkOption {
            type = types.listOf types.str;
            default = defaults.meters.mounts;
            description = ''
              Capacity gauges for the stat-mounts cell, in order: absolute
              mount points (measured by a 30 s df) plus the literal `swap`,
              which reads the meminfo figures. A path this host does not
              mount is omitted from the bar, never rendered as 0%.
            '';
          };
          thresholds = {
            cpu = thresholdPair "cpu" "percent";
            cpuTemp = thresholdPair "cpuTemp" "°C";
            memory = thresholdPair "memory" "percent";
            swap = thresholdPair "swap" "percent";
          };
        };

        notifications = {
          enable = mkOption {
            type = types.bool;
            default = defaults.notifications.enable;
            description = "Run the shell's notification server (the org.freedesktop.Notifications owner).";
          };
          defaultTimeout = mkOption {
            type = types.ints.positive;
            default = defaults.notifications.defaultTimeout;
            description = "Popup lifetime in ms for normal notifications. Critical ones never expire.";
          };
          maxVisible = mkOption {
            type = types.ints.positive;
            default = defaults.notifications.maxVisible;
            description = "Most popups shown at once; the rest queue.";
          };
          appRules = mkOption {
            type = types.attrsOf (types.submodule {
              options = {
                timeout = mkOption {
                  type = types.nullOr types.ints.positive;
                  default = null;
                  description = "Popup lifetime in ms for this app (null = the default).";
                };
                accent = mkOption {
                  type = types.nullOr (types.enum v16.semanticKeys);
                  default = null;
                  description = "Semantic slot for this app's popup accent/border.";
                };
                bypassDnd = mkOption {
                  type = types.bool;
                  default = false;
                  description = "Show this app's popups even in do-not-disturb.";
                };
              };
            });
            default = defaults.notifications.appRules;
            description = "Per-app notification rules, keyed by app name.";
          };
        };

        osd = {
          enable = mkOption {
            type = types.bool;
            default = defaults.osd.enable;
            description = "Render the on-screen display (volume/brightness flashes).";
          };
          timeout = mkOption {
            type = types.ints.positive;
            default = defaults.osd.timeout;
            description = "How long an OSD flash stays visible, in ms.";
          };
        };

        polkit = {
          enable = mkOption {
            type = types.bool;
            default = defaults.polkit.enable;
            description = "Run the shell's polkit authentication agent.";
          };
        };

        lock = {
          enable = mkOption {
            type = types.bool;
            default = defaults.lock.enable;
            description = "The shell's session lock (WlSessionLock + PAM).";
          };
          pamService = mkOption {
            type = types.str;
            default = defaults.lock.pamService;
            description = ''
              PAM service the lock authenticates against. The shell REFUSES to
              lock when /etc/pam.d/<service> is absent — never an unlockable
              screen. vogix's NixOS module declares the default service.
            '';
          };
        };

        background = {
          enable = mkOption {
            type = types.bool;
            default = defaults.background.enable;
            description = "Render the per-screen wallpaper layer from the theme's background set.";
          };
          animate = mkOption {
            type = types.enum [ "always" "on-ac" "never" ];
            default = defaults.background.animate;
            description = "When live background kinds (shader/video) may animate; every setting pauses them while the idle dim stage is up.";
          };
          scanlines = mkOption {
            type = types.bool;
            default = defaults.background.scanlines;
            description = "Overlay a subtle CRT scanline shader on the shell's chrome surfaces (bars, notifications).";
          };
        };

        decorations = {
          focusBrackets = mkOption {
            type = types.bool;
            default = defaults.decorations.focusBrackets;
            description = ''
              Corner brackets on the FOCUSED window, drawn by a click-through
              shell overlay (Hyprland borders are full-perimeter only, so the
              compositor cannot draw these). Brackets follow window geometry
              over IPC, so they glide to a drag's drop point rather than
              chasing it.
            '';
          };
        };

        launcher = {
          enable = mkOption {
            type = types.bool;
            default = defaults.launcher.enable;
            description = "The shell's launcher overlay (apps, files, calc, emoji, ssh, clipboard, pickers, menu, dmenu mode).";
          };
          modes = lib.genAttrs launcherModeNames (mode: {
            enable = mkOption {
              type = types.bool;
              default = defaults.launcher.modes.${mode}.enable;
              description = "Offer the ${mode} launcher mode.";
            };
          });
          menu = mkOption {
            type = types.listOf menuEntryType;
            default = defaults.launcher.menu;
            description = "The root menu (opened by `vogix desktop menu`); a loaded list, never hardcoded in the shell.";
          };
        };

        power = {
          enable = mkOption {
            type = types.bool;
            default = defaults.power.enable;
            description = "The shell's power menu (lock, logout, suspend, reboot, poweroff).";
          };
        };

        weather = {
          enable = mkOption {
            type = types.bool;
            default = defaults.weather.enable;
            description = "The weather widget/panel (wttrbar, refreshed every 30 minutes).";
          };
          location = mkOption {
            type = types.str;
            default = defaults.weather.location;
            description = "Location passed to wttrbar (empty = wttr.in geolocation).";
          };
        };

        nightlight = {
          temperature = mkOption {
            type = types.ints.positive;
            default = defaults.nightlight.temperature;
            description = "Color temperature (K) hyprsunset applies while night light is on.";
          };
        };

        idle = {
          screensaver = mkOption {
            type = types.nullOr types.ints.positive;
            default = defaults.idle.screensaver;
            description = "Seconds of idle before the screensaver overlay (null = never).";
          };
          dim = mkOption {
            type = types.nullOr types.ints.positive;
            default = defaults.idle.dim;
            description = "Seconds of idle before the screens dim (null = never).";
          };
          lock = mkOption {
            type = types.nullOr types.ints.positive;
            default = defaults.idle.lock;
            description = "Seconds of idle before the session locks (null = never).";
          };
          screenOff = mkOption {
            type = types.nullOr types.ints.positive;
            default = defaults.idle.screenOff;
            description = "Seconds of idle before displays power off (null = never).";
          };
          suspend = mkOption {
            type = types.nullOr types.ints.positive;
            default = defaults.idle.suspend;
            description = "Seconds of idle before the machine suspends (null = never).";
          };
        };

        surfaces = mkOption {
          type = types.attrsOf (types.attrsOf tokenType);
          default = { };
          description = ''
            Per-surface color tokens ({ slot, alpha }, or a bare slot name).
            Surfaces are free-form names (bar, popup, notification, lock, …);
            `vogix desktop check` projects them onto praxis surface functors
            and rejects unknown slots. Defaults for the shipped surfaces merge
            underneath (defaults.nix).
          '';
        };
      };
    };
  };
}
