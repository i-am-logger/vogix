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
    size = 13;
  };

  bar = {
    enable = true;
    position = "top";
    height = 32;
    layout = {
      left = [ "workspaces" "mode" ];
      center = [ "window" ];
      right = [
        "update"
        "tray"
        "media"
        "weather"
        "cpu"
        "memory"
        "network"
        "bluetooth"
        "audio"
        "mic"
        "battery"
        "dnd"
        "indicators"
        "theme"
        "clock"
      ];
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
