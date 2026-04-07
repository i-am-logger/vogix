{ lib, ... }:

let
  hexDigit = c:
    let
      m = {
        "0" = 0;
        "1" = 1;
        "2" = 2;
        "3" = 3;
        "4" = 4;
        "5" = 5;
        "6" = 6;
        "7" = 7;
        "8" = 8;
        "9" = 9;
        "a" = 10;
        "b" = 11;
        "c" = 12;
        "d" = 13;
        "e" = 14;
        "f" = 15;
        "A" = 10;
        "B" = 11;
        "C" = 12;
        "D" = 13;
        "E" = 14;
        "F" = 15;
      };
    in
    m.${c};

  hexToDec = s:
    hexDigit (builtins.substring 0 1 s) * 16 + hexDigit (builtins.substring 1 1 s);

  hexToRgba = hex:
    let
      clean = lib.removePrefix "#" hex;
      r = hexToDec (builtins.substring 0 2 clean);
      g = hexToDec (builtins.substring 2 2 clean);
      b = hexToDec (builtins.substring 4 2 clean);
    in
    { inherit r g b; a = 255; };

  hexToRgbaAlpha = hex: a:
    (hexToRgba hex) // { inherit a; };

  mkPreset = args: builtins.toJSON ({
    name = "vogix theme";
  } // args);
in
{
  # Bespec color preset — symlinked from ~/.local/share/bespec/presets/colors/
  configFile = "presets/colors/vogix_theme.json";

  # Presets live in XDG data dir, not config dir
  dataDir = "bespec";

  reloadMethod = {
    method = "signal";
    signal = "USR1";
    process_name = "bespec";
  };

  schemes = {
    vogix16 = colors: mkPreset {
      low = hexToRgba colors.foreground-border;
      high = hexToRgba colors.warning;
      peak = hexToRgba colors.danger;
      background = hexToRgba colors.background;
      text = hexToRgba colors.foreground-text;
      inspector_bg = hexToRgbaAlpha colors.background-surface 229;
      inspector_fg = hexToRgba colors.foreground-text;
    };

    base16 = colors: mkPreset {
      low = hexToRgba colors.base01;
      high = hexToRgba colors.base09;
      peak = hexToRgba colors.base08;
      background = hexToRgba colors.base00;
      text = hexToRgba colors.base05;
      inspector_bg = hexToRgbaAlpha colors.base01 229;
      inspector_fg = hexToRgba colors.base05;
    };

    base24 = colors: mkPreset {
      low = hexToRgba colors.base01;
      high = hexToRgba colors.base09;
      peak = hexToRgba colors.base08;
      background = hexToRgba colors.base00;
      text = hexToRgba colors.base05;
      inspector_bg = hexToRgbaAlpha colors.base01 229;
      inspector_fg = hexToRgba colors.base05;
    };

    ansi16 = colors: mkPreset {
      low = hexToRgba colors.color00;
      high = hexToRgba colors.color03;
      peak = hexToRgba colors.color01;
      background = hexToRgba colors.background;
      text = hexToRgba colors.foreground;
      inspector_bg = hexToRgbaAlpha colors.color00 229;
      inspector_fg = hexToRgba colors.foreground;
    };
  };
}
