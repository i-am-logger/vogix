# Smoke tests - Quick sanity checks
#
# Tests: Binary exists, status command, list command, activation setup
# These should run fast and catch obvious failures early.
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
testLib.mkTest "smoke" ''
  print("=== Test: Vogix Binary Exists ===")
  machine.succeed("which vogix")
  print("✓ vogix binary found")

  print("\n=== Test: Check Status Command ===")
  output = machine.succeed("su - vogix -c 'vogix theme status'")
  assert "theme:" in output
  assert "variant:" in output
  assert "scheme:" in output
  print("✓ Status command works")
  print(f"Output: {output}")

  print("\n=== Test: List Themes ===")
  output = machine.succeed("su - vogix -c 'vogix theme list'")
  assert "aikido" in output or "Available themes:" in output
  print("✓ List command works")
  print(f"Output: {output}")

  print("\n=== Test: No Config Files in ~/.config/vogix/ ===")
  result = machine.execute("su - vogix -c 'test -d ~/.config/vogix'")
  if result[0] != 0:
      print("✓ ~/.config/vogix/ does not exist (correct - config in ~/.local/state/vogix/)")
  else:
      print("⚠ WARNING: ~/.config/vogix/ exists but shouldn't")

  print("\n=== Test: Config.toml Generated in State Directory ===")
  result = machine.execute(f"su - vogix -c 'test -f {vogix_state}/config.toml'")
  if result[0] == 0:
      print("✓ config.toml exists in state directory")
  else:
      raise AssertionError("FAILED: config.toml not found in ~/.local/state/vogix/")

  print("\n=== Test: Home-Manager Activation Set Up Vogix ===")
  # New architecture uses home.activation instead of systemd service
  # Verify that theme packages exist in ~/.local/share/vogix/themes/
  machine.succeed(f"su - vogix -c 'test -d {vogix_themes}'")
  print("✓ Themes directory exists")

  # Verify current-theme symlink exists in state directory
  machine.succeed(f"su - vogix -c 'test -L {current_theme}'")
  print("✓ current-theme symlink exists")

  # Verify at least one app config symlink was created
  alacritty_link = machine.execute("su - vogix -c 'test -L ~/.config/alacritty/alacritty.toml'")
  if alacritty_link[0] == 0:
      print("✓ App config symlinks created by activation")
  else:
      print("⚠ alacritty config symlink not found (may not be enabled)")

  print("\n=== Test: Shell Completions ===")
  output = machine.succeed("su - vogix -c 'vogix completions bash | head -5'")
  assert "_vogix" in output or "completion" in output
  print("✓ Shell completions work")

  print("\n=== Test: Vogix Refresh in Login Shell Profile ===")
  # Check that vogix refresh is in bash profile or .profile (since bash is enabled in test VM)
  # Home-manager may put profileExtra in .profile which is sourced by .bash_profile
  profile_content = machine.succeed("su - vogix -c 'cat ~/.profile 2>/dev/null || echo NOTFOUND'")
  bash_profile_content = machine.succeed("su - vogix -c 'cat ~/.bash_profile 2>/dev/null || echo NOTFOUND'")

  if "vogix refresh" in profile_content or "vogix refresh" in bash_profile_content:
      print("✓ vogix refresh found in login shell profile")
  else:
      print(f".profile content: {profile_content[:500]}")
      print(f".bash_profile content: {bash_profile_content[:500]}")
      raise AssertionError("FAILED: vogix refresh not found in shell profile")

  print("\n" + "="*60)
  print("SMOKE TESTS PASSED!")
  print("="*60)
''
