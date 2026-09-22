# vogix-desktop theme contract — theme.json
#
# One `theme.json` per (theme, variant), riding the ordinary app machinery:
# it lands in the theme package at `<pkg>/vogix-desktop/theme.json`, the
# symlink chain serves it at `~/.config/vogix-desktop/theme.json` through
# `current-theme`, and a theme switch reaches the running shell through the
# declared reload command. The desktop shell reads ONLY this file for
# colors.
#
# The 16 `semantic` keys are exactly praxis `Vogix16Semantic::key()`
# (snake_case); the per-scheme branches derive them so the shell never sees a
# scheme difference. `palette` is always the 16 base slots — for ansi16 the
# fill goes through the praxis slot mapping — so the preview widget reads one
# shape whatever the scheme. `polarity`
# comes from the theme data — no luminance math in the shell. `backgrounds`
# is always empty: the background set carries store paths only Nix knows,
# so it lives in backgrounds.json beside this file (generators.nix), and
# this one stays identical between the two render layers.
#
# The generators return a JSON STRING (the theme-file-only branch, the
# console/ripgrep precedent). A Tera template per scheme renders the same
# bytes into the on-demand cache; contract-tests.nix (checks.nix-unit) and
# the Rust template test pin both layers to one golden line.
{ lib, ... }:

let
  vogix16Lib = import ../lib/vogix16.nix { inherit lib; };
  inherit (vogix16Lib)
    base16FromAnsi16
    baseSlots
    semanticFromAnsi16
    semanticFromBase16
    semanticFromSlot
    ;

  toSnake = lib.replaceStrings [ "-" ] [ "_" ];

  # The contract document. `builtins.toJSON` renders attrset keys sorted, so
  # the byte layout is deterministic — the Tera templates mirror it.
  themeJson = { meta, palette, semantic }:
    builtins.toJSON {
      schema = 1;
      inherit (meta) theme variant scheme polarity;
      inherit palette semantic;
      backgrounds = [ ];
    };

  # base16/base24: raw baseXX colors in, native 16-slot palette + the
  # hue-mapped semantic table out. base24's extended slots (base10..17) are
  # not part of the 16-slot contract palette.
  fromBase16 = { colors, meta }:
    themeJson {
      inherit meta;
      palette = lib.genAttrs baseSlots (slot: colors.${slot});
      semantic = semanticFromBase16 colors;
    };

in
{
  configFile = "theme.json";

  # The shell re-reads the contract through its own verb, which relays to
  # the running shell; with no shell running (TTY, tests) it exits 0
  # quietly. `touch`/file-watching is deliberately NOT used — the store
  # symlink swap is invisible to Qt's watcher.
  reloadMethod = {
    method = "command";
    command = "vogix desktop reload";
  };

  schemes = {
    # vogix16: `colors` arrives as the SEMANTIC mapping (hyphenated keys, see
    # generators.nix). The snake form is a rename; the palette is the same
    # values re-keyed through the canonical slot table's inverse.
    vogix16 = { colors, meta }:
      let
        semantic = lib.mapAttrs' (name: lib.nameValuePair (toSnake name)) colors;
      in
      themeJson {
        inherit meta semantic;
        palette = lib.mapAttrs (_slot: key: semantic.${key}) semanticFromSlot;
      };

    base16 = fromBase16;
    base24 = fromBase16;

    # ansi16: color00..15 + named extras in; the praxis slot mapping fills
    # ten slots, documented fallbacks fill the other six (lib/vogix16.nix).
    ansi16 = { colors, meta }:
      themeJson {
        inherit meta;
        palette = base16FromAnsi16 colors;
        semantic = semanticFromAnsi16 colors;
      };
  };
}
