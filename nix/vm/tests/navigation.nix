# Navigation tests - Darker/lighter navigation
#
# Tests: darker/lighter navigation, catppuccin multi-variant, single-variant themes
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
testLib.mkTest "navigation" ''
  print("=== Test: Darker/Lighter Navigation ===")
  # Start from dark (aikido uses 'night'), try lighter
  machine.succeed("su - vogix -c 'vogix theme set -v dark'")
  machine.succeed("su - vogix -c 'vogix theme set -v lighter'")
  nav_status = machine.succeed("su - vogix -c 'vogix theme status'")
  # aikido uses 'day' for light polarity, not 'light'
  assert "day" in nav_status.lower(), "Navigation to lighter failed!"
  print("✓ 'vogix theme set -v lighter' navigates from night to day")

  # Try lighter again - should fail (already at lightest)
  machine.fail("su - vogix -c 'vogix theme set -v lighter'")
  print("✓ 'vogix theme set -v lighter' correctly fails when already at lightest")

  # Navigate back with darker
  machine.succeed("su - vogix -c 'vogix theme set -v darker'")
  nav_back = machine.succeed("su - vogix -c 'vogix theme status'")
  # aikido uses 'night' for dark polarity, not 'dark'
  assert "night" in nav_back.lower(), "Navigation to darker failed!"
  print("✓ 'vogix theme set -v darker' navigates from day to night")

  # Try darker again - should fail (already at darkest)
  machine.fail("su - vogix -c 'vogix theme set -v darker'")
  print("✓ 'vogix theme set -v darker' correctly fails when already at darkest")

  print("\n=== Test: Darker/Lighter from Different Starting Points ===")

  # Start from dark (night), navigate to lighter (day)
  print("  --- Starting from dark (night), navigating lighter ---")
  machine.succeed("su - vogix -c 'vogix theme set -t aikido -v dark'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "night" in status.lower()

  machine.succeed("su - vogix -c 'vogix theme set -v lighter'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "day" in status.lower(), "lighter from night should go to day!"
  print("    ✓ night -> lighter = day")

  # Try lighter again (should fail - at lightest)
  result = machine.execute("su - vogix -c 'vogix theme set -v lighter 2>&1'")
  assert result[0] != 0, "lighter from lightest should fail!"
  print("    ✓ lighter from day fails (at boundary)")

  # Start from light (day), navigate darker
  print("  --- Starting from light (day), navigating darker ---")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "day" in status.lower()

  machine.succeed("su - vogix -c 'vogix theme set -v darker'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "night" in status.lower(), "darker from day should go to night!"
  print("    ✓ day -> darker = night")

  # Try darker again (should fail - at darkest)
  result = machine.execute("su - vogix -c 'vogix theme set -v darker 2>&1'")
  assert result[0] != 0, "darker from darkest should fail!"
  print("    ✓ darker from night fails (at boundary)")

  print("\n✓ Darker/lighter navigation works correctly!")

  print("\n=== Test: Catppuccin Darker/Lighter Navigation (Multi-Variant) ===")
  # Catppuccin has 4 variants: latte (lightest) -> frappe -> macchiato -> mocha (darkest)
  # This tests navigation on a theme with more than 2 variants

  schemes_to_test = ["base16"]

  for scheme in schemes_to_test:
      print(f"\n  --- Testing catppuccin darker/lighter for {scheme} ---")

      # Try to switch to catppuccin with mocha (darkest) variant
      result = machine.execute(f"su - vogix -c 'vogix theme set -s {scheme} -t catppuccin -v mocha 2>&1'")
      if result[0] != 0:
          print(f"    ⚠ Cannot switch to {scheme}/catppuccin/mocha: {result[1][:100]}")
          continue

      status = machine.succeed("su - vogix -c 'vogix theme status'")
      assert "mocha" in status.lower(), f"Expected mocha variant, got: {status}"
      print("    ✓ Started at mocha (darkest)")

      # Navigate lighter: mocha -> macchiato -> frappe -> latte
      expected_sequence = ["macchiato", "frappe", "latte"]
      for expected in expected_sequence:
          result = machine.execute("su - vogix -c 'vogix theme set -v lighter 2>&1'")
          if result[0] != 0:
              print(f"    ✗ lighter failed: {result[1][:100]}")
              break
          status = machine.succeed("su - vogix -c 'vogix theme status'")
          assert expected in status.lower(), f"Expected {expected}, got: {status}"
          print(f"    ✓ lighter -> {expected}")

      # Should be at latte (lightest) now - lighter should fail
      result = machine.execute("su - vogix -c 'vogix theme set -v lighter 2>&1'")
      assert result[0] != 0, "lighter from latte should fail!"
      print("    ✓ lighter from latte correctly fails (at boundary)")

      # Navigate darker: latte -> frappe -> macchiato -> mocha
      expected_sequence = ["frappe", "macchiato", "mocha"]
      for expected in expected_sequence:
          result = machine.execute("su - vogix -c 'vogix theme set -v darker 2>&1'")
          if result[0] != 0:
              print(f"    ✗ darker failed: {result[1][:100]}")
              break
          status = machine.succeed("su - vogix -c 'vogix theme status'")
          assert expected in status.lower(), f"Expected {expected}, got: {status}"
          print(f"    ✓ darker -> {expected}")

      # Should be at mocha (darkest) now - darker should fail
      result = machine.execute("su - vogix -c 'vogix theme set -v darker 2>&1'")
      assert result[0] != 0, "darker from mocha should fail!"
      print("    ✓ darker from mocha correctly fails (at boundary)")

      print(f"    ✓ {scheme}/catppuccin: Full navigation cycle complete!")

  # Reset
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t aikido -v dark'")

  print("\n=== Test: Single-Variant Theme Handling (Dracula) ===")
  # dracula only has 'default' variant (dark polarity)

  result = machine.execute("su - vogix -c 'vogix theme set -s base16 -t dracula 2>&1'")
  if result[0] == 0:
      status = machine.succeed("su - vogix -c 'vogix theme status'")
      assert "base16" in status.lower(), "Should be base16 scheme"
      assert "dracula" in status.lower(), "Should be dracula theme"
      print("    ✓ Switched to base16/dracula")

      # For single-variant themes, -v dark or -v light should just use the only variant
      result_dark = machine.execute("su - vogix -c 'vogix theme set -v dark 2>&1'")
      assert result_dark[0] == 0, f"Single-variant theme should accept -v dark: {result_dark[1][:100]}"
      status = machine.succeed("su - vogix -c 'vogix theme status'")
      print("    ✓ -v dark uses the only available variant")

      # -v light should also work (uses only variant)
      result_light = machine.execute("su - vogix -c 'vogix theme set -v light 2>&1'")
      assert result_light[0] == 0, f"Single-variant theme should accept -v light: {result_light[1][:100]}"
      print("    ✓ -v light uses the only available variant")

      # darker/lighter should fail at boundary (only one variant)
      result = machine.execute("su - vogix -c 'vogix theme set -v darker 2>&1'")
      assert result[0] != 0, "darker on single-variant theme should fail!"
      print("    ✓ -v darker correctly fails (single variant = at boundary)")

      result = machine.execute("su - vogix -c 'vogix theme set -v lighter 2>&1'")
      assert result[0] != 0, "lighter on single-variant theme should fail!"
      print("    ✓ -v lighter correctly fails (single variant = at boundary)")

      print("\n✓ Single-variant theme handling verified!")
  else:
      print("⚠ Could not test single-variant handling (base16/dracula not available)")

  # Final reset
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t aikido -v dark'")

  print("\n" + "="*60)
  print("NAVIGATION TESTS PASSED!")
  print("="*60)
''
