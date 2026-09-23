# Smoke tests - Quick sanity checks
#
# Tests: Binary exists, status command, list command, activation setup,
# login shells running nothing from vogix, the machine module's machine.json,
# drop zone and vogix-machine unit, and the session's theme restore unit,
# whose refresh publishes the owner's palette. These should run fast and
# catch obvious failures early.
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
  assert "yoga" in output or "Available themes:" in output
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

  print("\n=== Test: Login Shells Run Nothing From Vogix ===")
  # bash is enabled for the test user, so home-manager writes both login
  # files; neither may name the vogix binary.
  for profile in ("/home/vogix/.profile", "/home/vogix/.bash_profile"):
      machine.succeed(f"test -s {profile}")
      machine.fail(f"grep -F bin/vogix {profile}")
  print("✓ ~/.profile and ~/.bash_profile do not run vogix")

  # A refresh swaps current-theme by renaming a new link over it, so a login
  # shell that ran one would leave a different inode behind.
  def current_theme_inode():
      return machine.succeed(f"stat -c %i {current_theme}").strip()

  inode = current_theme_inode()
  machine.succeed("su - vogix -c true")
  assert current_theme_inode() == inode, "a login shell replaced current-theme: it ran a theme refresh"
  print("✓ a login shell leaves current-theme untouched")

  print("\n=== Test: Machine Surfaces From the NixOS Module ===")
  # vogix.enable with one vogix user: that user is the machine owner, and the
  # console is on, so vogix-machine owns the VT palette. Nothing has
  # published yet: login shells publish nothing.
  machine.succeed(
      "${pkgs.jq}/bin/jq -e '.schema == 1 and .owner == \"vogix\" and .dropZone == \"/var/lib/vogix/machine\"'"
      " /etc/vogix/machine.json"
  )
  assert machine.succeed("stat -c %U /var/lib/vogix/machine").strip() == "vogix"
  machine.wait_for_unit("vogix-machine.service")
  assert machine.succeed("systemctl is-active vogix-machine.service").strip() == "active"
  machine.fail("test -e /var/lib/vogix/machine/palette.json")
  print("✓ machine.json names the owner, the drop zone is theirs, vogix-machine is active, nothing is published")

  print("\n=== Test: Session Theme Restore Unit ===")
  units = "/home/vogix/.config/systemd/user"
  unit = machine.succeed(f"cat {units}/vogix-theme-restore.service")
  assert "Type=oneshot" in unit, unit
  assert "RemainAfterExit" not in unit, unit
  assert re.search(r"^ExecStart=/nix/store/[^/]+/bin/vogix theme refresh$", unit, re.M), unit
  assert re.search(r"^After=graphical-session\.target$", unit, re.M), unit
  assert re.search(r"^WantedBy=graphical-session\.target$", unit, re.M), unit
  machine.succeed(f"test -e {units}/graphical-session.target.wants/vogix-theme-restore.service")
  print("✓ vogix-theme-restore.service is a oneshot wanted by graphical-session.target")

  # This VM has no graphical session, so the unit is started directly: the
  # refresh it runs applies the theme and logs to the user journal.
  def user_systemctl(args):
      return machine.succeed(f"su - vogix -c 'XDG_RUNTIME_DIR=/run/user/1000 systemctl --user {args}'")

  machine.wait_for_unit("user@1000.service")
  user_systemctl("start vogix-theme-restore.service")
  assert current_theme_inode() != inode, "the restore unit did not refresh the theme"
  machine.wait_until_succeeds(
      "journalctl -o cat _SYSTEMD_USER_UNIT=vogix-theme-restore.service | grep -F 'Applied: yoga-night'"
  )
  result = user_systemctl("show vogix-theme-restore.service -p Result -p ActiveState")
  assert "Result=success" in result and "ActiveState=inactive" in result, result
  print("✓ starting vogix-theme-restore applies the theme, logs 'Applied:' and leaves the unit inactive")

  # The restore's refresh ran as the machine owner, so it published.
  palette = "/var/lib/vogix/machine/palette.json"
  assert machine.succeed(f"stat -c '%U %a' {palette}").strip() == "vogix 644"
  machine.succeed(f"${pkgs.jq}/bin/jq -e '.theme.name == \"yoga\" and .theme.variant == \"night\"' {palette}")
  print("✓ the restore published the owner's palette")

  print("\n" + "="*60)
  print("SMOKE TESTS PASSED!")
  print("="*60)
''
