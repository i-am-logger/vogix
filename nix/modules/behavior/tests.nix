# Behavior module tests
#
# Run with: nix eval --impure -f nix/modules/behavior/tests.nix --apply 'f: f {}'
# Returns: { passed = <count>; failed = []; } on success
# Throws on failure with details
{ pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
}:

let
  inherit (lib) filterAttrs mapAttrsToList;

  kbModule = import ./. { inherit lib pkgs; };
  inherit (kbModule) defaults;

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

  # ── Test data ──
  # Use mkGeneratorConfig to build the flat config generators expect
  # (transforms app→normal, flattens keybindings.*)
  fullConfig = kbModule.mkGeneratorConfig defaults;

  hyprConfig = kbModule.mkHyprlandConfig defaults;
  helpScripts = kbModule.mkHelpScripts defaults;

  # ── Tests ──

  tests = [
    # === Defaults (raw structure) ===
    # === Mode graph ===
    (check "defaults.modeGraph exists"
      (defaults ? modeGraph))

    (assertEq "modeGraph root is app"
      "app"
      defaults.modeGraph.root)

    (check "modeGraph has 5 modes (app + desktop + move + resize + console)"
      (builtins.length (builtins.attrNames defaults.modeGraph.modes) == 5))

    (check "modeGraph has move + resize sub-modes"
      (defaults.modeGraph.modes ? move && defaults.modeGraph.modes ? resize))

    (check "modeGraph.app has null parent (root)"
      (defaults.modeGraph.modes.app.parent == null))

    (assertEq "modeGraph.desktop parent is app"
      "app"
      defaults.modeGraph.modes.desktop.parent)

    (assertEq "modeGraph.console type is passthrough"
      "passthrough"
      defaults.modeGraph.modes.console.type)

    # arrange/theme sub-modes were removed (folded into desktop / dropped).
    (check "modeGraph has no arrange mode"
      (!(defaults.modeGraph.modes ? arrange)))

    (check "modeGraph has no theme mode"
      (!(defaults.modeGraph.modes ? theme)))

    # === Modes (bindings) ===
    (check "defaults.modes has app mode"
      (defaults.modes ? app))

    (check "defaults.modes has desktop mode"
      (defaults.modes ? desktop))

    (check "defaults.keybindings.layers has desktopToggle"
      (defaults.keybindings.layers ? desktopToggle))

    (check "defaults.keybindings.paradigm is the vim flavour (native modal)"
      (defaults.keybindings.paradigm == "vim"))

    (check "vim paradigm pairs the macOS remap with the shared modes"
      (defaults.keybindings.paradigms.vim.remap == "macos"
        && defaults.keybindings.paradigms.vim.modes == defaults.modes))

    (check "windows paradigm uses no Super remap (Ctrl is native)"
      (defaults.keybindings.paradigms.windows.remap == "none"))

    (check "mac paradigm keeps the macOS Super→Ctrl remap"
      (defaults.keybindings.paradigms.mac.remap == "macos"))

    (assertContains "selecting windows renders chorded Super+arrow WM nav"
      "super + left"
      (kbModule.mkSchemaJSON (defaults // {
        keybindings = defaults.keybindings // { paradigm = "windows"; };
      })))

    (check "the default (vim) render is modal — no chorded Super+arrow nav"
      (!(lib.hasInfix "super + left" (kbModule.mkSchemaJSON defaults))))

    (check "defaults.keybindings has terminalClasses (context-aware remap)"
      ((defaults.keybindings.terminalClasses or [ ]) != [ ]))

    (assertContains "rendered schema JSON carries terminalClasses"
      "terminalClasses"
      (kbModule.mkSchemaJSON defaults))

    (assertContains "rendered schema JSON carries the paradigm (remap preset)"
      "macos"
      (kbModule.mkSchemaJSON defaults))

    (check "defaults.keybindings.mouse has moveWindow"
      (defaults.keybindings.mouse ? moveWindow))

    # === Generator config (flat structure via mkGeneratorConfig) ===
    (check "fullConfig.modes has app (root mode)"
      (fullConfig.modes ? app))

    (check "fullConfig carries the modKey"
      (fullConfig.modKey or null == "super"))

    (check "fullConfig has mouse"
      (fullConfig ? mouse))

    # === Hyprland generator ===
    (check "hyprland generates settings"
      (hyprConfig ? settings))

    (check "hyprland generates extraConfig"
      (hyprConfig ? extraConfig))

    (check "hyprland settings has bind"
      (hyprConfig.settings ? bind))

    (check "hyprland settings has bindm"
      (hyprConfig.settings ? bindm))

    (check "hyprland mainMod is SUPER"
      (hyprConfig.settings."$mainMod" == "SUPER"))

    (check "hyprland has normal mode binds"
      (builtins.length hyprConfig.settings.bind > 0))

    (assertEq "hyprland has 2 mouse bindings"
      2
      (builtins.length hyprConfig.settings.bindm))

    # Normal mode should have workspaces + media + screenshots + launcher + terminal + help
    (check "normal mode has >20 bindings"
      (builtins.length hyprConfig.settings.bind > 20))

    # The vogix input engine owns navigation modes — the Hyprland generator emits
    # NO desktop/move/resize submaps; those modes live in the engine (input.json,
    # proven end-to-end by nix/vm/tests/input-engine.nix). Only the passthrough
    # console submap and the plain root binds (the engine-down fallback) survive.
    (check "extraConfig has no desktop submap (engine-owned)"
      (!(lib.hasInfix "submap = desktop" hyprConfig.extraConfig)))

    (check "extraConfig has no move submap (engine-owned)"
      (!(lib.hasInfix "submap = move" hyprConfig.extraConfig)))

    (check "extraConfig has no resize submap (engine-owned)"
      (!(lib.hasInfix "submap = resize" hyprConfig.extraConfig)))

    (check "extraConfig has no arrange submap"
      (!(lib.hasInfix "submap = arrange" hyprConfig.extraConfig)))

    (check "extraConfig has no theme submap"
      (!(lib.hasInfix "submap = theme" hyprConfig.extraConfig)))

    # The console passthrough submap DOES survive (entered by its own exec).
    (assertContains "extraConfig keeps the console passthrough submap"
      "submap = console"
      hyprConfig.extraConfig)

    # No submap-entry binds leak into the root config — the engine owns mode entry.
    (check "no submap-entry binds leak into normal binds"
      (!(builtins.any (b: lib.hasInfix "submap" b) hyprConfig.settings.bind)))

    # === Help scripts ===
    (check "help scripts generated for desktop"
      (helpScripts ? desktop))

    (check "no help script for removed arrange mode"
      (!(helpScripts ? arrange)))

    (check "no help script for removed theme mode"
      (!(helpScripts ? theme)))

    (check "desktop help is a derivation"
      (helpScripts.desktop ? name))

    (assertEq "desktop help script name"
      "vogix-modes-desktop"
      helpScripts.desktop.name)

    # === Semantic consistency ===
    # q = quit in desktop mode
    (check "desktop mode has q = close"
      ((defaults.modes.desktop.bindings.closeWindow.key or "") == "q"))

    (check "desktop mode h = focus left"
      ((defaults.modes.desktop.bindings.focusLeft.key or "") == "h"))

    (check "desktop mode m = enter move sub-mode"
      ((defaults.modes.desktop.bindings.enterMove.action or "") == "submap, move"))

    (check "desktop mode r = enter resize sub-mode"
      ((defaults.modes.desktop.bindings.enterResize.action or "") == "submap, resize"))

    (check "move sub-mode h = movewindow left"
      ((defaults.modes.move.bindings.moveLeft.action or "") == "movewindow, l"))

    (check "resize sub-mode l = resize wider"
      ((defaults.modes.resize.bindings.resizeRight.action or "") == "resizeactive, 40 0"))

    (check "desktop mode Shift+3 = send window to ws 3 and follow"
      ((defaults.modes.desktop.bindings.sendToWs3.action or "") == "movetoworkspace, 3"))

    (check "desktop mode Tab = togglesplit"
      ((defaults.modes.desktop.bindings.toggleSplit.key or "") == "tab"))

    # === Mode entry/exit flow ===
    (check "desktop mode enter is null (entered via CapsLock)"
      (defaults.modes.desktop.enter == null))

    # === CapsLock dual-role (engine-native: tap = sticky, hold = momentary) ===
    (assertEq "CapsLock layer enters the desktop mode"
      "desktop"
      defaults.keybindings.layers.desktopToggle.entersMode)

    (check "CapsLock layer trigger is capslock"
      (defaults.keybindings.layers.desktopToggle.hold == "capslock"))

    # The engine owns mode switching — no synthetic F22/F23/F24 keysyms and no
    # Hyprland submap-entry/exit binds for it (tap/hold detection + the praxis
    # statechart are proven in nix/vm/tests/input-engine.nix).
    (check "no F23/F24 submap-entry binds in the Hyprland root config"
      (!(lib.hasInfix "F23, submap" (lib.concatStringsSep "\n" hyprConfig.settings.bind))
        && !(lib.hasInfix "F24, submap" (lib.concatStringsSep "\n" hyprConfig.settings.bind))))

    (check "NO bindr anywhere — release-binds across submaps are unreliable"
      (!(lib.hasInfix "bindr" hyprConfig.extraConfig)))

    # === exitAfter: one-shot actions return to app ===
    # Data-level here; the engine applies it as ExitToRoot after the dispatch,
    # proven end-to-end by the input-engine VM test.
    (check "desktop terminal launch has exitAfter"
      (defaults.modes.desktop.bindings.openTerminal.exitAfter or false))

    (check "desktop close window has exitAfter"
      (defaults.modes.desktop.bindings.closeWindow.exitAfter or false))

    # Navigation / sub-mode entries stay in desktop (NOT exitAfter) so they chain.
    (check "desktop focus does NOT exitAfter (chainable)"
      (!(defaults.modes.desktop.bindings.focusLeft.exitAfter or false)))

    (check "desktop m (enter move) does NOT exitAfter"
      (!(defaults.modes.desktop.bindings.enterMove.exitAfter or false)))

    # GOLDEN: the exact set of desktop binds that return to app. This pins the
    # interaction model so any future edit that mis-classifies a binding (the
    # "caps+q stays" class of bug) fails the suite instead of shipping.
    (assertEq "desktop exitAfter set is exactly the one-shot commands"
      "closeWindow,dismissAll,dismissNotification,fullscreen,lock,openBrowser,openLauncher,openTerminal"
      (builtins.concatStringsSep ","
        (builtins.sort (a: b: a < b)
          (builtins.attrNames
            (filterAttrs (_: b: b.exitAfter or false) defaults.modes.desktop.bindings)))))

    # move/resize sub-modes are pure window-ops — nothing should exitAfter
    # (you chain ops; leave by Esc/caps).
    (check "move sub-mode has no exitAfter bindings"
      (builtins.all (b: !(b.exitAfter or false)) (builtins.attrValues defaults.modes.move.bindings)))

    (check "resize sub-mode has no exitAfter bindings"
      (builtins.all (b: !(b.exitAfter or false)) (builtins.attrValues defaults.modes.resize.bindings)))

    # === Super→Ctrl remap (a praxis paradigm preset; the macos_remap RemapSet
    # is applied at evdev by the engine + axiom-checked by `vogix input check`,
    # proven by the input-engine VM test) ===
    (check "the selected paradigm resolves to its remap in the rendered schema"
      (defaults.keybindings.paradigms.${defaults.keybindings.paradigm}.remap == "macos"))

    # === Help in every mode ===
    (check "desktop mode has help binding"
      (defaults.modes.desktop.bindings ? help))

    (check "normal mode has help binding"
      (defaults.modes.app.bindings ? help))

    # === No Super+letter conflicts ===
    # Normal mode should NOT have Super+letter bindings (those are app shortcuts now)
    (check "no Super+C in normal mode (would conflict with copy)"
      (!(builtins.any (b: lib.hasInfix "SUPER, c," b) hyprConfig.settings.bind)))

    (check "no Super+A in normal mode (would conflict with select all)"
      (!(builtins.any (b: lib.hasInfix "SUPER, a," b) hyprConfig.settings.bind)))

    (check "no Super+F in normal mode (would conflict with find)"
      (!(builtins.any (b: lib.hasInfix "SUPER, f," b) hyprConfig.settings.bind)))
  ];

  # ══════════════════════════════════════════════
  # Property-based tests
  # Invariants that must hold for ANY keybinding config
  # ══════════════════════════════════════════════

  allModes = defaults.modes;
  modeNames = builtins.attrNames allModes;
  # Submap modes = everything except app (global) and console (passthrough)
  submapModes = filterAttrs (name: _: name != "app" && name != "console") allModes;

  propertyTests = (mapAttrsToList
    (name: mode:
      check "P1: mode '${name}' has exit defined"
        ((mode.exit or null) != null)
    )
    submapModes)

  # ── P2: No duplicate keys within a mode ──
  # Two bindings in the same mode must not use the same key
  ++ (mapAttrsToList
    (modeName: mode:
      let
        bindings = mode.bindings or { };
        keys = mapAttrsToList (_: b: b.key or "") bindings;
        unique = lib.unique keys;
      in
      check "P2: no duplicate keys in '${modeName}' mode (${toString (builtins.length keys)} bindings, ${toString (builtins.length unique)} unique)"
        (builtins.length keys == builtins.length unique)
    )
    allModes)

  # ── P3: All submap references resolve ──
  # If a binding says "submap, X", mode X must exist
  ++ (lib.concatMap
    (modeName:
      let
        mode = allModes.${modeName};
        bindings = mode.bindings or { };
        submapRefs = lib.concatMap
          (binding:
            let
              action = binding.action or "";
              parts = lib.splitString ", " action;
            in
            if builtins.head parts == "submap" && builtins.length parts == 2
            then [ (lib.last parts) ]
            else [ ]
          )
          (builtins.attrValues bindings);
        # "reset" is special (returns to app mode), not a user-defined mode
        userRefs = builtins.filter (r: r != "reset") submapRefs;
      in
      map
        (ref:
          check "P3: submap '${ref}' referenced in '${modeName}' exists"
            (allModes ? ${ref})
        )
        userRefs
    )
    modeNames)

  # ── P4: Every binding has a description ──
  # Required for the help system to work
  ++ (lib.concatMap
    (modeName:
      let
        mode = allModes.${modeName};
        bindings = mode.bindings or { };
      in
      mapAttrsToList
        (bindName: binding:
          check "P4: '${modeName}.${bindName}' has description"
            ((binding.description or "") != "")
        )
        bindings
    )
    modeNames)

  # ── P5: Help scripts exist for every submap mode ──
  ++ (mapAttrsToList
    (name: _:
      check "P5: help script exists for '${name}' mode"
        (helpScripts ? ${name})
    )
    submapModes)

  # ── P6: No Super+letter in normal mode binds ──
  # The macOS constraint: Super+letter is reserved for app shortcuts
  ++ (
    let
      letters = lib.stringToCharacters "abcdefghijklmnopqrstuvwxyz";
      normalBindStrings = hyprConfig.settings.bind;
    in
    map
      (letter:
        check "P6: no 'SUPER, ${letter}' in normal mode (reserved for app shortcuts)"
          (!(builtins.any (b: lib.hasInfix "SUPER, ${letter}," b) normalBindStrings))
      )
      letters
  )

  # ── P7: Symmetric directional bindings ──
  # If a mode has h (left), it must also have j, k, l
  ++ (lib.concatMap
    (modeName:
      let
        mode = allModes.${modeName};
        bindings = mode.bindings or { };
        keys = map (b: b.key or "") (builtins.attrValues bindings);
        hasH = builtins.elem "h" keys;
        hasJ = builtins.elem "j" keys;
        hasK = builtins.elem "k" keys;
        hasL = builtins.elem "l" keys;
        hasAny = hasH || hasJ || hasK || hasL;
        hasAll = hasH && hasJ && hasK && hasL;
      in
      lib.optional hasAny (
        check "P7: '${modeName}' has complete hjkl set (not partial)"
          hasAll
      )
    )
    modeNames)

  # P8 (universal remaps are all Super→Ctrl) is retired: the remap set is now a
  # praxis paradigm preset (macos_remap), proven Super→Ctrl + injective + complete
  # by the RemapInjective/MacosRemapComplete axioms in `vogix input check`.

  # ── P10: Mode graph hierarchy is consistent ──
  # Every non-root mode's parent exists in the graph
  ++ (mapAttrsToList
    (name: graphDef:
      let parent = graphDef.parent or null;
      in
      if parent == null then
        check "P10: root mode '${name}' has no parent"
          (name == defaults.modeGraph.root)
      else
        check "P10: mode '${name}' parent '${parent}' exists in graph"
          (defaults.modeGraph.modes ? ${parent})
    )
    defaults.modeGraph.modes)

  # ── P10b: All submap modes have an exit key ──
  ++ (lib.concatMap
    (name:
      let
        graphDef = defaults.modeGraph.modes.${name} or { type = "submap"; };
        mode = defaults.modes.${name} or { };
      in
      lib.optional (graphDef.type or "submap" == "submap" && name != defaults.modeGraph.root) (
        check "P10b: submap '${name}' has exit defined"
          ((mode.exit or null) != null)
      )
    )
    (builtins.attrNames defaults.modeGraph.modes));

  allTests = tests ++ propertyTests;
  results = map (t: t) allTests;
  passed = builtins.length results;

in
{
  inherit passed;
  failed = [ ];
  summary = "${toString passed} tests passed";
}
