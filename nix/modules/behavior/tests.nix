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
  kanataConfig = kbModule.mkKanataConfig defaults;
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

    (check "defaults._superCtrlRemaps has copy"
      (defaults._superCtrlRemaps ? copy))

    (check "defaults.keybindings.mouse has moveWindow"
      (defaults.keybindings.mouse ? moveWindow))

    # === Generator config (flat structure via mkGeneratorConfig) ===
    (check "fullConfig.modes has app (root mode)"
      (fullConfig.modes ? app))

    (check "fullConfig has universal remaps"
      (fullConfig ? universal))

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

    # Single WM submap (desktop). No arrange/theme submaps.
    (assertContains "extraConfig has desktop submap"
      "submap = desktop"
      hyprConfig.extraConfig)

    (check "extraConfig has no arrange submap"
      (!(lib.hasInfix "submap = arrange" hyprConfig.extraConfig)))

    (check "extraConfig has no theme submap"
      (!(lib.hasInfix "submap = theme" hyprConfig.extraConfig)))

    # Escape exits to app via the NATIVE submap dispatcher (synchronous).
    (assertContains "desktop escape exits natively to reset"
      ", escape, submap, reset"
      hyprConfig.extraConfig)

    # Desktop: arrows/hjkl = focus; m/r enter the move/resize sub-modes.
    (assertContains "desktop focus left (h, no modifier)"
      ", h, movefocus, l"
      hyprConfig.extraConfig)

    (assertContains "desktop: m enters move sub-mode (native submap)"
      ", m, submap, move"
      hyprConfig.extraConfig)

    (assertContains "desktop: r enters resize sub-mode (native submap)"
      ", r, submap, resize"
      hyprConfig.extraConfig)

    (assertContains "desktop send-and-follow (Shift+3)"
      "SHIFT, 3, movetoworkspace, 3"
      hyprConfig.extraConfig)

    (assertContains "desktop has Tab = togglesplit"
      "tab, layoutmsg, togglesplit"
      hyprConfig.extraConfig)

    # move/resize sub-modes do the actual window ops on bare arrows/hjkl.
    (assertContains "move sub-mode: h moves window left"
      ", h, movewindow, l"
      hyprConfig.extraConfig)

    (assertContains "resize sub-mode: l resizes wider"
      ", l, resizeactive, 40 0"
      hyprConfig.extraConfig)

    # === Kanata generator ===
    (check "kanata config is not null"
      (kanataConfig != null))

    (assertContains "kanata has defsrc"
      "defsrc"
      kanataConfig)

    (assertContains "kanata has deflayer default"
      "deflayer default"
      kanataConfig)

    (assertContains "kanata caps tap-hold present"
      "tap-hold-press"
      kanataConfig)

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

    # === CapsLock dual-role (click = toggle, hold = momentary) ===
    (check "CapsLock click sends F24 (toggle submap)"
      (defaults.keybindings.layers.desktopToggle.tapAction == "f24"))

    (check "CapsLock hold enters via F23 (press)"
      (defaults.keybindings.layers.desktopToggle.holdAction == "f23"))

    (check "CapsLock hold-RELEASE exits via F22 (separate press, not bindr)"
      (defaults.keybindings.layers.desktopToggle.holdReleaseAction == "f22"))

    (check "app mode has F24 toggle entry"
      (defaults.modes.app.bindings ? enterDesktopToggle))

    (check "app mode has F23 hold entry"
      (defaults.modes.app.bindings ? enterDesktopHold))

    (check "desktop mode has F24 toggle exit"
      (defaults.modes.desktop.bindings ? exitDesktopToggle))

    # Entry uses the NATIVE submap dispatcher (synchronous) — both the hold
    # (F23) and click (F24) paths.
    (assertContains "hyprland app: F23 (hold) enters desktop natively"
      "F23, submap, desktop"
      (lib.concatStringsSep "\n" hyprConfig.settings.bind))

    (assertContains "hyprland app: F24 (click) enters desktop natively"
      "F24, submap, desktop"
      (lib.concatStringsSep "\n" hyprConfig.settings.bind))

    # ── THE CRITICAL INVARIANT: hold-release exit is a press-`bind`, NOT bindr ──
    # Hyprland's `bindr` (release-bind) does NOT fire when the key's press entered
    # the submap (proven via evtest: F23-up fired but the submap never reset). So
    # kanata taps a SEPARATE exit key (F22) on release and Hyprland exits with a
    # normal press-`bind`. This is the fix for "it stays in the mode".
    (assertContains "submap exits on F22 PRESS (reliable)"
      "bind = , F22, submap, reset"
      hyprConfig.extraConfig)

    (check "NO bindr anywhere — release-binds across submaps are unreliable"
      (!(lib.hasInfix "bindr" hyprConfig.extraConfig)))

    # kanata must emit the F22 exit key on hold-release (via on-release fakekey).
    (assertContains "kanata taps exit key on hold-release"
      "on-release-fakekey"
      kanataConfig)

    (assertContains "kanata defines the submap-exit fake key as F22"
      "deffakekeys vogixsubmapexit f22"
      kanataConfig)

    # ── PROPERTY: submap transitions are native/synchronous, never exec ──
    # The momentary-mode invariant. A transition via `exec hyprctl dispatch
    # submap` is async (~6-7ms) and the next key leaks to the app → "fast mode
    # doesn't work". Every line that binds a transition key (F23 / F24 / escape)
    # MUST use ", submap, " and MUST NOT contain exec.
    (
      let
        allBinds = hyprConfig.settings.bind
          ++ (lib.splitString "\n" hyprConfig.extraConfig);
        transitionLines = builtins.filter
          (l: lib.hasInfix ", F23, " l
            || lib.hasInfix ", F24, " l
            || lib.hasInfix ", F22, " l
            || lib.hasInfix ", escape, " l)
          allBinds;
        native = builtins.all
          (l: lib.hasInfix ", submap, " l && !(lib.hasInfix "exec" l))
          transitionLines;
      in
      check "PROPERTY: all submap transitions are native (no async exec on F23/F24/escape)"
        (transitionLines != [ ] && native)
    )

    # === exitAfter: launch actions auto-return to app ===
    # (exitAfter is OFF the momentary critical path — async exec reset is fine.)
    (check "desktop terminal launch has exitAfter"
      (defaults.modes.desktop.bindings.openTerminal.exitAfter or false))

    (assertContains "desktop terminal resets submap then launches"
      "dispatch submap reset ; $TERMINAL"
      hyprConfig.extraConfig)

    (check "desktop close window has exitAfter"
      (defaults.modes.desktop.bindings.closeWindow.exitAfter or false))

    # Non-exec exitAfter (close): dispatch the action, then reset to app.
    (assertContains "desktop close dispatches killactive then resets"
      "hyprctl dispatch killactive ; hyprctl dispatch submap reset"
      hyprConfig.extraConfig)

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

    # === Kanata Super→Ctrl ===
    (assertContains "kanata has defoverrides"
      "defoverrides"
      kanataConfig)

    (assertContains "kanata remaps Super+C to Ctrl+C"
      "(lmet c) (lctl c)"
      kanataConfig)

    # kanata caps is dual-role: tap-hold-press routing click=f24 / hold=f23
    (assertContains "kanata caps uses tap-hold-press"
      "tap-hold-press"
      kanataConfig)

    (assertContains "kanata caps click routes to f24"
      "f24"
      kanataConfig)

    (assertContains "kanata caps hold routes to f23"
      "f23"
      kanataConfig)

    (check "kanata caps does NOT use Scroll_Lock (lock-key bindr unreliable)"
      (!(lib.hasInfix "slck" kanataConfig)))

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

  # ── P8: Universal remaps are all Super→Ctrl ──
  # The pattern must be consistent
  ++ (mapAttrsToList
    (name: entry:
      let
        from = parseUniversalCombo (entry.from or "");
        to = parseUniversalCombo (entry.to or "");
      in
      check "P8: universal '${name}' maps Super→Ctrl"
        (from != null && to != null && from.mod == "super" && to.mod == "ctrl")
    )
    defaults._superCtrlRemaps)

  # ── P9: Kanata layer bindings all map to valid keys ──
  # No empty or null values in layer bindings
  ++ (lib.concatMap
    (layerName:
      let
        layer = defaults.keybindings.layers.${layerName};
        bindings = layer.bindings or { };
      in
      mapAttrsToList
        (src: dst:
          check "P9: kanata layer '${layerName}' key '${src}' maps to non-empty value"
            (dst != "" && dst != null)
        )
        bindings
    )
    (builtins.attrNames defaults.keybindings.layers))

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

  # Helper used by P8
  parseUniversalCombo = combo:
    let
      parts = map lib.trim (lib.splitString "+" combo);
      lower = map lib.toLower parts;
    in
    if builtins.length parts == 2 then {
      mod = builtins.head lower;
      key = lib.toLower (lib.last parts);
    }
    else null;

  allTests = tests ++ propertyTests;
  results = map (t: t) allTests;
  passed = builtins.length results;

in
{
  inherit passed;
  failed = [ ];
  summary = "${toString passed} tests passed";
}
