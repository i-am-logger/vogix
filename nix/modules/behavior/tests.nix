# Behavior module tests — engine-resolved `vogix` paradigm + overlay
#
# Run with: nix eval --impure -f nix/modules/behavior/tests.nix --apply 'f: f {}'
# Returns: { passed = <count>; failed = []; } on success
# Throws on failure with details (every assertion is FORCED — see `allTests`).
#
# Scope after the flip: defaults.nix no longer encodes the WM-navigation. It
# carries only the user's OVERLAY (launch/system/media) plus the paradigm
# SELECTION (`paradigm = "vogix"`); the engine resolves the selection into the
# nav modes + mode graph (guarded byte-for-byte by the Rust side,
# `src/input/catalog.rs::engine_resolved_vogix_equals_the_live_layout`). So these
# tests guard the NIX contract: the overlay content, and that `mkSchemaJSON`
# emits {paradigm, overlay} with NO modeGraph (its absence is what triggers
# engine resolution) and no nav leak.
{ pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
}:

let
  inherit (lib) mapAttrsToList;

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

  assertExcludes = name: needle: haystack:
    if !(lib.hasInfix needle haystack) then { inherit name; passed = true; }
    else throw "FAILED: ${name} — '${needle}' unexpectedly present in output";

  # ── Test data ──
  fullConfig = kbModule.mkGeneratorConfig defaults;
  hyprConfig = kbModule.mkHyprlandConfig defaults;
  schemaJSON = kbModule.mkSchemaJSON defaults;

  # The OVERLAY (launch/system/media). The WM-nav is NOT here — the engine
  # resolves it from `keybindings.paradigm`.
  app = defaults.modes.app.bindings;
  appBinds = builtins.attrValues app;
  hasAction = needle: builtins.any (b: lib.hasInfix needle (b.action or "")) appBinds;

  # ── Tests ──

  tests = [
    # === Mode graph — the overlay root only (a SINGLE flat `app` mode) ===
    (check "defaults.modeGraph exists"
      (defaults ? modeGraph))

    (assertEq "modeGraph root is app"
      "app"
      defaults.modeGraph.root)

    (assertEq "modeGraph has exactly 1 mode (app — the overlay root)"
      1
      (builtins.length (builtins.attrNames defaults.modeGraph.modes)))

    (check "modeGraph.app has null parent (root)"
      (defaults.modeGraph.modes.app.parent == null))

    (assertEq "modeGraph.app type is normal"
      "normal"
      defaults.modeGraph.modes.app.type)

    # The paradigm's sub-modes (if any) are resolved by the engine, never here.
    (check "modeGraph has no desktop / move / resize / console sub-modes"
      (!(defaults.modeGraph.modes ? desktop)
        && !(defaults.modeGraph.modes ? move)
        && !(defaults.modeGraph.modes ? resize)
        && !(defaults.modeGraph.modes ? console)))

    # === Modes ===
    (check "defaults.modes has the app mode"
      (defaults.modes ? app))

    (check "defaults.modes has ONLY the app mode"
      (builtins.attrNames defaults.modes == [ "app" ]))

    # === Keybindings — paradigm SELECTION, engine owns the catalog ===
    (assertEq "paradigm is the house default (vogix)"
      "vogix"
      defaults.keybindings.paradigm)

    (check "the `paradigms` catalog is GONE (the engine owns it now)"
      (!(defaults.keybindings ? paradigms)))

    (check "no CapsLock / dual-role interaction layer"
      (defaults.keybindings.layers == { }))

    (check "defaults.keybindings.mouse has moveWindow"
      (defaults.keybindings.mouse ? moveWindow))

    # === Schema emission — the FLIP contract (mirror of catalog.rs) ===
    (assertContains "rendered schema selects the vogix paradigm"
      ''"paradigm":"vogix"''
      schemaJSON)

    (assertExcludes "rendered schema OMITS modeGraph (its absence triggers engine resolution)"
      "modeGraph"
      schemaJSON)

    (assertExcludes "rendered schema carries no inline remap (the engine resolves vogix→copy-paste)"
      "copy-paste"
      schemaJSON)

    (assertExcludes "rendered schema is NOT modal — no submap entry"
      "submap"
      schemaJSON)

    # The overlay is the user's own non-paradigm keys — NOT the WM nav.
    (assertExcludes "no WM-nav (movefocus) leaks into the emitted overlay"
      "movefocus"
      schemaJSON)

    (assertExcludes "no WM-nav (workspace) leaks into the emitted overlay"
      ''"workspace,''
      schemaJSON)

    # === Generator config ===
    (check "fullConfig.modes has app (root mode)"
      (fullConfig.modes ? app))

    (check "fullConfig carries the modKey"
      (fullConfig.modKey or null == "super"))

    (check "fullConfig has mouse"
      (fullConfig ? mouse))

    # === Hyprland generator — the engine-OFF fallback (overlay only) ===
    (check "hyprland settings has bind"
      (hyprConfig.settings ? bind))

    (check "hyprland settings has bindm"
      (hyprConfig.settings ? bindm))

    (check "hyprland mainMod is SUPER"
      (hyprConfig.settings."$mainMod" == "SUPER"))

    (assertEq "hyprland has 2 mouse bindings"
      2
      (builtins.length hyprConfig.settings.bindm))

    # The fallback is the OVERLAY (launch/system/media), NOT the full keymap —
    # the WM nav lives in the engine. So it is small (the recovery surface), not
    # the >40-bind layout the old encode-in-Nix scheme produced.
    (check "the fallback is the overlay, not the full keymap (< 40 binds)"
      (builtins.length hyprConfig.settings.bind < 40))

    # Recovery: Super+Return + the launcher are enough to restart vogix-input if
    # the engine ever fails to start.
    (check "fallback keeps the terminal recovery bind (Super+Return)"
      (lib.hasInfix "exec, $TERMINAL" (lib.concatStringsSep "\n" hyprConfig.settings.bind)))

    (check "fallback keeps the launcher recovery bind"
      (lib.hasInfix "LAUNCHER" (lib.concatStringsSep "\n" hyprConfig.settings.bind)))

    # No sub-modes anywhere — the engine owns input, and there are no submaps.
    (check "extraConfig has no desktop / move / resize submap"
      (!(lib.hasInfix "submap = desktop" hyprConfig.extraConfig)
        && !(lib.hasInfix "submap = move" hyprConfig.extraConfig)
        && !(lib.hasInfix "submap = resize" hyprConfig.extraConfig)))

    (check "no submap-entry binds leak into the root binds"
      (!(builtins.any (b: lib.hasInfix "submap" b) hyprConfig.settings.bind)))

    (check "NO bindr anywhere"
      (!(lib.hasInfix "bindr" hyprConfig.extraConfig)))

    # === Overlay content — the REAL job of defaults.nix now ===
    (assertEq "Launch: Super+Return = terminal ($TERMINAL)"
      "exec, $TERMINAL"
      (app.terminal.action or ""))
    (assertEq "Launch: Super+Return key"
      "super + return"
      (app.terminal.key or ""))
    (assertEq "Launch: Super+E = browser ($BROWSER)"
      "exec, $BROWSER"
      (app.browser.action or ""))
    (assertEq "Launch: Super+Space = launcher (env $LAUNCHER, walker fallback)"
      "exec, \${LAUNCHER:-walker}"
      (app.launcher.action or ""))

    # Help is now an ENGINE view — `vogix input keys` materializes from the
    # resolved schema (this overlay + the paradigm nav), replacing the Nix script.
    (assertEq "System: Super+/ = show keybindings (engine view)"
      "exec, vogix input keys"
      (app.help.action or ""))
    (assertEq "System: Super+/ key"
      "super + slash"
      (app.help.key or ""))

    (assertEq "System: F12 = toggle system console"
      "Toggle system console"
      (app.console.description or ""))
    (assertEq "System: Super+D = dismiss notification (makoctl)"
      "exec, makoctl dismiss"
      (app.dismissNotification.action or ""))
    (assertEq "System: Super+Shift+D = dismiss all"
      "exec, makoctl dismiss --all"
      (app.dismissAll.action or ""))
    (assertEq "System: Super+Z = session undo"
      "super + z"
      (app.undoSession.key or ""))
    (assertEq "System: Print = screenshot → clipboard"
      "exec, grimblast --notify copy area"
      (app.screenshotClip.action or ""))

    (assertEq "Media: XF86AudioRaiseVolume = volume up"
      "XF86AudioRaiseVolume"
      (app.volumeUp.key or ""))
    (assertEq "Media: XF86MonBrightnessUp = brighter"
      "exec, brightnessctl set 5%+"
      (app.brightnessUp.action or ""))

    # === The nav is the ENGINE's job — it must NOT be in the overlay ===
    (check "no focus/move/resize WM-nav action leaks into the overlay"
      (!(hasAction "movefocus") && !(hasAction "swapwindow") && !(hasAction "resizeactive")))
    (check "no workspace-switch nav leaks into the overlay"
      (!(hasAction "workspace, ") && !(hasAction "movetoworkspace")))
    (check "no window-state nav (close/fullscreen/pseudo) leaks into the overlay"
      (!(app ? closeWindow) && !(app ? floatPin) && !(app ? toggleGroup)))
  ];

  # ══════════════════════════════════════════════
  # Property-based tests — invariants for ANY config
  # ══════════════════════════════════════════════

  allModes = defaults.modes;
  modeNames = builtins.attrNames allModes;

  # ── P2: No duplicate keys within a mode ──
  propertyTests = (mapAttrsToList
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

  # ── P3: All submap references resolve (overlay has none, but stays honest) ──
  ++ (lib.concatMap
    (modeName:
      let
        mode = allModes.${modeName};
        bindings = mode.bindings or { };
        submapRefs = lib.concatMap
          (binding:
            let parts = lib.splitString ", " (binding.action or "");
            in
            if builtins.head parts == "submap" && builtins.length parts == 2
            then [ (lib.last parts) ] else [ ]
          )
          (builtins.attrValues bindings);
        userRefs = builtins.filter (r: r != "reset") submapRefs;
      in
      map
        (ref: check "P3: submap '${ref}' referenced in '${modeName}' exists" (allModes ? ${ref}))
        userRefs
    )
    modeNames)

  # ── P4: Every binding has a description (the help system needs it) ──
  ++ (lib.concatMap
    (modeName:
      let bindings = allModes.${modeName}.bindings or { };
      in
      mapAttrsToList
        (bindName: binding:
          check "P4: '${modeName}.${bindName}' has description"
            ((binding.description or "") != "")
        )
        bindings
    )
    modeNames)

  # ── P10: Mode graph hierarchy is consistent ──
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
    defaults.modeGraph.modes);

  allTests = tests ++ propertyTests;
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
