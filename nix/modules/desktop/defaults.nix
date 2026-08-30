# Vogix desktop shell defaults
#
# Opinionated defaults for the shell surfaces. All values can be overridden
# via programs.vogix.desktop.*.
#
# Surface color tokens are `{ slot, alpha }` — slot is one of the 16 praxis
# semantic keys, resolved against the CURRENT theme's `theme.json` at
# runtime (one lookup in the shell, identical in QML and Rust). No hex here,
# ever: a vogix theme is a palette, and the shell follows it live.
_:

{
  font = {
    family = "Fira Code Nerd Font";
    size = 16;
  };

  # The HUD: four bars on every monitor. Sections run start→end along the
  # bar's axis. Horizontal-only widgets (window, media, weather, theme)
  # never appear on left/right — the home-manager module asserts it.
  bars = {
    top = {
      enable = true;
      size = 36;
      layout = {
        start = [ "workspaces" "mode" ];
        center = [ "window" ];
        end = [ "spectrum-mini" "kbd" "privacy" "dnd" "indicators" "theme" "clock" ];
      };
    };
    bottom = {
      enable = true;
      size = 32;
      layout = {
        start = [ "media" ];
        center = [ "stat-cpu" "stat-temp" "stat-mem" "stat-swap" "stat-disk" "stat-net" "vu-out" "vu-mic" ];
        end = [ "tailscale" "weather" "update" "tray" ];
      };
    };
    left = {
      enable = true;
      size = 36;
      layout = {
        start = [ "workspaces" "mode" ];
        center = [ ];
        end = [ "menu" "power-glyph" ];
      };
    };
    right = {
      enable = true;
      size = 48;
      layout = {
        start = [ "vu-rail" "spectrum-rail" ];
        center = [ "graph-cpu" "graph-mem" "graph-net" ];
        end = [ "batteries" "battery" "audio" "mic" "network" "bluetooth" ];
      };
    };
  };

  # Meter tuning: the 40dB VU window and the stat thresholds that flip a
  # meter's segments to warning/danger colors (percent, °C for cpuTemp).
  meters = {
    spectrum = {
      enable = true;
      bars = 16;
    };
    vu.floorDb = -40;
    history = 64;
    thresholds = {
      cpu = { warn = 50; danger = 90; };
      cpuTemp = { warn = 60; danger = 85; };
      memory = { warn = 60; danger = 90; };
      swap = { warn = 20; danger = 80; };
    };
  };

  notifications = {
    enable = true;
    # mako-parity behaviour, now rules the shell loads from desktop.json:
    # normal notifications expire; CRITICAL never does (GPG/YubiKey prompts
    # must outlive a glance away); per-app rules can pin a timeout, recolor
    # the accent, and bypass do-not-disturb.
    defaultTimeout = 5000;
    maxVisible = 5;
    appRules = {
      "yubikey-touch-detector" = {
        timeout = 15000;
        accent = "danger";
        bypassDnd = true;
      };
    };
  };

  osd = {
    enable = true;
    timeout = 1500;
  };

  polkit = {
    enable = true;
  };

  lock = {
    enable = true;
    pamService = "vogix-lock";
  };

  background = {
    enable = true;
    # Live kinds (shader/video) never cost battery by default.
    animate = "on-ac";
    scanlines = false;
  };

  launcher = {
    enable = true;
    modes = {
      apps.enable = true;
      files.enable = true;
      calc.enable = true;
      emoji.enable = true;
      ssh.enable = true;
      clipboard.enable = true;
      theme.enable = true;
      background.enable = true;
    };
    # The root menu — vogix's own surfaces, so the menu is useful with zero
    # host configuration. Hosts append or replace via desktop.launcher.menu.
    menu = [
      { id = "keybindings"; icon = "󰌌"; label = "Keybindings"; action = "vogix input keys"; }
      { id = "theme"; icon = "󰏘"; label = "Theme…"; action = "vogix desktop launcher --mode theme"; }
      { id = "background"; icon = "󰸉"; label = "Next background"; action = "vogix desktop background next"; }
      { id = "dnd"; icon = "󰂛"; label = "Do not disturb"; action = "vogix desktop notify dnd toggle"; }
      { id = "agents"; icon = "󱚝"; label = "Claude usage…"; action = "vogix desktop panel agents"; }
      { id = "lock"; icon = "󰌾"; label = "Lock"; action = "vogix desktop lock"; }
      { id = "power"; icon = "󰐥"; label = "Power…"; action = "vogix desktop power"; }
    ];
  };

  power = {
    enable = true;
  };

  weather = {
    enable = true;
    location = "";
  };

  nightlight = {
    temperature = 4000;
  };

  # Idle stages in seconds; null disables a stage. Suspend is off by
  # default — a desktop that vanishes mid-thought is a host decision — and
  # so is the screensaver (the dim stage is the default idle cue).
  idle = {
    screensaver = null;
    dim = 300;
    lock = 600;
    screenOff = 660;
    suspend = null;
  };

  surfaces = {
    # Every SegmentedMeter/VuMeter/Sparkline resolves through this one
    # surface: positional segment colors, the unlit trough, the peak cap,
    # the hairline frame, the label/value text.
    meter = {
      low = "success";
      mid = "warning";
      high = "danger";
      unlit = { slot = "foreground_border"; alpha = 0.22; };
      cap = "foreground_comment";
      frame = { slot = "foreground_border"; alpha = 0.35; };
      label = "foreground_comment";
      value = "foreground_text";
    };
    bar = {
      background = { slot = "background"; alpha = 0.92; };
      foreground = "foreground_text";
      muted = "foreground_comment";
      accent = "active";
      border = "foreground_border";
      urgent = "danger";
    };
    popup = {
      background = { slot = "background_surface"; alpha = 0.98; };
      foreground = "foreground_text";
      muted = "foreground_comment";
      accent = "active";
      border = "foreground_border";
    };
    notification = {
      background = { slot = "background_surface"; alpha = 0.98; };
      foreground = "foreground_text";
      muted = "foreground_comment";
      accent = "active";
      border = "foreground_border";
      urgent = "danger";
    };
    osd = {
      background = { slot = "background_surface"; alpha = 0.95; };
      foreground = "foreground_text";
      accent = "active";
      muted = "foreground_comment";
    };
    polkit = {
      background = { slot = "background_surface"; alpha = 1.0; };
      foreground = "foreground_text";
      muted = "foreground_comment";
      accent = "active";
      border = "foreground_border";
      danger = "danger";
    };
    lock = {
      background = { slot = "background"; alpha = 1.0; };
      surface = { slot = "background_surface"; alpha = 0.98; };
      foreground = "foreground_text";
      muted = "foreground_comment";
      accent = "active";
      danger = "danger";
    };
    launcher = {
      background = { slot = "background_surface"; alpha = 0.98; };
      foreground = "foreground_text";
      muted = "foreground_comment";
      accent = "active";
      border = "foreground_border";
      selection = { slot = "background_selection"; alpha = 1.0; };
    };
    power = {
      background = { slot = "background_surface"; alpha = 0.98; };
      foreground = "foreground_text";
      muted = "foreground_comment";
      accent = "active";
      border = "foreground_border";
      selection = { slot = "background_selection"; alpha = 1.0; };
      danger = "danger";
    };
  };
}
