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
in
{
  # "#rrggbb" → { r g b } as 0..1 floats.
  unitRgb = hex:
    let clean = lib.removePrefix "#" hex;
    in {
      r = channel clean 0;
      g = channel clean 2;
      b = channel clean 4;
    };
}
