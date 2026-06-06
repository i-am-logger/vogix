# Keybinding behavior test — runs the REAL generated kanata config on a
# virtual keyboard and asserts the tap-hold output events.
#
# This is the behavioral test that the string-matching unit tests can't be:
# it boots kanata, feeds it evdev events with timing, and reads kanata's
# OUTPUT device to verify the dual-role CapsLock actually does what we intend:
#
#   caps clicked alone  → F24         (toggle sticky desktop)
#   caps + another key  → Scroll_Lock (momentary desktop)  ← NOT F24
#   Super + letter      → Ctrl + letter (macOS-style remap)
#
# The "caps + key → F24" case is exactly the "caps+arrows doesn't exit" bug:
# if the tap-hold variant is wrong, caps+key emits F24 (toggle) and the submap
# sticks instead of being momentary. This test fails loudly on that.
#
# Why kanata-output rather than full end-to-end through Hyprland: wlroots'
# headless backend doesn't open real evdev devices, so kanata→evdev→Hyprland
# can't run headless. The Hyprland half (which keysym dispatches which submap
# action, exitAfter, etc.) is pinned by the golden unit tests in
# nix/modules/behavior/tests.nix.
{ pkgs
, self
, ...
}:

let
  inherit (pkgs) lib;
  behaviorModule = import ../../modules/behavior { inherit lib pkgs; };
  kanataConfig = behaviorModule.mkKanataConfig { }; # the real default-generated config

  pyenv = pkgs.python3.withPackages (ps: [ ps.evdev ]);

  # The behavioral assertions, as a standalone script (clearer than a heredoc).
  driver = pkgs.writeText "kanata-driver.py" ''
    import subprocess, sys, time, threading
    from evdev import UInput, InputDevice, list_devices, ecodes as e

    def fail(msg):
        print("FAIL:", msg)
        sys.exit(1)

    # 1) Virtual keyboard kanata will grab. Declares every key we inject.
    caps = { e.EV_KEY: [
        e.KEY_CAPSLOCK, e.KEY_LEFT, e.KEY_Q, e.KEY_LEFTMETA, e.KEY_C,
    ] }
    ui = UInput(caps, name="vogix-test-kbd")
    time.sleep(1)

    # 2) Restart kanata so it grabs the new virtual keyboard.
    subprocess.run(["systemctl", "restart", "kanata-default.service"], check=True)
    time.sleep(5)

    # 3) Find kanata's OUTPUT device and read it on a background thread.
    out = None
    for _ in range(20):
        for path in list_devices():
            d = InputDevice(path)
            if d.name == "kanata":
                out = d; break
        if out: break
        time.sleep(0.5)
    if not out:
        fail("kanata output device not found (kanata not running / didn't grab)")

    events = []  # (code, value, timestamp)
    def reader():
        try:
            for ev in out.read_loop():
                if ev.type == e.EV_KEY:
                    events.append((ev.code, ev.value, ev.timestamp()))
        except OSError:
            pass
    threading.Thread(target=reader, daemon=True).start()
    time.sleep(0.5)

    def tap(code, hold=0.08):
        ui.write(e.EV_KEY, code, 1); ui.syn()
        time.sleep(hold)
        ui.write(e.EV_KEY, code, 0); ui.syn()

    def down(code): ui.write(e.EV_KEY, code, 1); ui.syn()
    def up(code):   ui.write(e.EV_KEY, code, 0); ui.syn()

    def codes_since(mark):
        return [c for (c, v, t) in events[mark:]]

    # Warm-up: one caps tap to settle kanata's tap-hold state machine.
    tap(e.KEY_CAPSLOCK); time.sleep(0.6); events.clear()

    # --- Test 1: caps clicked alone → F24 (toggle) ---
    m = len(events)
    tap(e.KEY_CAPSLOCK, hold=0.08)
    time.sleep(0.6)
    c1 = codes_since(m)
    print("caps-tap output:", c1)
    if e.KEY_F24 not in c1:
        fail(f"caps click should emit F24 (toggle), got {c1}")
    if e.KEY_SCROLLLOCK in c1:
        fail(f"caps click must NOT emit Scroll_Lock, got {c1}")
    print("PASS: caps click -> F24 (toggle sticky)")

    # --- Test 2: caps + Left → F23 on press (ENTER), F22 on release (EXIT) ---
    # The exit is a SEPARATE keypress (F22) emitted on caps release, NOT the
    # hold key's release. Hyprland's bindr (release-bind) doesn't fire when the
    # press entered the submap, so the mode would stick; an explicit F22 keypress
    # lets Hyprland exit via a reliable press-bind.
    m = len(events)
    down(e.KEY_CAPSLOCK); time.sleep(0.04)
    tap(e.KEY_LEFT, hold=0.05); time.sleep(0.04)
    up(e.KEY_CAPSLOCK)
    time.sleep(0.6)
    seq = events[m:]                 # (code, value, timestamp) in order
    codes = [c for (c, v, t) in seq]
    print("caps+Left output:", [(c, v) for (c, v, t) in seq])
    if e.KEY_F23 not in codes:
        fail(f"caps+Left should emit F23 on press (ENTER submap), got {seq}")
    if e.KEY_LEFT not in codes:
        fail(f"caps+Left should pass Left through, got {seq}")
    if e.KEY_F22 not in codes:
        fail(f"releasing caps must emit F22 (EXIT submap via press-bind), got {seq}")
    if e.KEY_F24 in codes:
        fail(f"caps+Left must NOT emit F24 (toggle), got {seq}")
    if e.KEY_SCROLLLOCK in codes:
        fail(f"must NOT use Scroll_Lock, got {seq}")
    # Order: F23 (enter) before Left; F22 (exit) after the navigation keys.
    if codes.index(e.KEY_F23) > codes.index(e.KEY_LEFT):
        fail(f"F23 (enter) must precede Left, got {seq}")
    f22_down = [i for i, (c, v, t) in enumerate(seq) if c == e.KEY_F22 and v == 1]
    last_left = max(i for i, (c, v, t) in enumerate(seq) if c == e.KEY_LEFT)
    if not f22_down or f22_down[0] < last_left:
        fail(f"F22 (exit) must come AFTER the navigation keys (on release), got {seq}")
    # CRITICAL: the F22 exit must be HELD long enough for Hyprland to register it.
    # A ~1ms tap is dropped by Hyprland's input pipeline → submap never exits
    # (the real "it stays in the mode" bug). Require >= 30ms between down and up.
    f22_down_t = next(t for (c, v, t) in seq if c == e.KEY_F22 and v == 1)
    f22_up_t = next(t for (c, v, t) in seq if c == e.KEY_F22 and v == 0)
    held_ms = (f22_up_t - f22_down_t) * 1000
    print(f"F22 held for {held_ms:.0f}ms")
    if held_ms < 30:
        fail(f"F22 exit held only {held_ms:.0f}ms — too fast for Hyprland to register (must be >=30ms)")
    print(f"PASS: caps+Left -> F23 enter, Left, F22 exit held {held_ms:.0f}ms (Hyprland-registrable)")

    # --- Test 3: Super + C → Ctrl + C (defoverrides) ---
    m = len(events)
    down(e.KEY_LEFTMETA); time.sleep(0.02)
    down(e.KEY_C); time.sleep(0.03); up(e.KEY_C); time.sleep(0.02)
    up(e.KEY_LEFTMETA)
    time.sleep(0.5)
    c3 = codes_since(m)
    print("Super+C output:", c3)
    # The defoverride swaps Meta+C → Ctrl+C: Meta is passed through until the
    # combo completes, then Ctrl is emitted. So Ctrl MUST appear; Meta may too.
    if e.KEY_LEFTCTRL not in c3:
        fail(f"Super+C should emit Ctrl (macOS remap), got {c3}")
    if e.KEY_C not in c3:
        fail(f"Super+C should emit C, got {c3}")
    print("PASS: Super+C -> Ctrl+C")

    print("ALL KANATA BEHAVIOR TESTS PASSED")
  '';

in
pkgs.testers.nixosTest {
  name = "vogix-keybindings";

  nodes.machine = { ... }: {
    imports = [ self.nixosModules.default ];

    # Real generated kanata config under test (dual-role caps + Super→Ctrl).
    services.kanata = {
      enable = true;
      keyboards.default = {
        devices = [ ]; # grab all keyboards (incl. our virtual one)
        config = kanataConfig;
        extraDefCfg = "process-unmapped-keys yes";
      };
    };

    hardware.uinput.enable = true;
    boot.kernelModules = [ "uinput" ];
    environment.systemPackages = [ pyenv pkgs.evtest ];

    virtualisation = { memorySize = 1024; cores = 2; };
  };

  testScript = ''
    machine.start()
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("kanata-default.service")
    # uinput must be ready before the driver creates a virtual keyboard.
    machine.succeed("modprobe uinput || true")
    machine.wait_until_succeeds("test -e /dev/uinput")

    print(machine.succeed("${pyenv}/bin/python3 ${driver}"))
  '';
}
