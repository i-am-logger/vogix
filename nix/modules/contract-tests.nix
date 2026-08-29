# theme.json contract tests
#
# Pins the vogix-desktop generator (nix/modules/applications/vogix-desktop.nix)
# — the desktop shell's color contract — at the byte level:
#
# - the vogix16 branch's output equals the GOLDEN line that
#   src/template/tests.rs::theme_json_template_matches_nix_generator_bytes
#   also pins the Tera template to, so the two render layers (Nix-built theme
#   package, on-demand cache) provably produce identical bytes;
# - every scheme branch yields valid JSON carrying all 16 praxis semantic
#   keys and all 16 palette slots;
# - the per-scheme semantic tables map the slots the plan writes down.
#
# Run with: nix eval --impure -f nix/modules/contract-tests.nix --apply 'f: f {}'
{ pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
}:

let
  gen = import ./applications/vogix-desktop.nix { inherit lib; };
  v16 = import ./lib/vogix16.nix { inherit lib; };

  check = name: cond:
    if cond then { inherit name; passed = true; }
    else throw "FAILED: ${name}";

  assertEq = name: expected: actual:
    if expected == actual then { inherit name; passed = true; }
    else throw "FAILED: ${name} — expected ${toString expected}, got ${toString actual}";

  # ── Fixtures ──
  # base00..base0F = #101010..#1f1f1f — every slot value distinct and
  # slot-traceable. Identical to the Rust golden test's fixture.
  hexAt = i:
    let b = lib.toLower (lib.toHexString (16 + i));
    in "#${b}${b}${b}";
  rawBase = lib.listToAttrs (lib.imap0 (i: s: lib.nameValuePair s (hexAt i)) v16.baseSlots);

  mkMeta = scheme: {
    theme = "goldtest";
    variant = "night";
    inherit scheme;
    polarity = "dark";
  };

  vogix16Json = gen.schemes.vogix16 {
    colors = v16.semanticColors rawBase;
    meta = mkMeta "vogix16";
  };
  base16Json = gen.schemes.base16 { colors = rawBase; meta = mkMeta "base16"; };

  ansiColors = lib.listToAttrs
    (builtins.genList
      (i: lib.nameValuePair "color${lib.fixedWidthNumber 2 i}" (hexAt i))
      16) // {
    background = "#010101";
    foreground = "#020202";
    cursor_bg = "#030303";
    cursor_fg = "#040404";
    selection_bg = "#050505";
    selection_fg = "#060606";
  };
  ansi16Json = gen.schemes.ansi16 { colors = ansiColors; meta = mkMeta "ansi16"; };

  parsed = builtins.mapAttrs (_: builtins.fromJSON) {
    vogix16 = vogix16Json;
    base16 = base16Json;
    ansi16 = ansi16Json;
  };

  # The golden line — the SAME string the Rust template test pins (without
  # the trailing newline the file layer adds on both sides).
  golden = ''{"backgrounds":[],"palette":{"base00":"#101010","base01":"#111111","base02":"#121212","base03":"#131313","base04":"#141414","base05":"#151515","base06":"#161616","base07":"#171717","base08":"#181818","base09":"#191919","base0A":"#1a1a1a","base0B":"#1b1b1b","base0C":"#1c1c1c","base0D":"#1d1d1d","base0E":"#1e1e1e","base0F":"#1f1f1f"},"polarity":"dark","schema":1,"scheme":"vogix16","semantic":{"active":"#1c1c1c","background":"#101010","background_selection":"#121212","background_surface":"#111111","danger":"#1b1b1b","foreground_border":"#141414","foreground_bright":"#171717","foreground_comment":"#131313","foreground_heading":"#161616","foreground_text":"#151515","highlight":"#1e1e1e","link":"#1d1d1d","notice":"#1a1a1a","special":"#1f1f1f","success":"#181818","warning":"#191919"},"theme":"goldtest","variant":"night"}'';

  perScheme = name: doc: [
    (check "${name}: valid JSON with schema 1" (doc.schema == 1))
    (check "${name}: all 16 semantic keys, exactly"
      (builtins.attrNames doc.semantic == v16.semanticKeys))
    (check "${name}: all 16 palette slots, exactly"
      (builtins.attrNames doc.palette == v16.baseSlots))
    (check "${name}: backgrounds present and empty until step 5"
      (doc.backgrounds == [ ]))
    (assertEq "${name}: identity carried" "goldtest" doc.theme)
    (assertEq "${name}: polarity carried" "dark" doc.polarity)
  ];

  tests =
    [
      # The cross-layer byte pin.
      (check "vogix16 output is byte-identical to the golden line (Rust template test's twin)"
        (vogix16Json == golden))
    ]
    ++ perScheme "vogix16" parsed.vogix16
    ++ perScheme "base16" parsed.base16
    ++ perScheme "ansi16" parsed.ansi16
    ++ [
      # vogix16 semantics: slots carry vogix16 MEANING (base08 = success).
      (assertEq "vogix16: success is base08" (hexAt 8) parsed.vogix16.semantic.success)
      (assertEq "vogix16: danger is base0B" (hexAt 11) parsed.vogix16.semantic.danger)

      # base16 semantics: hue mapping (base08 = red = danger).
      (assertEq "base16: danger is base08" (hexAt 8) parsed.base16.semantic.danger)
      (assertEq "base16: success is base0B" (hexAt 11) parsed.base16.semantic.success)
      (assertEq "base16: notice is base09" (hexAt 9) parsed.base16.semantic.notice)
      (assertEq "base16: warning is base0A" (hexAt 10) parsed.base16.semantic.warning)

      # ansi16: the praxis slot fill + the documented fallbacks.
      (assertEq "ansi16: danger is ansi red (color01)" (hexAt 1) parsed.ansi16.semantic.danger)
      (assertEq "ansi16: success is ansi green (color02)" (hexAt 2) parsed.ansi16.semantic.success)
      (assertEq "ansi16: surface falls back to selection_bg" "#050505"
        parsed.ansi16.semantic.background_surface)
      (assertEq "ansi16: heading falls back to foreground" "#020202"
        parsed.ansi16.semantic.foreground_heading)
      (assertEq "ansi16: palette base04 falls back to bright black" (hexAt 8)
        parsed.ansi16.palette.base04)
    ];

  # FORCE every assertion (a failing check THROWS; see behavior/tests.nix).
  passed = builtins.deepSeq tests (builtins.length tests);

in
{
  inherit passed;
  failed = [ ];
  summary = "${toString passed} tests passed";
}
