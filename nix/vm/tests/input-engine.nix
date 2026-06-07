# Vogix input engine end-to-end test — the kanata-free path.
#
# This is the Phase-4 integration test: it boots a NixOS VM, creates a virtual
# uinput keyboard, runs the REAL `vogix input run` daemon against it, and
# asserts the full pipeline that the unit tests can't reach:
#
#   • a plain key is RE-EMITTED on the engine's virtual device  → typing works,
#     and works WITHOUT any compositor (this is what makes vogix the universal
#     keybinding layer, independent of Hyprland);
#   • CapsLock-hold + a bound key dispatches the right WM command as an IPC
#     MESSAGE to the compositor (here a mock control socket), and the bound key
#     is swallowed (not re-emitted);
#   • after using a mode the keyboard is NOT wedged — typing still works (the
#     "stuck / dead keyboard" regression guard).
#
# The compositor is mocked by a Unix socket that records the commands the engine
# writes, so the test is deterministic and doesn't need a real Wayland session
# (wlroots' headless backend can't open evdev anyway). The engine finds the mock
# the same way it finds a real Hyprland: by scanning $XDG_RUNTIME_DIR/hypr/*/
# for a `.socket.sock` (no HYPRLAND_INSTANCE_SIGNATURE needed — see hypr.rs).
{ pkgs
, self
, ...
}:

