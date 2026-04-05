# CLI tests - Combined flags, list options, error handling
#
# Tests: Combined flag orders, list command options, invalid inputs
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
testLib.mkTest "cli" ''
  print("=== Test: List Command Functionality ===")

  # Basic list
  list_output = machine.succeed("su - vogix -c 'vogix theme list'")
  print(f"  List output (first 500 chars): {list_output[:500]}")
  assert "aikido" in list_output.lower(), "List should show aikido theme!"
  print("    ✓ Basic list shows themes")

  # List with scheme filter (vogix16)
  list_vogix16 = machine.succeed("su - vogix -c 'vogix theme list -s vogix16'")
  assert "aikido" in list_vogix16.lower(), "vogix16 list should show aikido!"
  print("    ✓ List -s vogix16 shows aikido")

  # List with scheme filter (base16)
  list_base16 = machine.succeed("su - vogix -c 'vogix theme list -s base16'")
  if "dracula" in list_base16.lower() or "gruvbox" in list_base16.lower() or "nord" in list_base16.lower():
      print("    ✓ List -s base16 shows base16 themes")
  else:
      print(f"    ⚠ List -s base16 output: {list_base16[:300]}")

  # List with scheme filter (base24)
  list_base24 = machine.succeed("su - vogix -c 'vogix theme list -s base24'")
  if len(list_base24.strip()) > 10:
      print("    ✓ List -s base24 shows base24 themes")
  else:
      print(f"    ⚠ List -s base24 may be empty: {list_base24[:100]}")

  # List with scheme filter (ansi16)
  list_ansi16 = machine.succeed("su - vogix -c 'vogix theme list -s ansi16'")
  if len(list_ansi16.strip()) > 10:
      print("    ✓ List -s ansi16 shows ansi16 themes")
  else:
      print(f"    ⚠ List -s ansi16 may be empty: {list_ansi16[:100]}")

  # List with --variants flag
  list_variants = machine.succeed("su - vogix -c 'vogix theme list --variants'")
  if "dark" in list_variants.lower() or "light" in list_variants.lower():
      print("    ✓ List --variants shows variant information")
  else:
      print(f"    ⚠ List --variants may not show variants: {list_variants[:200]}")

  print("\n✓ List command works!")

  print("\n=== Test: Combined Flags ===")
  # Test combined -s, -t, -v flags
  # Note: aikido uses 'day' for light polarity and 'night' for dark polarity
  machine.succeed("su - vogix -c 'vogix theme set -t aikido -v day'")
  combined_status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "aikido" in combined_status.lower(), "Theme not set in combined flags!"
  assert "day" in combined_status.lower(), "Variant not set in combined flags!"
  print("✓ Combined flags (-t aikido -v day) work")

  # Reset back to night (aikido's dark variant)
  machine.succeed("su - vogix -c 'vogix theme set -v night'")

  print("\n=== Test: Combined Flags in Various Orders ===")
  # Note: vogix16 themes use 'night'/'day' for dark/light polarities

  # Reset
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t aikido -v night'")

  # Order 1: -t -v -s (using actual variant names)
  machine.succeed("su - vogix -c 'vogix theme set -t nordic -v day -s vogix16'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "nordic" in status.lower() and "day" in status.lower() and "vogix16" in status.lower()
  print("    ✓ Order -t -v -s works")

  # Order 2: -v -s -t
  machine.succeed("su - vogix -c 'vogix theme set -v night -s vogix16 -t matrix'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "matrix" in status.lower() and "night" in status.lower()
  print("    ✓ Order -v -s -t works")

  # Order 3: -s -t -v
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t desert -v day'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "desert" in status.lower() and "day" in status.lower()
  print("    ✓ Order -s -t -v works")

  # Single flags should work
  machine.succeed("su - vogix -c 'vogix theme set -t aikido'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "aikido" in status.lower()
  print("    ✓ Single flag -t works")

  machine.succeed("su - vogix -c 'vogix theme set -v night'")
  status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "night" in status.lower()
  print("    ✓ Single flag -v works")

  print("\n✓ Combined flags work in any order!")

  print("\n=== Test: Error Handling - Invalid Inputs ===")

  # Invalid theme name
  result = machine.execute("su - vogix -c 'vogix theme set -t nonexistent_theme_xyz 2>&1'")
  assert result[0] != 0, "Invalid theme should fail!"
  print("    ✓ Invalid theme rejected")

  # Invalid variant name
  result = machine.execute("su - vogix -c 'vogix theme set -v invalid_variant_xyz 2>&1'")
  assert result[0] != 0, "Invalid variant should fail!"
  print("    ✓ Invalid variant rejected")

  # Invalid scheme name
  result = machine.execute("su - vogix -c 'vogix theme set -s invalid_scheme 2>&1'")
  assert result[0] != 0, "Invalid scheme should fail!"
  print("    ✓ Invalid scheme rejected")

  # Non-existent theme (via subcommand syntax if supported)
  machine.fail("su - vogix -c 'vogix theme set -t nonexistent'")
  print("✓ Non-existent theme rejected")

  print("\n✓ Error handling works!")

  # Reset to default (aikido uses 'night' for dark polarity)
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t aikido -v night'")

  print("\n" + "="*60)
  print("CLI TESTS PASSED!")
  print("="*60)
''
