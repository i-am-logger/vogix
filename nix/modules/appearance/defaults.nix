# Vogix appearance defaults — Visual UX
#
# Opinionated defaults for how the desktop looks.
# All values can be overridden via programs.vogix.appearance.*
_:

{
  # Animations
  animations = {
    enable = true;
    bezier = "myBezier, 0.05, 0.9, 0.1, 1.05";
    rules = [
      "windows, 1, 2, myBezier"
      "windowsIn, 1, 2, myBezier, slide"
      "windowsOut, 1, 2, myBezier, slide"
      "windowsMove, 1, 2, myBezier"
      "border, 1, 2, default"
      "borderangle, 1, 2, default"
      "fade, 1, 2, default"
      "workspaces, 1, 2, default"
      "specialWorkspace, 1, 3, myBezier, slidefadevert top"
    ];
  };

  # Decoration
  decoration = {
    activeOpacity = 1.0;
    inactiveOpacity = 1.0;
    fullscreenOpacity = 1.0;
    rounding = 8;
    dimInactive = true;
    dimStrength = 0.3;
  };

  # Blur
  blur = {
    enable = true;
    size = 3;
    brightness = 0.7;
  };

  # Gaps and borders
  gaps = {
    inner = 10;
    outer = 10;
  };

  borderSize = 3;

  # Group bar
  group = {
    fontFamily = "Fira Code Nerd Font";
    fontSize = 28;
    height = 32;
    indicatorHeight = 5;
  };
}
