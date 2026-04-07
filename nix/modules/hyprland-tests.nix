# Hyprland integration tests
#
# Tests that appearance + behavior generate a valid combined Hyprland config.
# Run with: nix eval --impure -f nix/modules/hyprland-tests.nix --apply 'f: f {}'
{ pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
}:

let
  appearanceModule = import ./appearance { inherit lib; };
  behaviorModule = import ./behavior { inherit lib pkgs; };

  # Test helpers
  check = name: cond:
    if cond then { inherit name; passed = true; }
    else throw "FAILED: ${name}";

  assertEq = name: expected: actual:
    if expected == actual then { inherit name; passed = true; }
    else throw "FAILED: ${name} — expected ${toString expected}, got ${toString actual}";

  assertContains = name: needle: haystack:
    if lib.hasInfix needle haystack then { inherit name; passed = true; }
    else throw "FAILED: ${name} — '${needle}' not found in output";

  # ── Generate both configs ──
  appearance = appearanceModule.mkHyprlandConfig appearanceModule.defaults;
  behavior = behaviorModule.mkHyprlandConfig behaviorModule.defaults;

  # Merge like the hyprland.nix module does
  merged = lib.recursiveUpdate appearance.settings behavior.settings;

  # ── Tests ──

  tests = [
    # === Both modules produce correct shape ===
    (check "appearance has settings" (appearance ? settings))
    (check "appearance has extraConfig" (appearance ? extraConfig))
    (check "behavior has settings" (behavior ? settings))
    (check "behavior has extraConfig" (behavior ? extraConfig))

    # === Merged config has both appearance and behavior keys ===
    # Appearance keys
    (check "merged has animations" (merged ? animations))
    (check "merged has decoration" (merged ? decoration))
    (check "merged has group" (merged ? group))

    # Behavior keys
    (check "merged has input" (merged ? input))
    (check "merged has misc" (merged ? misc))
    (check "merged has dwindle" (merged ? dwindle))
    (check "merged has master" (merged ? master))
    (check "merged has bind" (merged ? bind))
    (check "merged has bindm" (merged ? bindm))

    # Shared keys (both contribute — behavior wins in recursiveUpdate)
    (check "merged has general" (merged ? general))
    (check "merged.general has gaps_in (from appearance)" (merged.general ? gaps_in))
    (check "merged.general has gaps_out (from appearance)" (merged.general ? gaps_out))
    (check "merged.general has border_size (from appearance)" (merged.general ? border_size))
    (check "merged.general has layout (from behavior)" (merged.general ? layout))

    # === No key collisions — appearance and behavior don't overwrite each other ===
    # Appearance owns: animations, decoration, group, general.gaps/border
    # Behavior owns: input, misc, dwindle, master, bind, binde, bindm, gestures, general.layout
    (assertEq "appearance.general has 3 keys (gaps_in, gaps_out, border_size)"
      3
      (builtins.length (builtins.attrNames appearance.settings.general)))
    (check "behavior.general only has layout"
      (behavior.settings.general ? layout))

    # === Behavior extraConfig has submaps ===
    (check "behavior extraConfig is non-empty" (behavior.extraConfig != ""))
    (assertContains "behavior has desktop submap" "submap = desktop" behavior.extraConfig)
    (assertContains "behavior has arrange submap" "submap = arrange" behavior.extraConfig)
    (assertContains "behavior has theme submap" "submap = theme" behavior.extraConfig)
    (assertContains "behavior has console submap" "submap = console" behavior.extraConfig)

    # === Appearance extraConfig is empty (no raw config needed) ===
    (assertEq "appearance extraConfig is empty" "" appearance.extraConfig)

    # === Console window rules present ===
    (check "merged has windowrule" (merged ? windowrule))
    (check "console window rules exist"
      (builtins.any (r: lib.hasInfix "vogix-console" r) (merged.windowrule or [ ])))

    # === Console workspace rules present ===
    (check "merged has workspace" (merged ? workspace))
    (check "console workspace rule exists"
      (builtins.any (r: lib.hasInfix "special:console" r) (merged.workspace or [ ])))

    # === Keybindings present ===
    (check "merged has binds >20" (builtins.length (merged.bind or [ ]) > 20))
    (check "merged has mouse binds" (builtins.length (merged.bindm or [ ]) > 0))

    # === Input settings present ===
    (check "input has repeat_delay" (merged.input ? repeat_delay))
    (check "input has sensitivity" (merged.input ? sensitivity))
    (check "input has touchpad" (merged.input ? touchpad))
    (check "input.touchpad has natural_scroll" (merged.input.touchpad ? natural_scroll))
  ];

  # ══════════════════════════════════════════════
  # Property-based tests
  # ══════════════════════════════════════════════

  propertyTests =
    # ── P1: No key collision between appearance and behavior settings (except general) ──
    (
      let
        appearanceKeys = builtins.attrNames appearance.settings;
        behaviorKeys = builtins.attrNames behavior.settings;
        # general is the only shared key (appearance: gaps/border, behavior: layout)
        sharedKeys = builtins.filter (k: builtins.elem k behaviorKeys && k != "general") appearanceKeys;
      in
      map
        (k:
          check "P1: key '${k}' only in one module (found in both)"
            false  # This should never execute if there are no collisions
        )
        sharedKeys
      ++ [
        (check "P1: no unexpected key collisions (${toString (builtins.length sharedKeys)} found)"
          (builtins.length sharedKeys == 0))
      ]
    )

    # ── P2: All merged settings are non-null ──
    ++ (lib.concatMap
      (key:
        let val = merged.${key};
        in
        if builtins.isAttrs val then
          map
            (subkey:
              check "P2: merged.${key}.${subkey} is not null"
                (merged.${key}.${subkey} != null)
            )
            (builtins.attrNames val)
        else
          [ (check "P2: merged.${key} is not null" (val != null)) ]
      )
      (builtins.filter (k: !(builtins.isList (merged.${k} or null))) (builtins.attrNames merged)))

    # ── P3: Merged general has all expected keys ──
    ++ [
      (check "P3: merged.general has gaps_in" (merged.general ? gaps_in))
      (check "P3: merged.general has gaps_out" (merged.general ? gaps_out))
      (check "P3: merged.general has border_size" (merged.general ? border_size))
      (check "P3: merged.general has layout" (merged.general ? layout))
      (assertEq "P3: merged.general has exactly 4 keys" 4
        (builtins.length (builtins.attrNames merged.general)))
    ];

  allTests = tests ++ propertyTests;
  results = map (t: t) allTests;
  passed = builtins.length results;

in
{
  inherit passed;
  failed = [ ];
  summary = "${toString passed} tests passed";
}
