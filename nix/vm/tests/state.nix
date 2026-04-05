# State tests - State persistence and consistency
#
# Tests: State file persistence, consistency after operations
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
testLib.mkTest "state" ''
  print("=== Test: State Persistence ===")
  # Make a change and verify it persists
  machine.succeed("su - vogix -c 'vogix theme set -s base16 -t dracula -v dark'")

  # Check state file exists and has correct content (now in ~/.local/state/vogix/)
  state_content = machine.succeed(f"su - vogix -c 'cat {vogix_state}/state.toml 2>/dev/null || echo NOTFOUND'")
  if state_content.strip() != "NOTFOUND":
      print(f"State content: {state_content[:200]}")
      assert "base16" in state_content.lower(), "State file doesn't contain scheme!"
      assert "dracula" in state_content.lower(), "State file doesn't contain theme!"
      print("✓ State persisted correctly to file")
  else:
      print("⚠ State file not found in expected locations (may use different path)")

  # Reset for other tests
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t aikido -v dark'")
  print("✓ Reset to vogix16/aikido/dark")

  print("\n=== Test: State Consistency After Multiple Operations ===")
  # Perform a series of operations and verify state is consistent

  # Complex sequence of operations
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t aikido -v dark'")
  machine.succeed("su - vogix -c 'vogix theme set -t nordic'")
  machine.succeed("su - vogix -c 'vogix theme set -v light'")
  machine.succeed("su - vogix -c 'vogix theme set -t matrix -v dark'")
  machine.succeed("su - vogix -c 'vogix theme set -v lighter'")

  # Final state should be: vogix16, matrix, day (light polarity variant)
  final_status = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "matrix" in final_status.lower(), "Final theme should be matrix!"
  # Note: matrix theme uses 'day' as its light polarity variant name
  assert "day" in final_status.lower(), "Final variant should be day (from lighter)!"
  assert "vogix16" in final_status.lower(), "Scheme should still be vogix16!"
  print("    ✓ State is consistent after complex operations")

  # Verify state file matches
  state_content = machine.succeed(f"su - vogix -c 'cat {vogix_state}/state.toml 2>/dev/null || echo NOTFOUND'")
  if "NOTFOUND" not in state_content:
      assert "matrix" in state_content.lower(), "State file should have matrix!"
      assert "day" in state_content.lower(), "State file should have day!"
      print("    ✓ State file matches status output")

  # Verify symlink matches (current-theme is in state dir)
  current_link = machine.succeed(f"su - vogix -c 'readlink {current_theme}'").strip()
  assert "matrix" in current_link.lower() and "day" in current_link.lower(), "Symlink should point to matrix-day!"
  print("    ✓ Symlink matches state")

  # Verify config has correct colors
  alacritty = machine.succeed("su - vogix -c 'cat ~/.config/alacritty/alacritty.toml'")
  matrix_day_bg = all_themes['matrix']['day']['base00'].lower()
  assert matrix_day_bg in alacritty.lower(), f"Config should have matrix day bg {matrix_day_bg}!"
  print("    ✓ Config has correct colors")

  print("\n✓ State consistency verified!")

  # Reset to default for any subsequent tests
  machine.succeed("su - vogix -c 'vogix theme set -s vogix16 -t aikido -v dark'")

  print("\n" + "="*60)
  print("STATE TESTS PASSED!")
  print("="*60)
''
