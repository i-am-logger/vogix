# Generated backgrounds — the PRIMARY background kind.
#
# Every theme variant gets at least one background rendered from ITS OWN
# 16-slot palette at build time: exact palette match by construction, light
# and dark guaranteed (the variant axis carries polarity), and zero
# licensing surface. Curated image/video/shader sets from the
# vogix-backgrounds data repo layer on top through
# `appearance.extraBackgrounds` when that input exists.
#
# The recipe ("veil"): a diagonal ramp over base00→base01→base02, an
# `active` (base0C) glow from the top-right, a `warning` (base09) glow from
# the bottom-left, and fractal-noise grain tinted with base05 — quiet enough
# to sit behind a desktop, unmistakably the theme's own colors.
{ lib, pkgs }:

let
  # "#rrggbb" → { r g b } as 0..1 floats for feColorMatrix.
  hexDigit = c:
    lib.lists.findFirstIndex (x: x == lib.toLower c)
      (throw "invalid hex digit '${c}'")
      [ "0" "1" "2" "3" "4" "5" "6" "7" "8" "9" "a" "b" "c" "d" "e" "f" ];
  channel = hex: off:
    (16 * hexDigit (builtins.substring off 1 hex)
    + hexDigit (builtins.substring (off + 1) 1 hex))
    / 255.0;
  unitRgb = hex:
    let clean = lib.removePrefix "#" hex;
    in {
      r = channel clean 0;
      g = channel clean 2;
      b = channel clean 4;
    };

  veilSvg = colors:
    let grain = unitRgb colors.base05;
    in ''
      <svg xmlns="http://www.w3.org/2000/svg" width="3840" height="2160">
        <defs>
          <linearGradient id="base" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="${colors.base00}"/>
            <stop offset="0.55" stop-color="${colors.base01}"/>
            <stop offset="1" stop-color="${colors.base02}"/>
          </linearGradient>
          <radialGradient id="glowA" cx="0.8" cy="0.15" r="0.9">
            <stop offset="0" stop-color="${colors.base0C}" stop-opacity="0.55"/>
            <stop offset="1" stop-color="${colors.base0C}" stop-opacity="0"/>
          </radialGradient>
          <radialGradient id="glowB" cx="0.12" cy="0.85" r="0.8">
            <stop offset="0" stop-color="${colors.base09}" stop-opacity="0.35"/>
            <stop offset="1" stop-color="${colors.base09}" stop-opacity="0"/>
          </radialGradient>
          <filter id="grain">
            <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" stitchTiles="stitch"/>
            <feColorMatrix type="matrix" values="0 0 0 0 ${toString grain.r} 0 0 0 0 ${toString grain.g} 0 0 0 0 ${toString grain.b} 0 0 0 0.05 0"/>
          </filter>
        </defs>
        <rect width="3840" height="2160" fill="url(#base)"/>
        <rect width="3840" height="2160" fill="url(#glowA)"/>
        <rect width="3840" height="2160" fill="url(#glowB)"/>
        <rect width="3840" height="2160" filter="url(#grain)"/>
      </svg>
    '';

  # A STANDALONE derivation per background (never the enclosing theme
  # package's own path — that would recurse); the theme package symlinks it
  # under vogix-desktop/backgrounds/.
  mkGeneratedBackground = { themeVariant, colors }:
    pkgs.runCommand "vogix-background-${themeVariant}-veil"
      {
        nativeBuildInputs = [ pkgs.librsvg ];
        svg = veilSvg colors;
        passAsFile = [ "svg" ];
      } ''
      rsvg-convert -w 3840 -h 2160 "$svgPath" -o $out
    '';

in
{
  inherit mkGeneratedBackground;
}
