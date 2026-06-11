# Behavior module tests — flat Super-combo scheme
#
# Run with: nix eval --impure -f nix/modules/behavior/tests.nix --apply 'f: f {}'
# Returns: { passed = <count>; failed = []; } on success
# Throws on failure with details
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

  # ── Test data ──
  fullConfig = kbModule.mkGeneratorConfig defaults;
  hyprConfig = kbModule.mkHyprlandConfig defaults;

  app = defaults.modes.app.bindings;

  # ── Tests ──

  tests = [
    # === Mode graph — a SINGLE flat mode ===
    (check "defaults.modeGraph exists"
      (defaults ? modeGraph))

    (assertEq "modeGraph root is app"
      "app"
      defaults.modeGraph.root)

    (assertEq "modeGraph has exactly 1 mode (app — flat, no sub-modes)"
      1
      (builtins.length (builtins.attrNames defaults.modeGraph.modes)))

    (check "modeGraph.app has null parent (root)"
      (defaults.modeGraph.modes.app.parent == null))

    (assertEq "modeGraph.app type is normal"
      "normal"
      defaults.modeGraph.modes.app.type)

    # The modal sub-modes are gone (folded into flat Super-combos).
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

    # === Keybindings — flat, no CapsLock, no remap ===
    (check "paradigm is the user's own (default)"
      (defaults.keybindings.paradigm == "default"))

    (check "default paradigm uses NO Super remap (Super is the WM modifier)"
      (defaults.keybindings.paradigms.default.remap == "none"))

    (check "default paradigm carries the shared (flat) modes"
      (defaults.keybindings.paradigms.default.modes == defaults.modes))

    (check "no CapsLock / dual-role interaction layer"
      (defaults.keybindings.layers == { }))

    (check "the windows/mac/emacs paradigms are not present (TODO: flat variants)"
      (!(defaults.keybindings.paradigms ? windows)
        && !(defaults.keybindings.paradigms ? mac)
        && !(defaults.keybindings.paradigms ? emacs)))

    (check "defaults.keybindings.mouse has moveWindow"
      (defaults.keybindings.mouse ? moveWindow))

    (assertContains "rendered schema JSON carries the remap preset (none)"
      "none"
      (kbModule.mkSchemaJSON defaults))

    (check "rendered schema is NOT modal — no CapsLock submap entry"
      (!(lib.hasInfix "submap" (kbModule.mkSchemaJSON defaults))))

    # === Generator config ===
    (check "fullConfig.modes has app (root mode)"
      (fullConfig.modes ? app))

    (check "fullConfig carries the modKey"
      (fullConfig.modKey or null == "super"))

    (check "fullConfig has mouse"
      (fullConfig ? mouse))

    # === Hyprland generator ===
    (check "hyprland settings has bind"
      (hyprConfig.settings ? bind))

    (check "hyprland settings has bindm"
      (hyprConfig.settings ? bindm))

    (check "hyprland mainMod is SUPER"
      (hyprConfig.settings."$mainMod" == "SUPER"))

    (assertEq "hyprland has 2 mouse bindings"
      2
      (builtins.length hyprConfig.settings.bindm))

    (check "the flat app emits a full keymap (>40 binds)"
      (builtins.length hyprConfig.settings.bind > 40))

    # No sub-modes anywhere — the engine owns input, and there are no submaps.
    (check "extraConfig has no desktop / move / resize submap"
      (!(lib.hasInfix "submap = desktop" hyprConfig.extraConfig)
        && !(lib.hasInfix "submap = move" hyprConfig.extraConfig)
        && !(lib.hasInfix "submap = resize" hyprConfig.extraConfig)))

    (check "no submap-entry binds leak into the root binds"
      (!(builtins.any (b: lib.hasInfix "submap" b) hyprConfig.settings.bind)))

    (check "NO bindr anywhere"
      (!(lib.hasInfix "bindr" hyprConfig.extraConfig)))

    # === Semantic consistency — the flat scheme, verbatim from bindings.conf ===
    (assertEq "Super+Q = close window"
      "super + q"
      (app.closeWindow.key or ""))

    (assertEq "Super+H = focus left (j=up, k=down — non-vim)"
      "movefocus, l"
      (app.focusLeft.action or ""))

    (assertEq "Super+J = focus UP (your original, not vim)"
      "movefocus, u"
      (app.focusUp.action or ""))

    (assertEq "Super+Shift+H = MOVE window left (swapwindow)"
      "swapwindow, l"
      (app.swapLeft.action or ""))

    (assertEq "Super+Shift+H key is super + shift + h"
      "super + shift + h"
      (app.swapLeft.key or ""))

    (assertEq "Ctrl+Shift+L = RESIZE wider (resizeactive)"
      "resizeactive, 30 0"
      (app.resizeRight.action or ""))

    (assertEq "Ctrl+Shift+L key is ctrl + shift + l"
      "ctrl + shift + l"
      (app.resizeRight.key or ""))

    (assertEq "Super+Y = float + pin (the yuiop row)"
      "super + y"
      (app.floatPin.key or ""))

    (assertEq "Super+O = toggle split (dwindle layoutmsg, not a top-level dispatcher)"
      "layoutmsg, togglesplit"
      (app.toggleSplit.action or ""))

    (assertEq "Super+U = toggle group"
      "togglegroup,"
      (app.toggleGroup.action or ""))

    (assertEq "Super+P = pseudotile"
      "pseudo,"
      (app.pseudo.action or ""))

    (assertEq "Super+C = Chat workspace"
      "workspace, Chat"
      (app.workspaceChat.action or ""))

    (assertEq "Super+Ctrl+3 = send window to workspace 3"
      "movetoworkspace, 3"
      (app.moveToWs3.action or ""))

    (assertEq "Super+Shift+3 = send window to workspace 3 (silent)"
      "movetoworkspacesilent, 3"
      (app.moveSilent3.action or ""))

    # === Super+letter is INTENTIONAL here (no macOS remap) ===
    (check "the flat scheme uses Super+letter directly as WM binds"
      (lib.hasInfix "SUPER, q" (lib.concatStringsSep "\n" hyprConfig.settings.bind)))
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

  # ── P3: All submap references resolve (flat scheme has none, but stays honest) ──
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
  results = map (t: t) allTests;
  passed = builtins.length results;

in
{
  inherit passed;
  failed = [ ];
  summary = "${toString passed} tests passed";
}
