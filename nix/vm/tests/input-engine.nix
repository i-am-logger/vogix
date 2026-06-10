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

  # The REAL shipped schema rendered from defaults.nix (not the hand-written
  # testSchema above). `vogix input check` validates it loads + every binding
  # parses, so a mistyped/unknown key that the router would silently drop fails
  # the suite instead of vanishing.
  behaviorReal = import ../../modules/behavior { inherit (pkgs) lib; inherit pkgs; };
  realDefaultsJson = behaviorReal.mkSchemaJSON { };
  # Each shipped interaction paradigm rendered from the REAL module, so every
  # paradigm is proven end-to-end in the VM (not just at eval): the schema loads,
  # all bindings parse + axioms hold (`vogix input check`), and the paradigm's
  # signature gesture actually dispatches (the behavioural section in the driver).
  paradigmWindowsJson = behaviorReal.mkSchemaJSON { keybindings = { paradigm = "windows"; }; };
  paradigmMacJson = behaviorReal.mkSchemaJSON { keybindings = { paradigm = "mac"; }; };
  paradigmEmacsJson = behaviorReal.mkSchemaJSON { keybindings = { paradigm = "emacs"; }; };

  # A fixed ENGINE-NATIVE schema so the behavioural assertions are deterministic.
  # Mirrors the shape `defaults.nix` renders (real-config parsing is covered by
  # the `parses_the_nix_shape` unit test); here we pin the daily-driver bindings
  # the UX assertions exercise. The caps layer names its target mode DIRECTLY
  # (`entersMode = "desktop"`) — no synthetic f23 keysym, no `enterDesktopHold`
  # root binding. Every mode carries `exit = "escape"` so the Esc safety-net is
  # exercised; in catchall modes it returns to root, in `app` it passes through.
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
      # No paradigm set → defaults to "macos" (praxis macos_remap): Super+C/V → Ctrl+C/V.
      layers = {
        desktopToggle = { hold = "capslock"; tapHoldMs = 250; entersMode = "desktop"; };
      };
    };
    # kitty is a terminal: the context-aware remap retargets copy/paste there.
    terminalClasses = [ "kitty" ];
    # Per-mode border colours for the mode-visibility surface.
    modeColors = {
      desktop = { active = "rgb(89b4fa)"; inactive = "rgb(313244)"; };
    };
    modes = {
      app = {
        exit = "escape";
        bindings = {
          ws1 = { key = "super + 1"; action = "workspace, 1"; };
          volumeUp = { key = "XF86AudioRaiseVolume"; action = "exec, pamixer -i 5"; };
        };
      };
      desktop = {
        exit = "escape";
        bindings = {
          focusLeft = { key = "h"; action = "movefocus, l"; repeat = true; };
          focusLeftArrow = { key = "left"; action = "movefocus, l"; repeat = true; };
          enterMove = { key = "m"; action = "submap, move"; };
          enterResize = { key = "r"; action = "submap, resize"; };
          enterConsole = { key = "c"; action = "submap, console"; };
          ws1 = { key = "1"; action = "workspace, 1"; };
          sendToWs1 = { key = "shift + 1"; action = "movetoworkspace, 1"; };
          toggleFloat = { key = "y"; action = "togglefloating,"; }; # stay (no exitAfter)
          close = { key = "q"; action = "killactive,"; exitAfter = true; };
          # In-place window management from desktop (the exitAfter rework): the
          # modifier on a direction picks the verb — bare = focus, Shift = move,
          # Ctrl = resize — and Tab toggles split. The quick path; the m/r
          # sub-modes below are for sustained arranging.
          moveShift = { key = "shift + h"; action = "movewindow, l"; repeat = true; };
          resizeCtrl = { key = "ctrl + h"; action = "resizeactive, -40 0"; repeat = true; };
          toggleSplit = { key = "tab"; action = "layoutmsg, togglesplit"; };
        };
      };
      move = {
        exit = "escape";
        bindings = {
          moveLeft = { key = "h"; action = "movewindow, l"; repeat = true; };
          toResize = { key = "r"; action = "submap, resize"; };
          toggleSplit = { key = "tab"; action = "layoutmsg, togglesplit"; };
        };
      };
      resize = {
        exit = "escape";
        bindings = {
          resizeLeft = { key = "h"; action = "resizeactive, -40 0"; repeat = true; };
          toMove = { key = "m"; action = "submap, move"; };
          toggleSplit = { key = "tab"; action = "layoutmsg, togglesplit"; };
        };
      };
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
    # A controllable mock compositor: binds a recording socket and returns its
    # server handle (so a test can CLOSE it to simulate a compositor crash) plus
    # its received-commands list. `received` below is mock #1 — existing tests use it.
    def make_mock(sock_path):
        recv = []
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            os.unlink(sock_path)
        except FileNotFoundError:
            pass
        srv.bind(sock_path)
        srv.listen(16)
        def loop():
            while True:
                try:
                    conn, _ = srv.accept()
                except OSError:
                    return  # server closed → stop (compositor 'crashed')
                data = conn.recv(4096)
                if data:
                    recv.append(data.decode(errors="replace"))
                try:
                    conn.sendall(b"ok")
                except OSError:
                    pass
                conn.close()
        threading.Thread(target=loop, daemon=True).start()
        return srv, recv
    srv1, received = make_mock(SOCK)
    time.sleep(0.5)

    # Mock Hyprland EVENT socket (.socket2.sock): streams `activewindow` events so
    # the engine can track the focused window class for the context-aware
    # Super→Ctrl remap. set_active_window() pushes a focus change to the engine.
    SOCK2EV = SOCKDIR + "/.socket2.sock"
    win_conns = []
    def mock_events():
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            os.unlink(SOCK2EV)
        except FileNotFoundError:
            pass
        srv.bind(SOCK2EV)
        srv.listen(16)
        while True:
            try:
                conn, _ = srv.accept()
            except OSError:
                return
            win_conns.append(conn)
    threading.Thread(target=mock_events, daemon=True).start()
    def set_active_window(cls):
        line = ("activewindow>>" + cls + ",title\n").encode()
        for c in list(win_conns):
            try:
                c.sendall(line)
            except OSError:
                pass
    time.sleep(0.3)

    # 2) Virtual keyboard the engine will grab (must advertise KEY_A so the
    #    engine recognises it as a keyboard). The full set below covers every
    #    key the UX assertions inject.
    caps = { e.EV_KEY: [
        e.KEY_A, e.KEY_CAPSLOCK, e.KEY_H, e.KEY_M, e.KEY_Q, e.KEY_R, e.KEY_Y,
        e.KEY_LEFTMETA, e.KEY_LEFTSHIFT, e.KEY_LEFTCTRL, e.KEY_C, e.KEY_V, e.KEY_X, e.KEY_1,
        e.KEY_LEFT, e.KEY_TAB, e.KEY_ESC, e.KEY_VOLUMEUP,
        # Enter + home-row letters so it passes the strict text-keyboard floor
        # (DeviceFilter), exercising the STRICT grab path rather than the fail-safe.
        e.KEY_ENTER, e.KEY_S, e.KEY_D, e.KEY_F,
    ] }
    ui = UInput(caps, name="vogix-test-kbd")

    # A YubiKey-shaped device: it passes the broad "has KEY_A" filter AND the
    # keyboard capability floor, but must be EXCLUDED by its Yubico vendor id
    # (0x1050) so the engine never grabs a security key (device-grab scope fix).
    yk = { e.EV_KEY: [
        e.KEY_A, e.KEY_ENTER, e.KEY_LEFTSHIFT, e.KEY_S, e.KEY_D, e.KEY_F, e.KEY_9,
    ] }
    ui_yk = UInput(yk, name="YubiKey OTP", vendor=0x1050)
    time.sleep(1)

    # 3) Start the engine: scan-based discovery via XDG_RUNTIME_DIR, no HIS.
    env = dict(os.environ)
    env["XDG_RUNTIME_DIR"] = XDG
    env["RUST_LOG"] = "info"
    # Deterministic state dir so the health-snapshot assertion (Test 1c) knows
    # where ~/.local/state/vogix/input-health.json lands.
    env["HOME"] = "/root"
    env.pop("XDG_STATE_HOME", None)
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

    # Seed a NON-terminal focused window so the Super→Ctrl remap behaves normally
    # (plain Ctrl+C) for the tests below. Wait for the engine's event-stream
    # connection first so the focus event isn't sent into the void.
    for _ in range(40):
        if win_conns:
            break
        time.sleep(0.1)
    set_active_window("firefox")
    time.sleep(0.4)

    # --- Test 1: a plain key is re-emitted (typing works, compositor-agnostic) ---
    emitted.clear()
    tap(e.KEY_A)
    time.sleep(0.4)
    if e.KEY_A not in emitted_codes():
        fail(f"plain 'a' must be re-emitted on vogix-input, got {emitted}")
    print("PASS: plain key re-emitted (typing works)")

    # --- Test 1b: a YubiKey (Yubico vendor 0x1050) is NOT grabbed ---
    # If the engine wrongly grabbed it, a key injected on it would be re-emitted
    # on vogix-input. It must not be. This ALSO proves the real keyboard passed
    # the STRICT filter: had it failed, the fail-safe would have grabbed every
    # candidate (incl. the YubiKey) and KEY_9 would re-emit here.
    emitted.clear()
    ui_yk.write(e.EV_KEY, e.KEY_9, 1); ui_yk.syn()
    ui_yk.write(e.EV_KEY, e.KEY_9, 0); ui_yk.syn()
    time.sleep(0.4)
    if e.KEY_9 in emitted_codes():
        fail("YubiKey (vendor 0x1050) must NOT be grabbed — its key was re-emitted")
    print("PASS: YubiKey not grabbed (security key left alone)")

    # --- Test 1c: the health snapshot reflects real flow, with NO key identity ---
    # The engine writes ~/.local/state/vogix/input-health.json about once a second
    # (`vogix input doctor` reads it). After the taps above it must list the
    # grabbed keyboard with events_in > 0 and no stuck keys — and contain no
    # keycode field (the no-keylog invariant).
    import json
    tap(e.KEY_A); tap(e.KEY_A)
    time.sleep(1.4)  # ensure at least one periodic snapshot write lands
    snap_path = "/root/.local/state/vogix/input-health.json"
    raw = open(snap_path).read()
    snap = json.loads(raw)
    names = [d["name"] for d in snap["devices"]]
    if "vogix-test-kbd" not in names:
        fail(f"health snapshot missing the grabbed keyboard, got {names}")
    kbd = next(d for d in snap["devices"] if d["name"] == "vogix-test-kbd")
    if kbd["events_in"] < 1:
        fail(f"health events_in should be > 0 after typing, got {kbd}")
    if snap["stuck_count"] != 0:
        fail(f"health stuck_count should be 0, got {snap['stuck_count']}")
    if '"code"' in raw or '"keycode"' in raw:
        fail("health snapshot must carry NO key identity (no-keylog invariant)")
    print("PASS: health snapshot reflects flow (no keylog)")

    # --- Test 1d: hotplug — a keyboard created AFTER startup is grabbed live ---
    # The engine watches /dev/input via inotify; a new node that passes the
    # keyboard filter is grabbed without a restart. Inject a key unique to the
    # new device and assert it is re-emitted on vogix-input (= it was grabbed).
    hp = { e.EV_KEY: [
        e.KEY_A, e.KEY_ENTER, e.KEY_LEFTSHIFT, e.KEY_S, e.KEY_D, e.KEY_F, e.KEY_Z,
    ] }
    ui_hp = UInput(hp, name="vogix-hotplug-kbd")
    time.sleep(2.0)  # let inotify fire (CREATE/ATTRIB) and the engine grab it
    emitted.clear()
    ui_hp.write(e.EV_KEY, e.KEY_Z, 1); ui_hp.syn()
    ui_hp.write(e.EV_KEY, e.KEY_Z, 0); ui_hp.syn()
    time.sleep(0.4)
    if e.KEY_Z not in emitted_codes():
        fail("hotplugged keyboard was not grabbed — its key was not re-emitted")
    print("PASS: hotplug — keyboard added after startup is grabbed")

    # --- Test 1e: hotplug REMOVE is clean (unplug → slot freed, no wedge) ---
    # Destroying the hotplugged keyboard raises POLLHUP on the engine's grab; the
    # slot is freed (the Device ungrabs on Drop) and the engine keeps running —
    # the ORIGINAL keyboard still types, no 100% CPU spin / wedge.
    ui_hp.close()
    time.sleep(1.0)  # let POLLHUP propagate and the slot drop
    if proc.poll() is not None:
        fail("engine died after a hotplugged keyboard was removed")
    emitted.clear()
    tap(e.KEY_A); time.sleep(0.3)
    if e.KEY_A not in emitted_codes():
        fail("after a hotplug remove, the original keyboard must still type")
    print("PASS: hotplug remove is clean (slot freed, original keyboard still types)")

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

    # ── Full daily-driver UX (engine-native model; the kanata-free path) ──
    # Every gesture class the engine now owns, proven end-to-end through the real
    # evdev grab → uinput re-emit / mock-compositor dispatch path.
    def dispatched():
        return " ".join(received)

    # --- Test 5: Super→Ctrl remap at evdev (data-driven: c AND v), no Super leak ---
    for letter, name in [(e.KEY_C, "c"), (e.KEY_V, "v")]:
        emitted.clear()
        down(e.KEY_LEFTMETA); tap(letter, hold=0.03); up(e.KEY_LEFTMETA)
        time.sleep(0.3)
        codes = emitted_codes()
        if e.KEY_LEFTCTRL not in codes or letter not in codes:
            fail(f"super+{name} must emit Ctrl+{name}, got {emitted}")
        if e.KEY_LEFTMETA in codes:
            fail(f"super+{name} must NOT leak LEFTMETA to apps, got {emitted}")
    print("PASS: Super→Ctrl remap (c, v) at evdev; Super never leaks")

    # --- Test 6: Super+number → workspace dispatch (NOT remapped, NOT typed) ---
    received.clear(); emitted.clear()
    down(e.KEY_LEFTMETA); tap(e.KEY_1, hold=0.03); up(e.KEY_LEFTMETA)
    time.sleep(0.4)
    if "dispatch workspace 1" not in dispatched():
        fail(f"super+1 must dispatch 'workspace 1', got {received}")
    if e.KEY_LEFTCTRL in emitted_codes() or e.KEY_1 in emitted_codes():
        fail(f"super+number must not be remapped or typed, got {emitted}")
    print("PASS: Super+1 → workspace dispatch (excluded from the remap set)")

    # --- Test 7: caps TAP → sticky desktop; chain ws+focus; catchall swallow; tap → exit ---
    received.clear(); emitted.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05)                       # < 250ms → sticky desktop
    time.sleep(0.1)
    tap(e.KEY_1, hold=0.03); time.sleep(0.1)             # bare 1 in desktop → workspace
    tap(e.KEY_H, hold=0.03); time.sleep(0.1)             # focus
    tap(e.KEY_A, hold=0.03); time.sleep(0.1)             # unbound → catchall swallow
    d = dispatched()
    if "dispatch workspace 1" not in d or "dispatch movefocus l" not in d:
        fail(f"sticky desktop must chain ws+focus, got {received}")
    if e.KEY_A in emitted_codes():
        fail(f"unbound 'a' in desktop must be swallowed (catchall), got {emitted}")
    emitted.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # tap again → exit
    tap(e.KEY_A, hold=0.03); time.sleep(0.2)
    if e.KEY_A not in emitted_codes():
        fail("after caps-tap exit, typing must resume in app")
    print("PASS: caps-tap sticky → chain (ws/focus) → catchall swallow → tap exit")

    # --- Test 8: caps-HOLD → m (move) → release caps → back to APP (no stuck) ---
    received.clear(); emitted.clear()
    down(e.KEY_CAPSLOCK); time.sleep(0.03)               # hold
    tap(e.KEY_M, hold=0.03); time.sleep(0.05)            # resolves hold → momentary desktop, m → move
    tap(e.KEY_H, hold=0.03); time.sleep(0.1)             # h in move → movewindow
    up(e.KEY_CAPSLOCK); time.sleep(0.3)                  # release → revert to root (app)
    if "dispatch movewindow l" not in dispatched():
        fail(f"caps-hold→m→h must dispatch 'movewindow l', got {received}")
    emitted.clear()
    tap(e.KEY_A, hold=0.03); time.sleep(0.2)
    if e.KEY_A not in emitted_codes():
        fail("caps release from a SWITCHED sub-mode must return to app (typing works)")
    print("PASS: caps-hold → move sub-mode → release returns to app (no stuck)")

    # --- Test 9: resize route + move↔resize switch ---
    received.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # sticky desktop
    tap(e.KEY_R, hold=0.03); time.sleep(0.05)            # → resize
    tap(e.KEY_H, hold=0.03); time.sleep(0.1)             # resizeactive
    tap(e.KEY_M, hold=0.03); time.sleep(0.05)            # resize → move (toMove)
    tap(e.KEY_H, hold=0.03); time.sleep(0.1)             # movewindow
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # exit
    d = dispatched()
    if "dispatch resizeactive -40 0" not in d or "dispatch movewindow l" not in d:
        fail(f"resize then switch-to-move must dispatch both, got {received}")
    print("PASS: resize sub-mode + move↔resize switch")

    # --- Test 9b: in-place move/resize/split from desktop (exitAfter rework) ---
    # Shift+dir moves the window, Ctrl+dir resizes it, Tab toggles split — all
    # straight from the desktop mode, no m/r sub-mode. (Bare h stays focus, Test 2.)
    received.clear()
    down(e.KEY_CAPSLOCK); time.sleep(0.03)                        # hold → momentary desktop
    down(e.KEY_LEFTSHIFT); tap(e.KEY_H, hold=0.03); up(e.KEY_LEFTSHIFT); time.sleep(0.1)  # Shift+h → move
    down(e.KEY_LEFTCTRL); tap(e.KEY_H, hold=0.03); up(e.KEY_LEFTCTRL); time.sleep(0.1)    # Ctrl+h → resize
    tap(e.KEY_TAB, hold=0.03); time.sleep(0.1)                    # Tab → toggle split
    up(e.KEY_CAPSLOCK); time.sleep(0.3)                          # release → app
    d = dispatched()
    if "dispatch movewindow l" not in d:
        fail(f"desktop Shift+h must move the window (movewindow l), got {received}")
    if "dispatch resizeactive -40 0" not in d:
        fail(f"desktop Ctrl+h must resize the window (resizeactive), got {received}")
    if "dispatch layoutmsg togglesplit" not in d:
        fail(f"desktop Tab must toggle split (layoutmsg togglesplit), got {received}")
    emitted.clear()
    tap(e.KEY_A, hold=0.03); time.sleep(0.2)
    if e.KEY_A not in emitted_codes():
        fail("after in-place management, caps release must return to app (typing works)")
    print("PASS: in-place move/resize/split from desktop (Shift/Ctrl+dir, Tab)")

    # --- Test 10: exitAfter returns to app WITHOUT a second caps tap ---
    received.clear(); emitted.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # sticky desktop
    tap(e.KEY_Q, hold=0.03); time.sleep(0.2)             # killactive, exitAfter
    if "dispatch killactive" not in dispatched():
        fail(f"q must dispatch 'killactive', got {received}")
    emitted.clear()
    tap(e.KEY_A, hold=0.03); time.sleep(0.2)
    if e.KEY_A not in emitted_codes():
        fail("exitAfter must return to app (typing works without a 2nd caps tap)")
    print("PASS: exitAfter (q) returns to app")

    # --- Test 11: stay/chainable binding (y) does NOT exit ---
    received.clear(); emitted.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # sticky desktop
    tap(e.KEY_Y, hold=0.03); time.sleep(0.1)             # togglefloating, stay
    tap(e.KEY_H, hold=0.03); time.sleep(0.1)             # still in desktop → movefocus
    d = dispatched()
    if "dispatch togglefloating" not in d or "dispatch movefocus l" not in d:
        fail(f"stay binding must keep desktop active for the next key, got {received}")
    if e.KEY_A in emitted_codes():
        fail("still in desktop after a stay binding → 'a' must be swallowed")
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # exit
    print("PASS: stay binding (y) is chainable, doesn't exit")

    # --- Test 12: Esc safety-net exits a catchall mode → app ---
    emitted.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # sticky desktop
    tap(e.KEY_M, hold=0.03); time.sleep(0.05)            # → move
    tap(e.KEY_ESC, hold=0.03); time.sleep(0.2)           # Esc → ExitToRoot
    if e.KEY_ESC in emitted_codes():
        fail("Esc inside a catchall mode must be swallowed (consumed as the exit)")
    emitted.clear()
    tap(e.KEY_A, hold=0.03); time.sleep(0.2)
    if e.KEY_A not in emitted_codes():
        fail("Esc safety-net must return to app (typing works)")
    print("PASS: Esc safety-net exits a catchall mode to app")

    # --- Test 13: arrow-key focus + auto-repeat refire ---
    received.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # sticky desktop
    down(e.KEY_LEFT); time.sleep(0.03)
    ui.write(e.EV_KEY, e.KEY_LEFT, 2); ui.syn()          # auto-repeat
    time.sleep(0.03)
    up(e.KEY_LEFT); time.sleep(0.2)
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # exit
    n = dispatched().count("dispatch movefocus l")
    if n < 2:
        fail(f"arrow press + repeat must dispatch movefocus l >=2x, got {n}: {received}")
    print("PASS: arrow-key focus + auto-repeat refire")

    # --- Test 14: app-mode exec/media dispatch (XF86 key) ---
    received.clear()
    tap(e.KEY_VOLUMEUP, hold=0.03); time.sleep(0.2)
    if "dispatch exec pamixer -i 5" not in dispatched():
        fail(f"XF86AudioRaiseVolume must dispatch its exec, got {received}")
    print("PASS: media key → exec dispatch")

    # --- Test 15: Shift+number send-to-workspace (modifier chord) ---
    received.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # sticky desktop
    down(e.KEY_LEFTSHIFT); tap(e.KEY_1, hold=0.03); up(e.KEY_LEFTSHIFT)
    time.sleep(0.2)
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)      # exit
    if "dispatch movetoworkspace 1" not in dispatched():
        fail(f"shift+1 must dispatch 'movetoworkspace 1', got {received}")
    print("PASS: Shift+number send-to-workspace chord")

    # --- Test 16: fast-typing burst in app — every key re-emitted, none dropped ---
    emitted.clear()
    burst = [e.KEY_A, e.KEY_H, e.KEY_M, e.KEY_Q, e.KEY_R, e.KEY_Y, e.KEY_C, e.KEY_V]
    for c in burst:
        down(c); up(c)
    down(e.KEY_A); ui.write(e.EV_KEY, e.KEY_A, 2); ui.syn(); up(e.KEY_A)  # auto-repeat
    time.sleep(0.5)
    codes = emitted_codes()
    missing = [c for c in burst if c not in codes]
    if missing:
        fail(f"fast-typing burst dropped keys {missing}; got {emitted}")
    print("PASS: fast-typing burst in app — all keys re-emitted, none dropped")

    # --- Test 17: console passthrough — unbound keys + Esc re-emit; caps-tap exits ---
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)   # sticky desktop
    tap(e.KEY_C, hold=0.03); time.sleep(0.1)          # c -> submap console (passthrough)
    emitted.clear()
    tap(e.KEY_A, hold=0.03); time.sleep(0.1)          # unbound in passthrough -> re-emitted
    if e.KEY_A not in emitted_codes():
        fail("console is passthrough: unbound 'a' must re-emit, not be swallowed")
    emitted.clear()
    tap(e.KEY_ESC, hold=0.03); time.sleep(0.1)        # Esc must pass through (console not catchall)
    if e.KEY_ESC not in emitted_codes():
        fail("console is passthrough: Esc must pass through (not consumed as an exit)")
    emitted.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)   # caps-tap exits console -> app
    tap(e.KEY_A, hold=0.03); time.sleep(0.2)
    if e.KEY_A not in emitted_codes():
        fail("caps-tap must exit console back to app (typing resumes)")
    print("PASS: console passthrough — unbound + Esc re-emit; caps-tap exits")

    # --- Test 19: context-aware Super→Ctrl — terminal vs GUI (from window class) ---
    # In a GUI app the macOS-Command remap applies: Super+C → Ctrl+C.
    set_active_window("firefox"); time.sleep(0.3)
    emitted.clear()
    down(e.KEY_LEFTMETA); tap(e.KEY_C, hold=0.03); up(e.KEY_LEFTMETA); time.sleep(0.3)
    codes = emitted_codes()
    if e.KEY_LEFTCTRL not in codes or e.KEY_C not in codes:
        fail(f"Super+C in a GUI app must emit Ctrl+C, got {emitted}")
    if e.KEY_LEFTSHIFT in codes:
        fail(f"Super+C in a GUI app must NOT add Shift, got {emitted}")
    # Focus a terminal (kitty ∈ terminalClasses): Super+C must become Ctrl+Shift+C,
    # never bare Ctrl+C — which (POSIX termios) would SIGINT the foreground job.
    set_active_window("kitty"); time.sleep(0.3)
    emitted.clear()
    down(e.KEY_LEFTMETA); tap(e.KEY_C, hold=0.03); up(e.KEY_LEFTMETA); time.sleep(0.3)
    codes = emitted_codes()
    if not (e.KEY_LEFTCTRL in codes and e.KEY_LEFTSHIFT in codes and e.KEY_C in codes):
        fail(f"Super+C in a terminal must emit Ctrl+Shift+C, got {emitted}")
    print("PASS: context-aware Super→Ctrl — GUI=Ctrl+C, terminal=Ctrl+Shift+C")
    set_active_window("firefox"); time.sleep(0.2)  # restore non-terminal for later tests

    # --- Test 20: mode-visibility surface — entering a mode paints the border ---
    set_active_window("firefox"); time.sleep(0.2)
    received.clear()
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.2)   # tap → sticky desktop (coloured)
    if "keyword general:col.active_border rgb(89b4fa)" not in " ".join(received):
        fail(f"entering desktop must paint the active-window border, got {received}")
    tap(e.KEY_CAPSLOCK, hold=0.05); time.sleep(0.1)   # exit back to app
    print("PASS: mode-visibility surface — entering desktop paints the border")

    # --- Test 18: no-compositor tolerance + self-heal re-discovery after restart ---
    # The headline "typing works with no compositor; re-attaches when one appears"
    # — the stale-socket "keybindings stop after Hyprland restarts" regression.
    srv1.close()                                   # compositor 'crash'
    try:
        os.unlink(SOCK)
    except OSError:
        pass
    time.sleep(0.3)
    # A dispatch into the now-dead socket must be dropped (no crash) + clear the
    # cached handle; typing (re-emit) must keep working with no compositor.
    emitted.clear()
    down(e.KEY_CAPSLOCK); time.sleep(0.03); tap(e.KEY_H, hold=0.03); up(e.KEY_CAPSLOCK)
    time.sleep(0.3)
    if proc.poll() is not None:
        fail("engine crashed when the compositor socket went away")
    tap(e.KEY_A, hold=0.03); time.sleep(0.2)
    if e.KEY_A not in emitted_codes():
        fail("typing must still work with no compositor (re-emit is compositor-agnostic)")
    # Compositor 'restarts' at a NEW instance dir; the engine must re-discover it
    # on the next dispatch (lazy, self-healing — no service restart).
    SOCKDIR2 = XDG + "/hypr/testsig2"
    SOCK2 = SOCKDIR2 + "/.socket.sock"
    os.makedirs(SOCKDIR2, exist_ok=True)
    _srv2, received2 = make_mock(SOCK2)
    time.sleep(0.3)
    received2.clear()
    down(e.KEY_CAPSLOCK); time.sleep(0.03); tap(e.KEY_H, hold=0.03); up(e.KEY_CAPSLOCK)
    time.sleep(0.6)
    if "dispatch movefocus l" not in " ".join(received2):
        fail(f"engine must re-discover the restarted compositor, got {received2}")
    print("PASS: no-compositor tolerated + self-heal re-discovery after restart")

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

    # ── Paradigm behavioural coverage: every shipped paradigm proven end-to-end ──
    # The tests above ran the engine on the default (modal) schema. Now restart it
    # on each REAL rendered paradigm schema and prove its signature gesture fires
    # the right WM dispatch — chorded nav that bypasses the CapsLock layer.
    def vogix_dev():
        for path in list_devices():
            try:
                if InputDevice(path).name == "vogix-input":
                    return path
            except OSError:
                continue
        return None

    def paradigm_dispatch(label, config, inject, expect):
        global proc
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()
        # The uinput device is created BEFORE the grab, so "device present" alone
        # doesn't mean ready. Wait for the OLD engine's device to disappear first,
        # so we don't mistake the stale device for the new engine and inject early.
        for _ in range(50):
            if vogix_dev() is None:
                break
            time.sleep(0.1)
        proc = subprocess.Popen(
            ["vogix", "input", "run", "--config", config],
            env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
        for _ in range(60):
            if vogix_dev() is not None:
                break
            if proc.poll() is not None:
                fail(label + ": engine exited early:\n" + (proc.stdout.read() if proc.stdout else ""))
            time.sleep(0.25)
        if vogix_dev() is None:
            fail(label + ": vogix-input device not found (engine didn't grab)")
        time.sleep(1.0)  # let the keyboard grab settle before injecting
        # After Test 18 the live compositor mock is testsig2 (received2); the
        # restarted engine re-discovers the newest-live socket, so assert there.
        received2.clear()
        inject()
        time.sleep(0.4)
        if expect not in " ".join(received2):
            fail(label + ": expected dispatch " + repr(expect) + ", got " + repr(received2))
        print("PASS: paradigm " + label)

    # windows: chorded Super+Left → focus left (no CapsLock; remap = none).
    paradigm_dispatch(
        "windows — Super+Left dispatches movefocus",
        "/etc/vogix-paradigm-windows.json",
        lambda: (down(e.KEY_LEFTMETA), tap(e.KEY_LEFT, hold=0.03), up(e.KEY_LEFTMETA)),
        "dispatch movefocus l",
    )
    # mac: native Control+Left → previous Space (workspace -1); the faithful macOS
    # Spaces gesture (Super+Tab=Cmd+Tab cycle is also bound, covered by check).
    paradigm_dispatch(
        "mac — Control+Left dispatches workspace (Spaces)",
        "/etc/vogix-paradigm-mac.json",
        lambda: (down(e.KEY_LEFTCTRL), tap(e.KEY_LEFT, hold=0.03), up(e.KEY_LEFTCTRL)),
        "dispatch workspace -1",
    )
    # emacs: a key SEQUENCE — CapsLock-tap → desktop, then C-x (prefix mode), then
    # C-c completes it → close window. Proves sequences = chord-triggered modes.
    def emacs_cxcc():
        tap(e.KEY_CAPSLOCK, hold=0.05)  # tap → sticky desktop
        time.sleep(0.15)
        down(e.KEY_LEFTCTRL); tap(e.KEY_X, hold=0.03); up(e.KEY_LEFTCTRL)  # C-x → emacs-cx
        time.sleep(0.1)
        down(e.KEY_LEFTCTRL); tap(e.KEY_C, hold=0.03); up(e.KEY_LEFTCTRL)  # C-c → close
    paradigm_dispatch(
        "emacs — C-x C-c sequence dispatches killactive",
        "/etc/vogix-paradigm-emacs.json",
        emacs_cxcc,
        "dispatch killactive",
    )

    print("ALL VOGIX INPUT ENGINE TESTS PASSED")
  '';

in
pkgs.testers.nixosTest {
  name = "vogix-input-engine";

  nodes.machine = _: {
    environment.etc."vogix-test-schema.json".text = testSchema;
    environment.etc."vogix-real-defaults.json".text = realDefaultsJson;
    environment.etc."vogix-paradigm-windows.json".text = paradigmWindowsJson;
    environment.etc."vogix-paradigm-mac.json".text = paradigmMacJson;
    environment.etc."vogix-paradigm-emacs.json".text = paradigmEmacsJson;
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

    # Every shipped paradigm's schema must load + parse + pass the graph/remap
    # axioms (an unknown key would otherwise be silently dropped by the router).
    print(machine.succeed("vogix input check --config /etc/vogix-real-defaults.json"))
    print(machine.succeed("vogix input check --config /etc/vogix-paradigm-windows.json"))
    print(machine.succeed("vogix input check --config /etc/vogix-paradigm-mac.json"))
    print(machine.succeed("vogix input check --config /etc/vogix-paradigm-emacs.json"))

    print(machine.succeed("${pyenv}/bin/python3 ${driver}"))
  '';
}