let
  pyenv = pkgs.python3.withPackages (ps: [ ps.evdev ]);

  # A fixed schema so the behavioural assertions are deterministic. Mirrors the
  # shape `defaults.nix` renders (the real-config parsing is covered by the
  # `parses_the_nix_shape` unit test); here we pin the bindings we assert on:
  #   desktop mode: h → "movefocus, l"   (a bound key → IPC dispatch + swallow)
  #   caps hold    → enter desktop       (via the F23 root binding, like defaults)
  testSchema = builtins.toJSON {
    modeGraph = {
      root = "app";
      modes = {
        app = { parent = null; type = "normal"; };
        desktop = { parent = "app"; type = "submap"; };
        move = { parent = "app"; type = "submap"; };
        resize = { parent = "app"; type = "submap"; };
        console = { parent = "app"; type = "passthrough"; };
      };
    };
    keybindings = {
      modKey = "super";
      layers = {
        desktopToggle = { hold = "capslock"; tapHoldMs = 250; holdAction = "f23"; };
      };
    };
    _superCtrlRemaps = {
      copy = { from = "super + c"; to = "ctrl + c"; };
    };
    modes = {
      app = {
        bindings = {
          ws1 = { key = "super + 1"; action = "workspace, 1"; };
          enterDesktopHold = { key = "F23"; action = "submap, desktop"; };
        };
      };
      desktop = {
        bindings = {
          focusLeft = { key = "h"; action = "movefocus, l"; repeat = true; };
          enterMove = { key = "m"; action = "submap, move"; };
          close = { key = "q"; action = "killactive,"; exitAfter = true; };
        };
      };
      move = { bindings = { moveLeft = { key = "h"; action = "movewindow, l"; repeat = true; }; }; };
      resize = { bindings = { }; };
      console = { bindings = { }; };
    };
  };

  driver = pkgs.writeText "vogix-input-driver.py" ''
    import subprocess, sys, time, threading, socket, os
    from evdev import UInput, InputDevice, list_devices, ecodes as e

    def fail(msg):
        print("FAIL:", msg)
        sys.exit(1)

    # The engine discovers the compositor socket by scanning this tree, exactly
    # as it will in production (no HYPRLAND_INSTANCE_SIGNATURE in a daemon env).
    XDG = "/tmp/xdg"
    SOCKDIR = XDG + "/hypr/testsig"
    SOCK = SOCKDIR + "/.socket.sock"
    os.makedirs(SOCKDIR, exist_ok=True)

    # 1) Mock compositor: a Unix socket that records the dispatch commands the
    #    engine sends. It replies "ok" so the engine's read drains cleanly.
    received = []
    def mock_compositor():
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            os.unlink(SOCK)
        except FileNotFoundError:
            pass
        srv.bind(SOCK)
        srv.listen(16)
        while True:
            conn, _ = srv.accept()
            data = conn.recv(4096)
            if data:
                received.append(data.decode(errors="replace"))
            try:
                conn.sendall(b"ok")
            except OSError:
                pass
            conn.close()
    threading.Thread(target=mock_compositor, daemon=True).start()
    time.sleep(0.5)

    # 2) Virtual keyboard the engine will grab (must advertise KEY_A so the
    #    engine recognises it as a keyboard).
    caps = { e.EV_KEY: [
        e.KEY_A, e.KEY_CAPSLOCK, e.KEY_H, e.KEY_M, e.KEY_Q,
        e.KEY_LEFTMETA, e.KEY_C, e.KEY_1,
    ] }
    ui = UInput(caps, name="vogix-test-kbd")
    time.sleep(1)

    # 3) Start the engine: scan-based discovery via XDG_RUNTIME_DIR, no HIS.
    env = dict(os.environ)
    env["XDG_RUNTIME_DIR"] = XDG
    env["RUST_LOG"] = "info"
    env.pop("HYPRLAND_INSTANCE_SIGNATURE", None)
    proc = subprocess.Popen(
        ["vogix", "input", "run", "--config", "/etc/vogix-test-schema.json"],
        env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )

    # 4) Find the engine's re-emit device (proof it grabbed + created uinput).
    out = None
    for _ in range(40):
        for path in list_devices():
            try:
                d = InputDevice(path)
            except OSError:
                continue
            if d.name == "vogix-input":
                out = d
                break
        if out:
            break
        if proc.poll() is not None:
            fail("vogix exited early:\n" + (proc.stdout.read() if proc.stdout else ""))
        time.sleep(0.25)
    if not out:
        fail("vogix-input device not found (engine didn't grab / create uinput)")
    print("engine grabbed + created vogix-input device")

    emitted = []
    def reader():
        try:
            for ev in out.read_loop():
                if ev.type == e.EV_KEY:
                    emitted.append((ev.code, ev.value))
        except OSError:
            pass
    threading.Thread(target=reader, daemon=True).start()
    time.sleep(0.5)

    def down(c): ui.write(e.EV_KEY, c, 1); ui.syn()
    def up(c):   ui.write(e.EV_KEY, c, 0); ui.syn()
    def tap(c, hold=0.05):
        down(c); time.sleep(hold); up(c)
    def emitted_codes():
        return [c for (c, v) in emitted]

    # --- Test 1: a plain key is re-emitted (typing works, compositor-agnostic) ---
    emitted.clear()
    tap(e.KEY_A)
    time.sleep(0.4)
    if e.KEY_A not in emitted_codes():
        fail(f"plain 'a' must be re-emitted on vogix-input, got {emitted}")
    print("PASS: plain key re-emitted (typing works)")

    # --- Test 2: caps-hold + h → IPC 'dispatch movefocus l', h swallowed ---
    received.clear()
    emitted.clear()
    down(e.KEY_CAPSLOCK)        # hold caps
    time.sleep(0.04)
    tap(e.KEY_H, hold=0.05)     # within tap-hold window → momentary desktop
    time.sleep(0.10)
    up(e.KEY_CAPSLOCK)          # release → back to app
    time.sleep(0.5)
    joined = " ".join(received)
    print("compositor received:", received)
    if "dispatch movefocus l" not in joined:
        fail(f"caps-hold+h must dispatch 'movefocus l', got {received}")
    if e.KEY_H in emitted_codes():
        fail(f"bound key h must be swallowed (not re-emitted), got {emitted}")
    print("PASS: caps-hold+h dispatched 'movefocus l' over the socket; h swallowed")

    # --- Test 3: no lockout — typing still works after using a mode ---
    emitted.clear()
    tap(e.KEY_A)
    time.sleep(0.3)
    if e.KEY_A not in emitted_codes():
        fail("keyboard wedged after mode use — typing stopped re-emitting")
    print("PASS: no lockout — typing still works after mode use")

    # --- Test 4: single-instance guard — a 2nd engine refuses, never double-grabs ---
    # Two engines grabbing the same keyboards at once collide and drop keystrokes
    # (the "can't type the first letter" failure after an overlapping restart).
    proc2 = subprocess.run(
        ["vogix", "input", "run", "--config", "/etc/vogix-test-schema.json"],
        env=env, capture_output=True, text=True, timeout=15,
    )
    msg = proc2.stdout + proc2.stderr
    if proc2.returncode == 0:
        fail("second vogix-input must refuse while one is running, but it succeeded")
    if "already holds" not in msg:
        fail(f"second instance must refuse with the single-instance lock message, got: {msg!r}")
    if proc.poll() is not None:
        fail("first engine must stay alive when a second is launched")
    emitted.clear()
    tap(e.KEY_A)
    time.sleep(0.3)
    if e.KEY_A not in emitted_codes():
        fail("first engine must keep working after a second instance is rejected")
    print("PASS: single-instance guard — 2nd engine refused, 1st intact + typing works")

    print("ALL VOGIX INPUT ENGINE TESTS PASSED")
  '';

in
pkgs.testers.nixosTest {
  name = "vogix-input-engine";

  nodes.machine = _: {
    environment.etc."vogix-test-schema.json".text = testSchema;
    # Reference the flake's vogix build directly (the `pkgs` in scope here is the
    # outer test pkgs, which carries no vogix overlay; an overlay would only land
    # on the machine's own pkgs arg). pyenv/evtest come from the plain pkgs.
    environment.systemPackages = [ self.packages.x86_64-linux.vogix pyenv pkgs.evtest ];

    # The engine needs /dev/uinput to emit and /dev/input/* to grab. The test
    # runs as root so it bypasses the input/uinput group wiring that the real
    # NixOS module sets up for the user service.
    hardware.uinput.enable = true;
    boot.kernelModules = [ "uinput" ];

    virtualisation = { memorySize = 1024; cores = 2; };
  };

  testScript = ''
    machine.start()
    machine.wait_for_unit("multi-user.target")
    machine.succeed("modprobe uinput || true")
    machine.wait_until_succeeds("test -e /dev/uinput")

    print(machine.succeed("${pyenv}/bin/python3 ${driver}"))
  '';
}
