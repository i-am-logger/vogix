# vogix16 scheme utilities
#
# The canonical semantic-color tables. praxis's `Vogix16Semantic` (theming
# ontology, `applied/hmi/theming/schemes.rs`) is the source of truth for the
# key set and the vogix16 slot assignment; the tables here mirror it for the
# Nix render layer, and `tests/` + the desktop VM suite pin the two against
# each other. Everything beyond `semanticColors` is DERIVED from it — no
# second hand-maintained list.
{ lib }:

let
  # Create semantic color mapping from baseXX colors
  # This provides a clean API for application modules using vogix16 semantic names
  #
  # Monochromatic scale (base00-07): background -> foreground progression
  # Functional colors (base08-0F): status and accent colors
  semanticColors = baseColors: {
    # Monochromatic base (base00-07)
    background = baseColors.base00;
    background-surface = baseColors.base01;
    background-selection = baseColors.base02;
    foreground-comment = baseColors.base03;
    foreground-border = baseColors.base04;
    foreground-text = baseColors.base05;
    foreground-heading = baseColors.base06;
    foreground-bright = baseColors.base07;

    # Functional colors (base08-0F)
    success = baseColors.base08;
    warning = baseColors.base09;
    notice = baseColors.base0A;
    danger = baseColors.base0B;
    active = baseColors.base0C;
    link = baseColors.base0D;
    highlight = baseColors.base0E;
    special = baseColors.base0F;
  };

  # The 16 base slots, in slot order.
  baseSlots = [
    "base00"
    "base01"
    "base02"
    "base03"
    "base04"
    "base05"
    "base06"
    "base07"
    "base08"
    "base09"
    "base0A"
    "base0B"
    "base0C"
    "base0D"
    "base0E"
    "base0F"
  ];

  toSnake = lib.replaceStrings [ "-" ] [ "_" ];

  # Same mapping, snake_case keys — the runtime convention
  # (praxis `Vogix16Semantic::key()`, the Tera template variables).
  semanticColorsSnake = baseColors:
    lib.mapAttrs' (name: lib.nameValuePair (toSnake name)) (semanticColors baseColors);

  # slot name -> snake semantic key, DERIVED by feeding the mapping its own
  # slot names ({ background = "base00"; … } inverted). One table, two
  # directions.
  semanticFromSlot =
    lib.mapAttrs' (name: slot: lib.nameValuePair slot (toSnake name))
      (semanticColors (lib.genAttrs baseSlots (s: s)));

  # The 16 snake_case semantic keys, sorted (attrNames is sorted).
  semanticKeys = builtins.attrNames (semanticColorsSnake (lib.genAttrs baseSlots (s: s)));

  # base16/base24 -> semantic. WRITTEN DOWN, not derived from `semanticColors`:
  # vogix16 assigns its own MEANING to slots (base08 = success), while base16's
  # convention is hue-based (base08 = red). The ramp base00..07 carries over;
  # the accents map by hue. Mirrors praxis's base16 semantic table.
  semanticFromBase16 = baseColors: {
    background = baseColors.base00;
    background_surface = baseColors.base01;
    background_selection = baseColors.base02;
    foreground_comment = baseColors.base03;
    foreground_border = baseColors.base04;
    foreground_text = baseColors.base05;
    foreground_heading = baseColors.base06;
    foreground_bright = baseColors.base07;

    danger = baseColors.base08; # base16 red
    notice = baseColors.base09; # base16 orange
    warning = baseColors.base0A; # base16 yellow
    success = baseColors.base0B; # base16 green
    active = baseColors.base0C; # base16 cyan
    link = baseColors.base0D; # base16 blue
    highlight = baseColors.base0E; # base16 magenta
    special = baseColors.base0F; # base16 brown
  };

  # ansi16 -> a full 16-slot base16 palette. The ten slots ansi16 fills come
  # from praxis `Ansi16Color::to_base16_slot` (black→00, red→08, green→0B,
  # yellow→0A, blue→0D, magenta→0E, cyan→0C, white→05, bright black→03,
  # bright white→07; the other brights land in base24's 12-17, outside a
  # 16-slot palette). The six slots ansi16 never fills (01/02/04/06/09/0F)
  # take documented fallbacks from the scheme's named extras and brights, so
  # "16 semantic keys always present" holds for every scheme:
  #   base01 surface        <- selection_bg (the one raised background ansi has)
  #   base02 selection      <- selection_bg (its native meaning)
  #   base04 border/dim fg  <- bright black (the gray)
  #   base06 heading fg     <- foreground
  #   base09 orange         <- bright red (nearest hue ansi carries)
  #   base0F special        <- bright magenta
  base16FromAnsi16 = colors: {
    base00 = colors.color00;
    base01 = colors.selection_bg;
    base02 = colors.selection_bg;
    base03 = colors.color08;
    base04 = colors.color08;
    base05 = colors.color07;
    base06 = colors.foreground;
    base07 = colors.color15;
    base08 = colors.color01;
    base09 = colors.color09;
    base0A = colors.color03;
    base0B = colors.color02;
    base0C = colors.color06;
    base0D = colors.color04;
    base0E = colors.color05;
    base0F = colors.color13;
  };

  # ansi16 -> semantic: through the slot fill, then the base16 hue table.
  semanticFromAnsi16 = colors: semanticFromBase16 (base16FromAnsi16 colors);

in
{
  inherit
    baseSlots
    base16FromAnsi16
    semanticColors
    semanticColorsSnake
    semanticFromAnsi16
    semanticFromBase16
    semanticFromSlot
    semanticKeys
    ;
}
