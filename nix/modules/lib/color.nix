# Color plumbing shared by the build-time renderers (generated backgrounds,
# the plymouth theme): "#rrggbb" → unit-float channels.
{ lib }:

let
  hexDigit = c:
    lib.lists.findFirstIndex (x: x == lib.toLower c)
      (throw "invalid hex digit '${c}'")
      [ "0" "1" "2" "3" "4" "5" "6" "7" "8" "9" "a" "b" "c" "d" "e" "f" ];
  channel = hex: off:
    (16 * hexDigit (builtins.substring off 1 hex)
    + hexDigit (builtins.substring (off + 1) 1 hex))
    / 255.0;

  # "#rrggbb" → { r g b } as 0..1 floats.
  unitRgb = hex:
    let clean = lib.removePrefix "#" hex;
    in {
      r = channel clean 0;
      g = channel clean 2;
      b = channel clean 4;
    };

  # a^(1/5) for 0 < a <= 1 by Newton's method (Nix has no float pow)
  fifthRoot = a:
    let step = y: (4 * y + a / (y * y * y * y)) / 5;
    in builtins.foldl' (y: _: step y) 1.0 (lib.range 1 40);

  # The sRGB transfer function: x^2.4 is x^2 times the fifth root of x^2
  linear = c:
    if c <= 0.04045 then c / 12.92
    else
      let x = (c + 0.055) / 1.055;
      in x * x * fifthRoot (x * x);

  # WCAG relative luminance
  luminance = hex:
    let c = unitRgb hex;
    in 0.2126 * linear c.r + 0.7152 * linear c.g + 0.0722 * linear c.b;
in
{
  inherit unitRgb luminance;

  # WCAG contrast ratio of two "#rrggbb" colours, 1 to 21
  contrast = a: b:
    let
      la = luminance a;
      lb = luminance b;
    in
    (lib.max la lb + 0.05) / (lib.min la lb + 0.05);
}
