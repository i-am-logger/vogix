# Hyprland RUNTIME keybinding test — the test that actually catches the
# "it stays in the mode" bug class.
#
# Unit/config tests verify generated strings; the kanata test verifies kanata's
# output. Neither sees Hyprland's RUNTIME submap behavior — and the bug WAS in
# Hyprland: `bindr` (release-bind) does not fire when a key's press entered a
# submap, so momentary mode never exited on release.
#
# This boots a real (headless) Hyprland with the generated config and inspects
# the LIVE registered binds via `hyprctl binds -j`, asserting:
#   • F23/F24 → submap desktop (enter) and F22 → submap reset (exit) are live
#   • the F22 exit is a PRESS-bind (release=false), never a release-bind
#   • NO submap transition anywhere uses a release-bind (`bindr`) — the exact
#     mechanism that caused "it stays in the mode". This is the runtime guard.
#   • the submaps are valid and switchable (hyprctl dispatch submap).
#
# NOTE: simulating a real keyPRESS→bind needs a GPU-accelerated VM (headless
# Hyprland doesn't read evdev; the virtual-keyboard protocol doesn't trigger
# binds). The keypress side is covered by the kanata VM test (kanata emits F22
# on caps release) + the config property test (exit is a press-bind, no bindr).
{ pkgs
,
}:

let
  inherit (pkgs) lib;
  behaviorModule = import ../../modules/behavior { inherit lib pkgs; };
  beh = behaviorModule.mkHyprlandConfig behaviorModule.defaults;
  binds = lib.concatMapStringsSep "\n" (b: "bind = ${b}") beh.settings.bind;

  hyprConf = pkgs.writeText "test-hyprland.conf" ''
    monitor = HEADLESS-1, 1920x1080@60, 0x0, 1
    misc {
      disable_hyprland_logo = true
      disable_hyprland_qtutils_check = true
    }
    ${binds}
    ${beh.extraConfig}
  '';

in
pkgs.testers.nixosTest {
  name = "vogix-keybindings-hyprland";

  nodes.machine = _: {
    programs.hyprland.enable = true;
    environment.systemPackages = [ pkgs.hyprland ];
    virtualisation = { memorySize = 2048; cores = 2; };
  };

  testScript = ''
    machine.start()
    machine.wait_for_unit("multi-user.target")

    machine.succeed("mkdir -p /run/user/0 && chmod 700 /run/user/0")
    # Headless Hyprland, no dbus session needed for keybind handling.
    machine.succeed(
        "systemd-run --unit=hypr "
        "--setenv=XDG_RUNTIME_DIR=/run/user/0 "
        "--setenv=WLR_BACKENDS=headless --setenv=WLR_RENDERER=pixman "
        "--setenv=AQ_NO_MODESET=1 "
        "Hyprland --i-am-really-stupid -c ${hyprConf}"
    )

    machine.sleep(5)
    machine.succeed("journalctl -u hypr --no-pager | tail -30 || true")
    machine.wait_until_succeeds("ls /run/user/0/hypr/*/.socket.sock 2>/dev/null", timeout=60)

    his = machine.succeed("ls /run/user/0/hypr/").strip()
    print("HIS:", his)
    env = f"XDG_RUNTIME_DIR=/run/user/0 HYPRLAND_INSTANCE_SIGNATURE={his} WAYLAND_DISPLAY=wayland-1"
    machine.wait_until_succeeds(f"{env} hyprctl version", timeout=30)

    def submap():
        s = machine.succeed(f"{env} hyprctl submap").strip()
        return "app" if s in ("", "unnamed", "default") else s

    # Hyprland parsed and registered the binds at runtime (catches config
    # errors / invalid dispatchers that string tests can't). `hyprctl binds -j`
    # is the live registered-bind list.
    binds = machine.succeed(f"{env} hyprctl binds -j")
    import json
    reg = json.loads(binds)

    def find(key, arg):
        return [b for b in reg if b.get("key") == key and b.get("arg") == arg]

    assert find("F23", "desktop"), "F23 hold-enter bind not registered"
    print("PASS: registered F23 -> submap desktop (hold enter)")
    assert find("F24", "desktop"), "F24 click-enter bind not registered"
    print("PASS: registered F24 -> submap desktop (click enter)")

    # THE FIX + REGRESSION GUARD for the exact bug: the momentary exit must be a
    # PRESS-bind on F22 (release=false), never a release-bind (bindr). Hyprland's
    # release-binds don't fire when the press entered the submap, so any `bindr`
    # exit would leave the user stuck ("it stays in the mode").
    f22 = find("F22", "reset")
    assert f22, "F22 hold-RELEASE exit bind not registered (the fix)"
    assert all(not b.get("release") for b in f22), "F22 exit must be a PRESS-bind, not bindr"
    print("PASS: F22 -> submap reset is a press-bind (reliable exit)")

    # No submap-transition anywhere may use a release-bind — this is the runtime
    # assertion that would have caught the original bug.
    rel = [b for b in reg if b.get("release") and b.get("dispatcher") == "submap"]
    assert not rel, f"submap transition(s) use unreliable release-bind (bindr): {rel}"
    print("PASS: zero submap release-binds (no bindr) — the bug cannot recur")

    # The submaps are valid and switchable at runtime.
    assert submap() == "app", f"should start in app, got {submap()!r}"
    machine.succeed(f"{env} hyprctl dispatch submap desktop"); machine.sleep(1)
    assert submap() == "desktop", f"submap desktop not active, got {submap()!r}"
    print("PASS: submap desktop valid + active")
    machine.succeed(f"{env} hyprctl dispatch submap reset"); machine.sleep(1)
    assert submap() == "app", f"submap reset failed, got {submap()!r}"
    print("PASS: submap reset returns to app")

    print("ALL HYPRLAND RUNTIME TESTS PASSED")
  '';
}
