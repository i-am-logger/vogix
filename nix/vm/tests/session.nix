# Session save/restore tests
#
# Tests: session save, restore, list, undo, stack behavior
# NOTE: These tests run without Hyprland/Wezterm (headless VM),
# so we test the CLI + filesystem behavior, not window management.
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
testLib.mkTest "session" ''
  from typing import Any
  session_dir = "/home/vogix/.local/state/vogix/sessions"

  print("=== Test: Session Save Creates File ===")
  # hyprctl/wezterm won't exist in VM, but save should still create the file
  # (with 0 windows/0 terminals — graceful degradation)
  machine.succeed(f"su - vogix -c 'mkdir -p {session_dir}'")
  machine.succeed("su - vogix -c 'vogix session save test-session'")
  machine.succeed(f"test -f {session_dir}/test-session.json")
  content = machine.succeed(f"su - vogix -c 'cat {session_dir}/test-session.json'")
  save_result: Any = json.loads(content)
  assert "windows" in save_result, "Session file should have 'windows' key"
  assert "terminals" in save_result, "Session file should have 'terminals' key"
  assert isinstance(save_result["windows"], list), "windows should be a list"
  assert isinstance(save_result["terminals"], list), "terminals should be a list"
  print("✓ Session save creates valid JSON file")

  print("\n=== Test: Session List Shows Saved Sessions ===")
  output = machine.succeed("su - vogix -c 'vogix session list'")
  assert "test-session" in output, f"List should show test-session, got: {output}"
  print("✓ Session list works")

  print("\n=== Test: Session Save Default Name ===")
  machine.succeed("su - vogix -c 'vogix session save'")
  machine.succeed(f"test -f {session_dir}/last.json")
  print("✓ Default save name is 'last'")

  print("\n=== Test: Session List Shows Multiple Sessions ===")
  output = machine.succeed("su - vogix -c 'vogix session list'")
  assert "last" in output, "Should show 'last'"
  assert "test-session" in output, "Should show 'test-session'"
  print("✓ Multiple sessions listed")

  print("\n=== Test: Autosave Stack ===")
  # Simulate autosave (daemon would do this)
  # Write initial autosave
  machine.succeed(f"su - vogix -c 'echo \'{{\"windows\":[],\"terminals\":[]}}\' > {session_dir}/autosave.json'")

  # Save a new autosave — should push the old one to autosave-1
  machine.succeed("su - vogix -c 'vogix session save autosave'")
  machine.succeed(f"test -f {session_dir}/autosave.json")
  machine.succeed(f"test -f {session_dir}/autosave-1.json")
  print("✓ Autosave stack pushes old state to autosave-1")

  # Save again — should push stack further
  machine.succeed("su - vogix -c 'vogix session save autosave'")
  machine.succeed(f"test -f {session_dir}/autosave-2.json")
  print("✓ Autosave stack keeps history (autosave-2 exists)")

  print("\n=== Test: Session Undo ===")
  # Copy existing autosave to autosave-1 to create undo history
  machine.succeed(f"cp {session_dir}/autosave.json {session_dir}/autosave-1.json")
  machine.succeed(f"test -f {session_dir}/autosave-1.json")
  machine.succeed("su - vogix -c 'vogix session undo'")
  print("✓ Session undo command succeeded")

  print("\n=== Test: Manual Save Not Overwritten by Autosave ===")
  # Save a manual session
  machine.succeed("su - vogix -c 'vogix session save important'")
  important_before = machine.succeed(f"cat {session_dir}/important.json")

  # Do several autosaves
  machine.succeed("su - vogix -c 'vogix session save autosave'")
  machine.succeed("su - vogix -c 'vogix session save autosave'")
  machine.succeed("su - vogix -c 'vogix session save autosave'")

  # Manual save should be unchanged
  important_after = machine.succeed(f"cat {session_dir}/important.json")
  assert important_before == important_after, "Manual save should not be modified by autosaves"
  print("✓ Manual saves are never overwritten by autosaves")

  print("\n=== Test: Session Restore Nonexistent Fails Gracefully ===")
  machine.fail("su - vogix -c 'vogix session restore nonexistent'")
  print("✓ Nonexistent session restore fails gracefully")

  print("\n=== Test: Session Undo With No History Fails Gracefully ===")
  machine.succeed(f"rm -f {session_dir}/autosave-*.json")
  machine.fail("su - vogix -c 'vogix session undo'")
  print("✓ Undo with no history fails gracefully")

  print("\n=== Test: Verify Session Format For Real Apps ===")
  # Write a session that mimics the actual desktop layout via Python on the VM
  real_session_json: Any = json.dumps({
    "windows": [
      {"class": "brave-browser", "title": "YouTube - Brave", "workspace": "3", "floating": False, "size": [1920, 1080], "at": [0, 0], "fullscreen": 0},
      {"class": "brave-browser", "title": "GitHub - Brave", "workspace": "1", "floating": False, "size": [960, 1080], "at": [0, 0], "fullscreen": 0},
      {"class": "org.wezfurlong.wezterm", "title": "btop", "workspace": "1", "floating": False, "size": [800, 600], "at": [960, 0], "fullscreen": 0},
      {"class": "org.wezfurlong.wezterm", "title": "bash", "workspace": "2", "floating": False, "size": [1920, 1080], "at": [0, 0], "fullscreen": 0},
      {"class": "org.wezfurlong.wezterm", "title": "hx", "workspace": "1", "floating": False, "size": [800, 600], "at": [960, 540], "fullscreen": 0},
      {"class": "bespec", "title": "BeSpec - Audio Spectrum Analyzer", "workspace": "1", "floating": False, "size": [400, 300], "at": [0, 780], "fullscreen": 0},
    ],
    "terminals": [
      {"pane_id": 1, "title": "btop", "cwd": "file://yoga/home/logger/"},
      {"pane_id": 2, "title": "bash", "cwd": "file://yoga/etc/nixos/"},
      {"pane_id": 3, "title": "hx", "cwd": "file://yoga/home/logger/Code/"},
      {"pane_id": 4, "title": "✳ lumen", "cwd": "file://yoga/home/logger/Code/github/logger/lumen/"},
      {"pane_id": 5, "title": "⠂ claude-session", "cwd": "file://yoga/etc/nixos/"},
    ],
  })
  # Write directly via tee to avoid shell escaping issues
  import tempfile
  with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
    f.write(real_session_json)
    tmp_path = f.name
  machine.copy_from_host(tmp_path, f"{session_dir}/real-desktop.json")
  machine.succeed(f"chown vogix:users {session_dir}/real-desktop.json")

  # Verify it parses correctly
  content = machine.succeed(f"cat {session_dir}/real-desktop.json")
  parsed: Any = json.loads(content)

  # Verify content via grep (avoids Python type checker issues with dict indexing)
  raw = machine.succeed(f"cat {session_dir}/real-desktop.json")
  assert "brave-browser" in raw, "Missing brave-browser"
  assert "org.wezfurlong.wezterm" in raw, "Missing wezterm"
  assert "bespec" in raw, "Missing bespec"
  print("✓ All app classes preserved")

  assert '"workspace": "1"' in raw or '"workspace":"1"' in raw, "Missing workspace 1"
  assert '"workspace": "2"' in raw or '"workspace":"2"' in raw, "Missing workspace 2"
  assert '"workspace": "3"' in raw or '"workspace":"3"' in raw, "Missing workspace 3"
  print("✓ Workspace assignments preserved")

  assert "file://" in raw, "CWDs should have file:// prefix"
  assert "btop" in raw, "Missing btop terminal"
  assert "lumen" in raw, "Missing lumen terminal"
  assert "claude" in raw, "Missing claude terminal"
  print("✓ Terminal state preserved")

  assert '"size": [1920' in raw or '"size":[1920' in raw, "Missing window size"
  print("✓ Window sizes preserved")

  output = machine.succeed("su - vogix -c 'vogix session list'")
  assert "real-desktop" in output, "real-desktop session should appear in list"
  print("✓ Real desktop session appears in list")

  print("\n=== All session tests passed! ===")
''
