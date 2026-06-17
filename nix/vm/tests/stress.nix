# Stress tests - Rapid switching
#
# Tests: Rapid theme/variant switching to catch race conditions
#
{ pkgs
, vogix16Themes
, home-manager
, self
,
}:

let
  testLib = import ./lib.nix {
    inherit
      pkgs
      home-manager
      self
      vogix16Themes
      ;
  };
in
testLib.mkTest "stress" ''
  print("=== Test: Rapid Theme Switching (Stress Test) ===")
  # Rapidly switch themes to ensure no race conditions or state corruption

  themes_cycle = ["yoga", "nordic", "matrix", "desert", "yoga"]
  for i, theme in enumerate(themes_cycle):
      machine.succeed(f"su - vogix -c 'vogix theme set -t {theme}'")
      status = machine.succeed("su - vogix -c 'vogix theme status'")
      assert theme in status.lower(), f"Rapid switch {i}: theme should be {theme}!"
  print("    ✓ Rapid theme switching works")

  # -v dark/light step one toward darker/lighter (aliases of -v darker/-v lighter).
  # yoga has only night/day, so from day they simply alternate night/day.
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t yoga -v day'")
  variants_cycle = [("dark", "night"), ("light", "day"), ("dark", "night"), ("light", "day"), ("dark", "night")]
  for i, (polarity, expected_variant) in enumerate(variants_cycle):
      machine.succeed(f"su - vogix -c 'vogix theme set -v {polarity}'")
      status = machine.succeed("su - vogix -c 'vogix theme status'")
      assert expected_variant in status.lower(), f"Rapid switch {i}: variant should be {expected_variant} (requested {polarity})!"
  print("    ✓ Rapid variant switching works")

  print("\n=== Test: Rapid Combined Switching ===")
  # Rapidly switch both theme and variant together. On a theme switch, -v dark/
  # -v light land on that theme's darkest/lightest variant (its night/day here).
  # Move off yoga first so the first (yoga, dark) entry is a real switch.
  machine.succeed("su - vogix -c 'vogix theme set -t nordic'")
  combinations = [
      ("yoga", "dark", "night"),
      ("nordic", "light", "day"),
      ("matrix", "dark", "night"),
      ("desert", "light", "day"),
      ("yoga", "dark", "night"),
  ]
  for i, (theme, polarity, expected_variant) in enumerate(combinations):
      machine.succeed(f"su - vogix -c 'vogix theme set -t {theme} -v {polarity}'")
      status = machine.succeed("su - vogix -c 'vogix theme status'")
      assert theme in status.lower(), f"Combo switch {i}: theme should be {theme}!"
      assert expected_variant in status.lower(), f"Combo switch {i}: variant should be {expected_variant} (requested {polarity})!"
  print("    ✓ Rapid combined switching works")

  print("\n=== Test: Symlink Integrity After Stress ===")
  # After rapid switching, verify symlinks are still correct

  # Final state should be yoga-night (dark polarity)
  current_link = machine.succeed(f"su - vogix -c 'readlink {current_theme}'").strip()
  assert "yoga" in current_link.lower(), "Symlink should point to yoga after stress test!"
  assert "night" in current_link.lower(), "Symlink should point to night variant after stress test!"
  print("    ✓ Symlink integrity maintained after stress test")

  # Verify config is accessible and correct
  alacritty = machine.succeed("su - vogix -c 'cat ~/.config/alacritty/alacritty.toml'")
  yoga_night_bg = all_themes['yoga']['night']['base00'].lower()
  assert yoga_night_bg in alacritty.lower(), "Config should have yoga night colors after stress test!"
  print("    ✓ Config content correct after stress test")

  print("\n✓ Stress test passed!")

  print("\n" + "="*60)
  print("STRESS TESTS PASSED!")
  print("="*60)
''
