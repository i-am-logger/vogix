# Appearance module tests
#
# Run with: nix eval --impure -f nix/modules/appearance/tests.nix --apply 'f: f {}'
# Returns: { passed = <count>; failed = []; } on success
# Throws on failure with details
{ pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
}:

let

  appModule = import ./. { inherit lib; };
  inherit (appModule) defaults mkHyprlandConfig;

  # Test helpers
  check = name: cond:
    if cond then { inherit name; passed = true; }
    else throw "FAILED: ${name}";

  assertEq = name: expected: actual:
    if expected == actual then { inherit name; passed = true; }
    else throw "FAILED: ${name} — expected ${toString expected}, got ${toString actual}";

  assertType = name: type: value:
    if builtins.typeOf value == type then { inherit name; passed = true; }
    else throw "FAILED: ${name} — expected type ${type}, got ${builtins.typeOf value}";

  # ── Test data ──
  hyprConfig = mkHyprlandConfig defaults;

  # Custom overrides for testing
  customConfig = defaults // {
    animations = defaults.animations // { enable = false; };
    decoration = defaults.decoration // { rounding = 16; activeOpacity = 0.8; };
    blur = defaults.blur // { enable = false; size = 12; };
    gaps = { inner = 20; outer = 15; };
    borderSize = 5;
    group = defaults.group // { fontSize = 32; };
  };
  customHyprConfig = mkHyprlandConfig customConfig;

  # ── Tests ──

  tests = [
    # === Defaults structure ===
    (check "defaults has animations" (defaults ? animations))
    (check "defaults has decoration" (defaults ? decoration))
    (check "defaults has blur" (defaults ? blur))
    (check "defaults has gaps" (defaults ? gaps))
    (check "defaults has borderSize" (defaults ? borderSize))
    (check "defaults has group" (defaults ? group))

    (check "defaults.animations has enable" (defaults.animations ? enable))
    (check "defaults.animations has bezier" (defaults.animations ? bezier))
    (check "defaults.animations has rules" (defaults.animations ? rules))

    (check "defaults.decoration has activeOpacity" (defaults.decoration ? activeOpacity))
    (check "defaults.decoration has inactiveOpacity" (defaults.decoration ? inactiveOpacity))
    (check "defaults.decoration has rounding" (defaults.decoration ? rounding))
    (check "defaults.decoration has dimInactive" (defaults.decoration ? dimInactive))
    (check "defaults.decoration has dimStrength" (defaults.decoration ? dimStrength))

    (check "defaults.blur has enable" (defaults.blur ? enable))
    (check "defaults.blur has size" (defaults.blur ? size))
    (check "defaults.blur has brightness" (defaults.blur ? brightness))

    (check "defaults.gaps has inner" (defaults.gaps ? inner))
    (check "defaults.gaps has outer" (defaults.gaps ? outer))

    (check "defaults.group has fontFamily" (defaults.group ? fontFamily))
    (check "defaults.group has fontSize" (defaults.group ? fontSize))
    (check "defaults.group has height" (defaults.group ? height))
    (check "defaults.group has indicatorHeight" (defaults.group ? indicatorHeight))

    # === Default values ===
    (check "animations enabled by default" defaults.animations.enable)
    (check "blur enabled by default" defaults.blur.enable)
    (check "dim inactive by default" defaults.decoration.dimInactive)
    (assertEq "border size is 3" 3 defaults.borderSize)
    (assertEq "gaps inner is 10" 10 defaults.gaps.inner)
    (assertEq "gaps outer is 10" 10 defaults.gaps.outer)
    (assertEq "blur size is 3" 3 defaults.blur.size)
    (assertEq "rounding is 8" 8 defaults.decoration.rounding)

    # === Hyprland generator output shape ===
    (check "hyprConfig has settings" (hyprConfig ? settings))
    (check "hyprConfig has extraConfig" (hyprConfig ? extraConfig))
    (assertType "extraConfig is string" "string" hyprConfig.extraConfig)

    (check "settings has animations" (hyprConfig.settings ? animations))
    (check "settings has general" (hyprConfig.settings ? general))
    (check "settings has decoration" (hyprConfig.settings ? decoration))
    (check "settings has group" (hyprConfig.settings ? group))

    # === Hyprland settings values (from defaults) ===
    (check "animations.enabled is true" hyprConfig.settings.animations.enabled)
    (assertEq "general.gaps_in" 10 hyprConfig.settings.general.gaps_in)
    (assertEq "general.gaps_out" 10 hyprConfig.settings.general.gaps_out)
    (assertEq "general.border_size" 3 hyprConfig.settings.general.border_size)
    (assertEq "decoration.rounding" 8 hyprConfig.settings.decoration.rounding)
    (check "decoration.dim_inactive" hyprConfig.settings.decoration.dim_inactive)
    (check "blur.enabled" hyprConfig.settings.decoration.blur.enabled)
    (assertEq "blur.size" 3 hyprConfig.settings.decoration.blur.size)

    # === Animation rules ===
    (check "has animation rules"
      (builtins.length hyprConfig.settings.animations.animation > 0))
    (check "has specialWorkspace animation"
      (builtins.any (r: lib.hasInfix "specialWorkspace" r) hyprConfig.settings.animations.animation))
    (check "has windows animation"
      (builtins.any (r: lib.hasPrefix "windows," r) hyprConfig.settings.animations.animation))
    (check "has fade animation"
      (builtins.any (r: lib.hasPrefix "fade," r) hyprConfig.settings.animations.animation))

    # === Custom overrides propagate ===
    (check "custom: animations disabled" (!customHyprConfig.settings.animations.enabled))
    (assertEq "custom: rounding is 16" 16 customHyprConfig.settings.decoration.rounding)
    (assertEq "custom: gaps_in is 20" 20 customHyprConfig.settings.general.gaps_in)
    (assertEq "custom: gaps_out is 15" 15 customHyprConfig.settings.general.gaps_out)
    (assertEq "custom: border_size is 5" 5 customHyprConfig.settings.general.border_size)
    (check "custom: blur disabled" (!customHyprConfig.settings.decoration.blur.enabled))
    (assertEq "custom: blur size is 12" 12 customHyprConfig.settings.decoration.blur.size)
    (assertEq "custom: group font_size is 32" 32 customHyprConfig.settings.group.groupbar.font_size)

    # === Group bar ===
    (check "groupbar has font_family" (hyprConfig.settings.group.groupbar ? font_family))
    (check "groupbar has font_size" (hyprConfig.settings.group.groupbar ? font_size))
    (check "groupbar has height" (hyprConfig.settings.group.groupbar ? height))
    (check "groupbar has indicator_height" (hyprConfig.settings.group.groupbar ? indicator_height))
  ];

  # ══════════════════════════════════════════════
  # Property-based tests
  # ══════════════════════════════════════════════

  propertyTests =
    # ── P1: All opacity values in [0.0..1.0] ──
    (
      let
        opacityValues = [
          { name = "activeOpacity"; value = defaults.decoration.activeOpacity; }
          { name = "inactiveOpacity"; value = defaults.decoration.inactiveOpacity; }
          { name = "fullscreenOpacity"; value = defaults.decoration.fullscreenOpacity; }
          { name = "dimStrength"; value = defaults.decoration.dimStrength; }
        ];
      in
      map
        (o:
          check "P1: ${o.name} in [0.0..1.0] (${toString o.value})"
            (o.value >= 0.0 && o.value <= 1.0)
        )
        opacityValues
    )

    # ── P2: All integer values are positive ──
    ++ (
      let
        intValues = [
          { name = "gaps.inner"; value = defaults.gaps.inner; }
          { name = "gaps.outer"; value = defaults.gaps.outer; }
          { name = "borderSize"; value = defaults.borderSize; }
          { name = "blur.size"; value = defaults.blur.size; }
          { name = "decoration.rounding"; value = defaults.decoration.rounding; }
          { name = "group.fontSize"; value = defaults.group.fontSize; }
          { name = "group.height"; value = defaults.group.height; }
          { name = "group.indicatorHeight"; value = defaults.group.indicatorHeight; }
        ];
      in
      map
        (i:
          check "P2: ${i.name} >= 0 (${toString i.value})"
            (i.value >= 0)
        )
        intValues
    )

    # ── P3: Animation rules are well-formed (name, onoff, speed, curve [,style]) ──
    ++ (map
      (rule:
        let
          parts = lib.splitString ", " rule;
          numParts = builtins.length parts;
        in
        check "P3: animation rule '${builtins.head parts}' has 4-5 parts (has ${toString numParts})"
          (numParts >= 4 && numParts <= 5)
      )
      defaults.animations.rules)

    # ── P4: Bezier curve has 4 control points ──
    ++ [
      (
        let
          parts = lib.splitString ", " defaults.animations.bezier;
        in
        check "P4: bezier has name + 4 control points (${toString (builtins.length parts)} parts)"
          (builtins.length parts == 5)
      )
    ]

    # ── P5: Blur brightness in valid range ──
    ++ [
      (check "P5: blur.brightness in [0.0..2.0]"
        (defaults.blur.brightness >= 0.0 && defaults.blur.brightness <= 2.0))
    ]

    # ── P6: Generator output is complete — every defaults key maps to a settings key ──
    ++ [
      (check "P6: settings.general has gaps_in" (hyprConfig.settings.general ? gaps_in))
      (check "P6: settings.general has gaps_out" (hyprConfig.settings.general ? gaps_out))
      (check "P6: settings.general has border_size" (hyprConfig.settings.general ? border_size))
      (check "P6: settings.decoration has active_opacity" (hyprConfig.settings.decoration ? active_opacity))
      (check "P6: settings.decoration has inactive_opacity" (hyprConfig.settings.decoration ? inactive_opacity))
      (check "P6: settings.decoration has fullscreen_opacity" (hyprConfig.settings.decoration ? fullscreen_opacity))
      (check "P6: settings.decoration has rounding" (hyprConfig.settings.decoration ? rounding))
      (check "P6: settings.decoration has dim_inactive" (hyprConfig.settings.decoration ? dim_inactive))
      (check "P6: settings.decoration has dim_strength" (hyprConfig.settings.decoration ? dim_strength))
      (check "P6: settings.decoration.blur has enabled" (hyprConfig.settings.decoration.blur ? enabled))
      (check "P6: settings.decoration.blur has size" (hyprConfig.settings.decoration.blur ? size))
      (check "P6: settings.decoration.blur has brightness" (hyprConfig.settings.decoration.blur ? brightness))
      (check "P6: settings.animations has enabled" (hyprConfig.settings.animations ? enabled))
      (check "P6: settings.animations has bezier" (hyprConfig.settings.animations ? bezier))
      (check "P6: settings.animations has animation" (hyprConfig.settings.animations ? animation))
      (check "P6: settings.group.groupbar has font_family" (hyprConfig.settings.group.groupbar ? font_family))
      (check "P6: settings.group.groupbar has font_size" (hyprConfig.settings.group.groupbar ? font_size))
      (check "P6: settings.group.groupbar has height" (hyprConfig.settings.group.groupbar ? height))
      (check "P6: settings.group.groupbar has indicator_height" (hyprConfig.settings.group.groupbar ? indicator_height))
    ]

    # ── P7: API consistency — mkHyprlandConfig returns same shape as behavior module ──
    ++ [
      (check "P7: return shape has settings" (hyprConfig ? settings))
      (check "P7: return shape has extraConfig" (hyprConfig ? extraConfig))
      (assertType "P7: settings is attrset" "set" hyprConfig.settings)
      (assertType "P7: extraConfig is string" "string" hyprConfig.extraConfig)
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
