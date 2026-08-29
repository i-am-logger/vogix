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
  };
}
