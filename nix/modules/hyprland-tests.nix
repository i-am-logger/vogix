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

  # ── Generate both configs ──
  appearance = appearanceModule.mkHyprlandConfig appearanceModule.defaults;
  behavior = behaviorModule.mkHyprlandConfig behaviorModule.defaults;

  # A behavior config whose mode graph declares the console passthrough —
  # what gates the console window/workspace rules on.
  behaviorWithConsole = behaviorModule.mkHyprlandConfig {
    modeGraph = {
      root = "app";
      modes = {
        app = { parent = null; type = "normal"; };
        console = { parent = "app"; type = "passthrough"; };
      };
    };
  };

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

    # === Behavior extraConfig carries no submaps by default ===
    # The engine owns the mode statechart; only PASSTHROUGH submaps are ever
    # emitted, and the default mode graph declares none — so the fallback
    # config's extraConfig is empty. (This block used to assert desktop and
    # console submaps from the pre-engine architecture.)
    (assertEq "behavior extraConfig is empty by default" "" behavior.extraConfig)

    # === Appearance extraConfig is empty (no raw config needed) ===
    (assertEq "appearance extraConfig is empty" "" appearance.extraConfig)

    # === Console rules are GRAPH-GATED ===
    # They render only when the mode graph declares a console mode; the
    # default graph is `app`-only, so the default render carries none, and a
    # console-bearing graph carries all four effects + the workspace rule.
    (check "no console rules with the default (app-only) graph"
      (!(builtins.any (r: lib.hasInfix "vogix-console" r) (merged.windowrule or [ ]))
        && !(builtins.any (r: lib.hasInfix "special:console" r) (merged.workspace or [ ]))))
    (check "console window rules exist when the graph declares console"
      (builtins.any (r: lib.hasInfix "vogix-console" r)
        (behaviorWithConsole.settings.windowrule or [ ])))
    (check "console workspace rule exists when the graph declares console"
      (builtins.any (r: lib.hasInfix "special:console" r)
        (behaviorWithConsole.settings.workspace or [ ])))

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

  # ── Lua projection (configType = "lua") ── the HM Lua shapes both modules
  # render; the same merge the hyprland.nix module performs, other dialect.
  appearanceLua = appearanceModule.mkHyprlandLuaConfig appearanceModule.defaults;
  behaviorLua = behaviorModule.mkHyprlandLuaConfig { };
  mergedLuaConfig = lib.recursiveUpdate appearanceLua.settings.config behaviorLua.settings.config;
  firstCurveArgs = (builtins.head appearanceLua.settings.curve)._args;

  luaTests = [
    # One hl.config carrying both modules' sections, with REAL booleans — the
    # Lua parser hard-rejects hyprlang's "yes"/"no"/"on"/"off" strings.
    (check "L1: merged hl.config has appearance + behavior sections"
      (mergedLuaConfig ? decoration && mergedLuaConfig ? input && mergedLuaConfig ? misc))
    (assertEq "L1: natural_scroll is a real bool" true mergedLuaConfig.input.natural_scroll)
    (assertEq "L1: numlock_by_default is a real bool" false mergedLuaConfig.input.numlock_by_default)
    (assertEq "L1: touchpad natural_scroll is a real bool" true mergedLuaConfig.input.touchpad.natural_scroll)

    # The legacy bezier string parses into the hl.curve(name, {type, points}) form.
    (assertEq "L2: curve name" "myBezier" (builtins.head firstCurveArgs))
    (assertEq "L2: curve points" [ [ 0.05 0.9 ] [ 0.1 1.05 ] ]
      (builtins.elemAt firstCurveArgs 1).points)

    # Every legacy animation rule becomes an hl.animation table, field for field.
    (assertEq "L3: animation rule count" 9 (builtins.length appearanceLua.settings.animation))
    (assertEq "L3: last animation keeps its style" "slidefadevert top"
      (lib.last appearanceLua.settings.animation).style)

    # Binds are {_args = [keys dispatcher opts?]} elements with `+`-joined keys
    # and an hl.dsp.* inline dispatcher — never a bare legacy string.
    (check "L4: every bind is an _args element with lua-inline dispatcher"
      (builtins.all
        (b:
          b ? _args
          && builtins.isString (builtins.head b._args)
          && lib.isType "lua-inline" (builtins.elemAt b._args 1))
        behaviorLua.settings.bind))
    (check "L4: some bind carries the drag dispatcher"
      (builtins.any
        (b: (builtins.elemAt b._args 1).expr == "hl.dsp.window.drag()")
        behaviorLua.settings.bind))
    (check "L4: no legacy comma keys leak into the keys string"
      (builtins.all
        (b: !(lib.hasInfix ", " (builtins.head b._args)))
        behaviorLua.settings.bind))
  ];

  allTests = tests ++ propertyTests ++ luaTests;
  # FORCE every assertion: a failing check/assertEq THROWS, so deepSeq makes the
  # eval fail loudly. (The old `map (t: t)` + `length` only counted unforced
  # thunks — it never actually evaluated an assertion, so it always "passed".)
  passed = builtins.deepSeq allTests (builtins.length allTests);

in
{
  inherit passed;
  failed = [ ];
  summary = "${toString passed} tests passed";
}
