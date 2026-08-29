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
      right = [ "theme" "clock" ];
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

  # Idle stages in seconds; null disables a stage. Suspend is off by
  # default — a desktop that vanishes mid-thought is a host decision.
  idle = {
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
  };
}
