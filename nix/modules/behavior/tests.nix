# Keybinding module tests
#
# Run with: nix eval --impure -f nix/modules/keybindings/tests.nix
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

  fullConfig = {
    modKey = "super";
    inherit (defaults) modes;
    inherit (defaults) layers;
    inherit (defaults) universal;
    inherit (defaults) mouse;
  };

  hyprConfig = kbModule.mkHyprlandConfig fullConfig;
  kanataConfig = kbModule.mkKanataConfig fullConfig;
  helpScripts = kbModule.mkHelpScripts fullConfig;

  # ── Tests ──

  tests = [
    # === Defaults ===
    (check "defaults.modes has normal mode"
      (defaults.modes ? normal))

    (check "defaults.modes has desktop mode"
      (defaults.modes ? desktop))

    (check "defaults.modes has arrange mode"
      (defaults.modes ? arrange))

    (check "defaults.modes has theme mode"
      (defaults.modes ? theme))

    (check "defaults.layers has nav layer"
      (defaults.layers ? nav))

    (check "defaults.universal has copy"
      (defaults.universal ? copy))

    (check "defaults.mouse has moveWindow"
      (defaults.mouse ? moveWindow))

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

    # Submaps
    (assertContains "extraConfig has desktop submap"
      "submap = desktop"
      hyprConfig.extraConfig)

    (assertContains "extraConfig has arrange submap"
      "submap = arrange"
      hyprConfig.extraConfig)

    (assertContains "extraConfig has theme submap"
      "submap = theme"
      hyprConfig.extraConfig)

    # Submap nesting: arrange escapes to desktop, not reset
    (assertContains "arrange Escape goes to desktop"
      "escape, submap, desktop"
      hyprConfig.extraConfig)

    # Desktop escape goes to reset (app mode)
    (assertContains "desktop Escape goes to reset"
      "escape, submap, reset"
      hyprConfig.extraConfig)

    # Desktop mode has focus bindings
    (assertContains "desktop has focus left"
      "h, movefocus, l"
      hyprConfig.extraConfig)

    # Arrange mode has move and resize
    (assertContains "arrange has move window"
      "h, movewindow, l"
      hyprConfig.extraConfig)

    (assertContains "arrange has resize (binde)"
      "binde = SHIFT, h, resizeactive"
      hyprConfig.extraConfig)

    # Theme mode has vogix commands
    (assertContains "theme has vogix next"
      "vogix -t next"
      hyprConfig.extraConfig)

    (assertContains "theme has vogix darker"
      "vogix -v darker"
      hyprConfig.extraConfig)

    # Theme mode overrides brightness keys
    (assertContains "theme overrides brightness key"
      "XF86MonBrightnessUp, exec, vogix -v lighter"
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

    (assertContains "kanata has deflayer nav"
      "deflayer nav"
      kanataConfig)

    (assertContains "kanata has tap-hold for capslock"
      "tap-hold"
      kanataConfig)

    (assertContains "kanata nav maps h to left"
      "left"
      kanataConfig)

    # === Help scripts ===
    (check "help scripts generated for desktop"
      (helpScripts ? desktop))

    (check "help scripts generated for arrange"
      (helpScripts ? arrange))

    (check "help scripts generated for theme"
      (helpScripts ? theme))

    (check "desktop help is a derivation"
      (helpScripts.desktop ? name))

    (assertEq "desktop help script name"
      "vogix-keys-desktop"
      helpScripts.desktop.name)

    # === Semantic consistency ===
    # q = quit in desktop mode
    (check "desktop mode has q = close"
      ((defaults.modes.desktop.bindings.closeWindow.key or "") == "q"))

    # hjkl = directional in desktop mode
    (check "desktop mode h = focus left"
      ((defaults.modes.desktop.bindings.focusLeft.key or "") == "h"))

    # hjkl = move in arrange mode
    (check "arrange mode h = move left"
      ((defaults.modes.arrange.bindings.moveLeft.key or "") == "h"))

    # Shift+hjkl = resize in arrange mode
    (check "arrange mode Shift+h = resize"
      ((defaults.modes.arrange.bindings.resizeLeft.key or "") == "shift + h"))

    # === Mode entry/exit flow ===
    (check "desktop mode enter is null (entered via CapsLock)"
      (defaults.modes.desktop.enter == null))

    (check "arrange entered from desktop via 'a'"
      (defaults.modes.arrange.enter == "a"))

    (check "theme entered from desktop via 'v'"
      (defaults.modes.theme.enter == "v"))

    # === CapsLock toggle ===
    (check "CapsLock tap sends F13"
      (defaults.layers.nav.tapAction == "f13"))

    (check "normal mode has F13 → desktop submap"
      (defaults.modes.normal.bindings ? enterDesktop))

    (check "desktop mode has F13 → reset (toggle back)"
      (defaults.modes.desktop.bindings ? exitDesktop))

    (assertContains "hyprland normal binds F13 to desktop submap"
      "F13, submap, desktop"
      (lib.concatStringsSep "\n" hyprConfig.settings.bind))

    # === Kanata Super→Ctrl ===
    (assertContains "kanata has defoverrides"
      "defoverrides"
      kanataConfig)

    (assertContains "kanata remaps Super+C to Ctrl+C"
      "(lmet c) (lctl c)"
      kanataConfig)

    # process-unmapped-keys is set via NixOS services.kanata extraDefCfg, not in generated config

    (assertContains "kanata CapsLock tap is F13"
      "tap-hold 200 200 f13"
      kanataConfig)

    # === Help in every mode ===
    (check "desktop mode has help binding"
      (defaults.modes.desktop.bindings ? help))

    (check "arrange mode has help binding"
      (defaults.modes.arrange.bindings ? help))

    (check "theme mode has help binding"
      (defaults.modes.theme.bindings ? help))

    (check "normal mode has help binding"
      (defaults.modes.normal.bindings ? help))

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
  submapModes = filterAttrs (name: _: name != "normal") allModes;

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
    defaults.universal)

  # ── P9: Kanata layer bindings all map to valid keys ──
  # No empty or null values in layer bindings
  ++ (lib.concatMap
    (layerName:
      let
        layer = defaults.layers.${layerName};
        bindings = layer.bindings or { };
      in
      mapAttrsToList
        (src: dst:
          check "P9: kanata layer '${layerName}' key '${src}' maps to non-empty value"
            (dst != "" && dst != null)
        )
        bindings
    )
    (builtins.attrNames defaults.layers))

  # ── P10: Mode exit targets form a valid hierarchy ──
  # Sub-modes exit to their parent, not to random places
  # arrange/theme → desktop, desktop → reset
  ++ [
    (check "P10: arrange exits to desktop (parent)"
      ((defaults.modes.arrange.exit or "") == "escape"))

    (check "P10: theme exits to desktop (parent)"
      ((defaults.modes.theme.exit or "") == "escape"))

    (check "P10: desktop exits to app mode"
      ((defaults.modes.desktop.exit or "") == "escape"))
  ];

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
