# The desktop shell on a real compositor: Hyprland 0.56 on the VM's
# virtio-gpu (DRM backend, software-rendered), started by greetd the way
# the hosts start it, with the home-manager vogix desktop, the input
# engine, PipeWire + WirePlumber and the session bus. Three nodes boot in
# turn: `lua` runs Hyprland's Lua config provider and every gate below
# but the last, `hyprlang` the legacy provider and the gates whose command
# path differs between the two, and `luks` a root filesystem on LUKS for
# the last.
#
# Input goes through the compositor as a user's would: pointer moves and
# clicks over the virtual-pointer protocol (wlrctl), keys through QEMU's
# keyboard, which the input engine grabs and re-emits. Observations are
# hyprctl -j, the `vogix desktop` status verbs (`bar geometry` places the
# click targets), the shell's journal, unit state and, for what only the
# screen shows (the focused workspace box, the focus brackets), the pixels
# of the output as QEMU scans it out.
#
# Gates, one function each (a failing gate is reported and the run goes
# on, so one boot names every failure):
#   mode-border         the input engine paints the mode's border colour
#                       at session start, read back from the compositor
#   mode-border-stall   an engine started while the compositor has not
#                       answered yet still paints it, once it answers
#   mode-border-reload  a config reload resets the border; the engine
#                       paints the mode's colour again
#   network-backend     NetworkManager starting after the shell restarts
#                       the shell exactly once, cleanly, and it attaches
#   keyboard-active     the LANG cell lights the keyboard's active layout by
#                       index, following switches
#   keyboard-reload     the LANG cell follows a layout list changed at runtime
#   workspace-click     clicking a workspace box focuses that workspace
#   focus-brackets      accent brackets mark the focused window's corners
#   panel-placement     a bar widget's panel opens beside its bar, centred
#                       on the widget; a verb-opened one under the top bar
#   notification-place  notification cards clear the top bar and right rail
#   tray-menu           a tray item's menu opens against its icon
#   screencast          a screen capture session lights PRIVACY while it runs
#   vu-out              a −6 dBFS tone reads −6 dB on the output VU, at full
#                       and at half sink volume
#   vu-mic              the same tone through a virtual microphone reads
#                       −6 dB on the MIC VU
#   taps-pipewire       spectrum and scope taps return after PipeWire restarts
#   lock-sampling       a locked session samples nothing; unlocking resumes
#   network-lock        a NetworkManager restart due while locked waits for
#                       the unlock
#   root-gauge-device   the root gauge names the dm-N device a LUKS root
#                       is counted under in /proc/diskstats
{ pkgs
, home-manager
, self
, ...
}:

let
  inherit (pkgs) lib;

  # A StatusNotifierItem with a one-entry menu, as Qt exports any tray
  # application's: the item and its menu over com.canonical.dbusmenu. The
  # entry logs when it is triggered.
  trayIcon = pkgs.runCommand "vogix-test-tray.png" { nativeBuildInputs = [ pkgs.imagemagick ]; } ''
    magick -size 32x32 xc:'#ff00ff' png:$out
  '';
  trayQml = pkgs.writeText "vogix-test-tray.qml" ''
    import QtQuick
    import Qt.labs.platform

    SystemTrayIcon {
        visible: true
        icon.source: "file://${trayIcon}"
        tooltip: "vogix test tray"
        menu: Menu {
            MenuItem {
                text: "vogix-test-item"
                onTriggered: console.log("TRAY-ACTIVATED vogix-test-item")
            }
        }
    }
  '';
  trayClient = pkgs.writeShellScriptBin "vogix-test-tray" ''
    export QT_PLUGIN_PATH=${pkgs.qt6.qtbase}/${pkgs.qt6.qtbase.qtPluginPrefix}:${pkgs.qt6.qtwayland}/${pkgs.qt6.qtbase.qtPluginPrefix}
    export QML_IMPORT_PATH=${pkgs.qt6.qtdeclarative}/${pkgs.qt6.qtbase.qtQmlPrefix}
    exec ${pkgs.qt6.qtdeclarative}/bin/qml --apptype widget ${trayQml}
  '';

  # The VU reference: a 30 s, 1 kHz stereo sine peaking at −6.00 dBFS. 32-bit
  # float, so no dither moves the peak, and 48 samples a period at 48 kHz,
  # so every period has a sample on the crest. The build fails unless sox
  # measures that peak.
  tone = pkgs.runCommand "vogix-test-tone-6dbfs.wav" { nativeBuildInputs = [ pkgs.sox pkgs.gawk ]; } ''
    sox -n -r 48000 -c 2 -e floating-point -b 32 -t wav $out synth 30 sine 1000 vol -6dB
    peak=$(sox $out -n stat 2>&1 | awk '/^Maximum amplitude/ { print $3 }')
    awk -v p="$peak" 'BEGIN {
      db = 20 * log(p) / log(10)
      if (db < -6.005 || db > -5.995) { printf "tone peak %s is %.4f dBFS\n", p, db; exit 1 }
    }'
  '';

  # The root filesystem on LUKS, as an encrypted install has it. The node's
  # root disk starts empty: on first boot the initrd LUKS-formats it and
  # puts ext4 inside, then systemd-cryptsetup opens it with a key file as
  # /dev/mapper/cryptroot. The filesystem is made before that open because
  # systemd holds back an encrypted device with no filesystem on it
  # (99-systemd.rules: it may still be being formatted), so a mount that
  # formats on demand would wait for it forever.
  luksKey = pkgs.writeText "vogix-test-luks.key" "vogix test root key\n";
  luksRoot = { pkgs, ... }:
    let
      disk = "/dev/disk/by-id/virtio-root";
      diskUnit = "dev-disk-by\\x2did-virtio\\x2droot.device";
      key = "/etc/vogix-test-luks.key";
    in
    {
      virtualisation.useDefaultFilesystems = false;
      virtualisation.fileSystems."/" = {
        device = "/dev/mapper/cryptroot";
        fsType = "ext4";
      };
      boot.initrd.systemd.enable = true;
      boot.initrd.luks.devices.cryptroot = {
        device = disk;
        keyFile = key;
      };
      boot.initrd.systemd.contents.${key}.source = luksKey;
      boot.initrd.systemd.extraBin.cryptsetup = "${pkgs.cryptsetup}/bin/cryptsetup";
      boot.initrd.systemd.services.vogix-test-luks-format = {
        description = "LUKS-format the empty root disk, with ext4 inside";
        requiredBy = [ "systemd-cryptsetup@cryptroot.service" ];
        before = [ "systemd-cryptsetup@cryptroot.service" ];
        requires = [ diskUnit ];
        after = [ diskUnit ];
        unitConfig.DefaultDependencies = false;
        serviceConfig.Type = "oneshot";
        script = ''
          if ! cryptsetup isLuks ${disk}; then
            cryptsetup luksFormat -q --iter-time=1 --key-file=${key} ${disk}
            cryptsetup open --key-file=${key} ${disk} vogix-test-mkfs
            mkfs.ext4 -q /dev/mapper/vogix-test-mkfs
            cryptsetup close vogix-test-mkfs
          fi
        '';
      };
    };

  node = { dialect, networkManager, luks ? false }: { ... }: {
    imports = [
      self.nixosModules.default
      home-manager.nixosModules.home-manager
    ] ++ lib.optional luks luksRoot;

    nixpkgs.overlays = [ self.overlays.default ];

    vogix.enable = true;

    users.users.vogix = {
      isNormalUser = true;
      uid = 1000;
      password = "vogix";
    };

    # greetd starts the session as it does on the hosts; the initial
    # session logs the user straight in.
    programs.hyprland.enable = true;
    services.greetd = {
      enable = true;
      settings = {
        default_session.command = "${pkgs.greetd}/bin/agreety --cmd start-hyprland";
        initial_session = {
          command = "start-hyprland";
          user = "vogix";
        };
      };
    };

    # No sound hardware: a null sink WirePlumber picks as the default.
    services.pipewire = {
      enable = true;
      wireplumber.enable = true;
      extraConfig.pipewire."90-vogix-null-sink" = {
        "context.objects" = [{
          factory = "adapter";
          args = {
            "factory.name" = "support.null-audio-sink";
            "node.name" = "vogix-null-sink";
            "node.description" = "VM null sink";
            "media.class" = "Audio/Sink";
            "audio.position" = [ "FL" "FR" ];
            "monitor.channel-volumes" = true;
          };
        }];
      };
    };

    # Installed but not started at boot: the shell comes up without it,
    # and the test starts it.
    networking.networkmanager = lib.mkIf networkManager {
      enable = true;
      unmanaged = [ "*" ];
    };
    systemd.services = lib.mkIf networkManager {
      NetworkManager.wantedBy = lib.mkForce [ ];
      NetworkManager-wait-online.wantedBy = lib.mkForce [ ];
    };

    environment.systemPackages = [
      pkgs.jq
      pkgs.wlrctl
      pkgs.wl-mirror
      pkgs.foot
      pkgs.libnotify
      trayClient
    ];

    # The graphical session's environment for commands the test runs as
    # the user: the user manager holds what Hyprland exported.
    environment.etc."vogix-session.sh".text = ''
      export XDG_RUNTIME_DIR=/run/user/$(id -u)
      export DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus
      eval "$(systemctl --user show-environment \
        | grep -E '^(WAYLAND_DISPLAY|HYPRLAND_INSTANCE_SIGNATURE|XDG_CURRENT_DESKTOP|XDG_SESSION_TYPE)=' \
        | sed 's/^/export /')"
    '';

    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;
      sharedModules = [ self.homeManagerModules.default ];
      users.vogix = {
        home.stateVersion = "24.11";
        programs.vogix = {
          enable = true;
          appearance.prebuiltThemes = [ "yoga" ];
          # A layout whose first entry is not English, so the active entry
          # is told apart by index.
          behavior.input.kbLayout = "de,us";
          desktop = {
            enable = true;
            # The default layout and bars; only the idle stages are off, so
            # a slow run is never dimmed, blanked or locked under a gate.
            idle = {
              dim = null;
              lock = null;
              screenOff = null;
            };
          };
        };
        wayland.windowManager.hyprland = {
          enable = true;
          package = null;
          portalPackage = null;
          configType = dialect;
        };
      };
    };

    virtualisation = {
      memorySize = 4096;
      cores = 4;
      qemu.options = [ "-vga none" "-device virtio-gpu-pci,xres=1920,yres=1080" ];
    };
    system.stateVersion = "24.11";
  };
in
pkgs.testers.nixosTest {
  name = "vogix-desktop-hyprland";

  nodes = {
    lua = node { dialect = "lua"; networkManager = true; };
    hyprlang = node { dialect = "hyprlang"; networkManager = false; };
    luks = node { dialect = "lua"; networkManager = false; luks = true; };
  };

  testScript = ''
    import json
    import os
    import re
    import shlex
    import time
    from typing import Any, Callable

    Rect = tuple[int, int, int, int]
    failures: list[str] = []

    PASSWORD = "vogix"
    NM_LINE = "NetworkBackend: NetworkManager appeared after the shell started; restarting to attach"


    def gate(name: str, fn: Callable[[], None]) -> None:
        print(f"=== GATE {name}")
        try:
            fn()
            print(f"PASS {name}")
        except Exception as e:
            print(f"FAIL {name}: {e}")
            failures.append(f"{name}: {e}")


    class Desktop:
        def __init__(self, m: Any, dialect: str) -> None:
            self.m = m
            self.dialect = dialect
            self.width = 0
            self.height = 0

        def user(self, cmd: str) -> str:
            return "su - vogix -c " + shlex.quote(". /etc/vogix-session.sh; " + cmd)

        def run(self, cmd: str) -> str:
            return self.m.succeed(self.user(cmd))

        def attempt(self, cmd: str) -> str:
            return self.m.execute(self.user(cmd + " 2>&1"))[1]

        def poll(self, cmd: str, ok: Callable[[str], bool], timeout: float, what: str) -> str:
            deadline = time.monotonic() + timeout
            last = ""
            while True:
                last = self.attempt(cmd).strip()
                if ok(last):
                    return last
                if time.monotonic() > deadline:
                    raise Exception(f"{what}: not within {timeout} s (last `{cmd}`: {last!r})")
                time.sleep(0.2)

        def boot(self) -> None:
            self.m.start()
            self.m.wait_for_unit("multi-user.target")
            self.m.wait_until_succeeds(self.user("hyprctl -j monitors | jq -e 'length > 0'"), timeout=120)
            mon = json.loads(self.run("hyprctl -j monitors"))[0]
            self.width, self.height = mon["width"], mon["height"]
            status = json.loads(self.run("hyprctl -j status"))
            assert status["configProvider"] == self.dialect, f"config provider: {status}"
            self.wait_shell()

        def wait_shell(self) -> None:
            self.poll("vogix desktop status", lambda o: "shell: running" in o, 120, "shell answers")
            self.poll("vogix desktop bar geometry", lambda o: " top workspaces " in o, 60, "bars placed")

        def dispatch(self, lua: str, hyprlang: str) -> None:
            self.run("hyprctl dispatch " + shlex.quote(lua if self.dialect == "lua" else hyprlang))

        def focus_workspace(self, n: int) -> None:
            self.dispatch(f'hl.dsp.focus({{ workspace = "{n}" }})', f"workspace {n}")
            self.poll("hyprctl -j activeworkspace", lambda o: json.loads(o)["id"] == n, 10,
                      f"workspace {n} focused")

        def widget(self, edge: str, name: str) -> Rect:
            for line in self.run("vogix desktop bar geometry").splitlines():
                f = line.split()
                if len(f) == 7 and f[1] == edge and f[2] == name:
                    return int(f[3]), int(f[4]), int(f[5]), int(f[6])
            raise Exception(f"no {edge} {name} in bar geometry")

        def layers(self) -> list[dict[str, Any]]:
            out = []
            for mon in json.loads(self.run("hyprctl -j layers")).values():
                for level, surfaces in mon["levels"].items():
                    for s in surfaces:
                        out.append(dict(s, level=int(level)))
            return out

        def await_layer(self, ok: Callable[[dict[str, Any]], bool], timeout: float, what: str) -> dict[str, Any]:
            deadline = time.monotonic() + timeout
            while True:
                found = [s for s in self.layers() if ok(s)]
                if found:
                    return found[0]
                if time.monotonic() > deadline:
                    raise Exception(f"{what}: no such layer within {timeout} s: {self.layers()}")
                time.sleep(0.2)

        def await_no_layer(self, ok: Callable[[dict[str, Any]], bool], timeout: float, what: str) -> None:
            deadline = time.monotonic() + timeout
            while [s for s in self.layers() if ok(s)]:
                if time.monotonic() > deadline:
                    raise Exception(f"{what}: layer still mapped after {timeout} s")
                time.sleep(0.2)

        def pointer_to(self, x: int, y: int) -> None:
            for _ in range(6):
                cx, cy = (int(float(v)) for v in self.run("hyprctl cursorpos").split(","))
                if (cx, cy) == (x, y):
                    return
                self.run(f"wlrctl pointer move {x - cx} {y - cy}")
            raise Exception(f"pointer did not reach {x},{y}")

        def click(self, x: int, y: int, button: str = "left") -> None:
            self.pointer_to(x, y)
            self.run(f"wlrctl pointer click {button}")

        def journal(self, unit: str) -> str:
            return self.m.succeed(f"journalctl --no-pager -o cat _SYSTEMD_USER_UNIT={unit}")

        def prop(self, unit: str, name: str) -> str:
            return self.run(f"systemctl --user show {unit} -p {name} --value").strip()

        def lock(self) -> None:
            self.run("vogix desktop lock")
            self.poll("vogix desktop lock status", lambda o: o == "secure", 10, "session locked")

        def unlock(self) -> None:
            self.m.send_chars(PASSWORD + "\n")
            self.poll("vogix desktop lock status", lambda o: o == "unlocked", 15, "session unlocked")

        def desktop_json(self) -> dict[str, Any]:
            return json.loads(self.run("cat ~/.local/state/vogix/desktop.json"))

        def bar_sizes(self) -> dict[str, int]:
            return {e: (b["size"] if b["enable"] else 0) for e, b in self.desktop_json()["bars"].items()}

        def accent(self) -> tuple[int, int, int]:
            slot = self.desktop_json()["surfaces"]["bar"]["accent"]["slot"]
            theme = json.loads(self.run("cat ~/.config/vogix-desktop/theme.json"))
            hexa = theme["semantic"][slot].lstrip("#")
            return int(hexa[0:2], 16), int(hexa[2:4], 16), int(hexa[4:6], 16)

        def find_color(self, area: Rect, color: tuple[int, int, int]) -> Rect | None:
            """The bounding box of `color` inside `area`, read from the output."""
            path = os.path.join(os.environ.get("TMPDIR", "/tmp"), "vogix-screen.ppm")
            self.m.send_monitor_command(f"screendump {path}")
            with open(path, "rb") as f:
                data = f.read()
            magic, dims, _depth, pixels = data.split(b"\n", 3)
            assert magic == b"P6", "screendump is not a binary PPM"
            w = int(dims.split()[0])
            ax, ay, aw, ah = area
            xs: list[int] = []
            ys: list[int] = []
            for y in range(ay, ay + ah):
                row = (y * w) * 3
                for x in range(ax, ax + aw):
                    i = row + x * 3
                    px = pixels[i], pixels[i + 1], pixels[i + 2]
                    if all(abs(px[k] - color[k]) <= 6 for k in range(3)):
                        xs.append(x)
                        ys.append(y)
            if not xs:
                return None
            return min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1


    def gate_network_backend(d: Desktop) -> None:
        # The shell started while NetworkManager was down.
        assert "Network will not work" in d.journal("vogix-desktop.service"), \
            "the shell did not start without a network backend"
        pid, restarts = d.prop("vogix-desktop", "MainPID"), int(d.prop("vogix-desktop", "NRestarts"))
        d.m.succeed("systemctl start NetworkManager")
        d.poll("systemctl --user show vogix-desktop -p MainPID --value",
               lambda o: o not in ("", "0", pid), 10, "shell restarted after NetworkManager appeared")
        after = int(d.prop("vogix-desktop", "NRestarts"))
        assert after == restarts + 1, f"NRestarts {restarts} -> {after}, expected one restart"
        assert d.prop("vogix-desktop", "Result") == "success", "the restart was not a clean exit"
        count = d.journal("vogix-desktop.service").count(NM_LINE)
        assert count == 1, f"the attach restart was logged {count} times"
        d.wait_shell()


    GET_BORDER = "hyprctl -j getoption general:col.active_border"


    def border_colour(reply: str) -> str:
        """The first colour of a getoption reply for the active border, AARRGGBB."""
        try:
            doc = json.loads(reply)
        except ValueError:
            return ""
        # A gradient reads as "<colour>... <angle>deg"; the Lua provider
        # reports it as `gradient`, hyprlang as a `custom` type.
        return (str(doc.get("gradient") or doc.get("custom") or "").split() or [""])[0].lower()


    def mode_border(d: Desktop) -> str:
        """The border the engine owes the current mode: its slot in input.json's
        modeColors, resolved through the current theme's semantic palette."""
        state = "~/.local/state/vogix"
        mode = d.run(f"cat {state}/current-mode").strip()
        slot = json.loads(d.run(f"cat {state}/input.json"))["modeColors"][mode]["slot"]
        theme = json.loads(d.run(f"cat {state}/current-theme/vogix-desktop/theme.json"))
        return "ff" + theme["semantic"][slot].lstrip("#").lower()


    def set_active_border(d: Desktop, rgb: str) -> None:
        if d.dialect == "lua":
            d.run("hyprctl eval " + shlex.quote(f'hl.config({{ ["general.col.active_border"] = "rgb({rgb})" }})'))
        else:
            d.run("hyprctl keyword general:col.active_border " + shlex.quote(f"rgb({rgb})"))


    def gate_mode_border(d: Desktop) -> None:
        # Hyprland's own default is white and the config sets no border, so
        # the mode's colour is there only if the engine's write landed.
        want = mode_border(d)
        d.poll(GET_BORDER, lambda o: border_colour(o) == want, 5,
               f"the active border is the mode's colour {want}")


    def gate_mode_border_stall(d: Desktop) -> None:
        # The engine starts while the compositor accepts connections but has
        # not answered them yet, as at session start. It must not write in a
        # guessed dialect, and the border must be painted once it answers.
        want, sentinel = mode_border(d), "123456"
        set_active_border(d, sentinel)
        d.poll(GET_BORDER, lambda o: border_colour(o) == "ff" + sentinel, 5, "the sentinel border is set")
        pid = json.loads(d.run("hyprctl instances -j"))[0]["pid"]
        running = "journalctl --user -u vogix-input -o cat | grep -c 'vogix input running'"
        started = int(d.attempt(running).strip() or "0")
        d.run("systemctl --user stop vogix-input")
        d.m.succeed(f"kill -STOP {pid}")
        try:
            d.run("systemctl --user start vogix-input")
            d.poll(running, lambda o: o.isdigit() and int(o) > started, 15,
                   "the engine started while the compositor was stopped")
        finally:
            d.m.succeed(f"kill -CONT {pid}")
        d.poll(GET_BORDER, lambda o: border_colour(o) == want, 10,
               f"the mode's colour {want} is painted once the compositor answers")


    TONE = "${tone}"
    TONE_DB = -6.0


    def vu_tolerance(d: Desktop) -> float:
        """How far a VU reading of a steady tone may be from the tone's level.

        The meters publish their level quantized to 1/steps of the
        [floorDb, 0] window (Ballistics.steps), rounded to nearest, so a
        reading is at most half a step from the level it shows. The attack
        is instant and the release and the peak cap act only when the input
        falls, so a steady tone's level sits on its peak every tick, and the
        rounding is the whole error."""
        qml = d.run("cat ~/.config/quickshell/vogix/Components/Ballistics.qml")
        found = re.search(r"readonly property int steps: (\d+)", qml)
        assert found, "Ballistics.qml has no steps property"
        steps = int(found.group(1))
        floor = ((d.desktop_json().get("meters") or {}).get("vu") or {}).get("floorDb", -40)
        step = abs(floor) / steps
        reported = json.loads(d.run("vogix desktop vu"))["stepDb"]
        assert abs(reported - step) < 1e-9, f"vu reports a {reported} dB step, the meter's is {step}"
        return step / 2


    def vu_holds(d: Desktop, key: str, want: float, tol: float, what: str) -> None:
        """`key` of `vogix desktop vu` reads want ± tol, and keeps reading it
        for longer than the peak cap holds."""
        def ok(reply: str) -> bool:
            try:
                value = json.loads(reply)[key]
            except (ValueError, KeyError):
                return False
            values = value if isinstance(value, list) else [value]
            return None not in values and all(abs(v - want) <= tol for v in values)

        d.poll("vogix desktop vu", ok, 15, f"{what} reads {want} dB ± {tol}")
        end = time.monotonic() + 1.5
        while time.monotonic() < end:
            reply = d.attempt("vogix desktop vu").strip()
            assert ok(reply), f"{what} left {want} dB ± {tol} while the tone played: {reply}"
            time.sleep(0.2)
        print(f"VU {what}: {reply}")


    def node_id(d: Desktop, name: str) -> int:
        query = ("pw-dump | jq -r '.[] | select(.type == \"PipeWire:Interface:Node\""
                 + f" and .info.props[\"node.name\"] == \"{name}\") | .id'")
        return int(d.poll(query, lambda o: o.isdigit(), 10, f"PipeWire node {name}"))


    def gate_vu_out(d: Desktop) -> None:
        tol = vu_tolerance(d)
        before = d.run("wpctl get-volume @DEFAULT_AUDIO_SINK@").split()[1]
        d.run("wpctl set-volume @DEFAULT_AUDIO_SINK@ 1.0")
        d.run(f"systemd-run --user --collect --unit=vogix-test-tone pw-play {TONE}")
        try:
            vu_holds(d, "out", TONE_DB, tol, "the output VU at full volume")
            # This sink's monitor carries its volume (monitor.channel-volumes),
            # which quickshell divides back out: the meter reads what
            # applications send, before the sink's volume.
            d.run("wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.5")
            vu_holds(d, "out", TONE_DB, tol, "the output VU at half volume")
        finally:
            d.attempt("systemctl --user stop vogix-test-tone")
            d.run(f"wpctl set-volume @DEFAULT_AUDIO_SINK@ {before}")


    def gate_vu_mic(d: Desktop) -> None:
        # A virtual microphone: pw-loopback's sink side takes the tone and its
        # source side, made the default input, carries it to the MIC meter.
        tol = vu_tolerance(d)
        d.run("systemd-run --user --collect --unit=vogix-test-mic pw-loopback -m '[ FL FR ]'"
              " --capture-props='media.class=Audio/Sink node.name=vogix-test-mic-in'"
              " --playback-props='media.class=Audio/Source node.name=vogix-test-mic'")
        try:
            sink, source = node_id(d, "vogix-test-mic-in"), node_id(d, "vogix-test-mic")
            d.run(f"wpctl set-volume {sink} 1.0 && wpctl set-volume {source} 1.0 && wpctl set-default {source}")
            d.run(f"systemd-run --user --collect --unit=vogix-test-mic-tone pw-play --target vogix-test-mic-in {TONE}")
            vu_holds(d, "mic", TONE_DB, tol, "the MIC VU")
        finally:
            d.attempt("systemctl --user stop vogix-test-mic-tone vogix-test-mic")


    def gate_root_gauge_device(d: Desktop) -> None:
        # The kernel device behind "/", found apart from the shell's own
        # probe: the root's major:minor under /sys/dev/block.
        majmin = d.m.succeed("findmnt -no MAJ:MIN /").strip()
        dev = d.m.succeed(f"basename \"$(readlink -f /sys/dev/block/{majmin})\"").strip()
        uuid = d.m.succeed(f"cat /sys/block/{dev}/dm/uuid").strip()
        assert uuid.startswith("CRYPT-LUKS"), f"the root {dev} is not a LUKS mapping ({uuid!r})"
        assert f" {dev} " in d.m.succeed("cat /proc/diskstats"), f"/proc/diskstats has no {dev} row"
        stats = d.poll("vogix desktop stats", lambda o: '"/"' in o and "gaugeDevice" in o, 30,
                       "the shell's block-device probe answered")
        got = json.loads(stats)["gaugeDevice"].get("/")
        assert got is not None and re.fullmatch(r"dm-[0-9]+", got), \
            f"the root gauge's device is {got!r}, not the dm-N /proc/diskstats counts"
        assert got == dev, f"the root gauge reads {got}, the root is on {dev}"


    def gate_mode_border_reload(d: Desktop) -> None:
        # A config reload, as Hyprland's autoreload runs after a switch,
        # resets every value set at runtime; the engine paints the mode's
        # border again.
        want = mode_border(d)
        d.poll(GET_BORDER, lambda o: border_colour(o) == want, 5, "the mode's colour before the reload")
        d.run("hyprctl reload")
        d.poll(GET_BORDER, lambda o: border_colour(o) == want, 5,
               f"the mode's colour {want} is painted again after a config reload")


    def set_layouts(d: Desktop, layouts: str) -> None:
        if d.dialect == "lua":
            d.run("hyprctl eval " + shlex.quote(f'hl.config({{ input = {{ kb_layout = "{layouts}" }} }})'))
        else:
            d.run(f"hyprctl keyword input:kb_layout {layouts}")


    def gate_keyboard_active(d: Desktop) -> None:
        kb = "vogix desktop keyboard"
        d.poll(kb, lambda o: o.startswith("device:vogix-input layouts:de,us active:de "), 10,
               "LANG follows vogix-input with de active")
        d.run("hyprctl switchxkblayout vogix-input next")
        d.poll(kb, lambda o: " active:us " in o, 2, "LANG follows the switch to us")
        d.run("hyprctl switchxkblayout vogix-input next")
        d.poll(kb, lambda o: " active:de " in o, 2, "LANG follows the switch back to de")


    def gate_keyboard_reload(d: Desktop) -> None:
        kb = "vogix desktop keyboard"
        d.poll(kb, lambda o: " layouts:de,us " in o, 10, "LANG starts on de,us")
        set_layouts(d, "fr,de")
        try:
            d.poll(kb, lambda o: " layouts:fr,de " in o, 2, "LANG follows a runtime layout change")
        finally:
            set_layouts(d, "de,us")


    def gate_workspace_click(d: Desktop) -> None:
        # A window on each workspace keeps both boxes on the bar.
        d.focus_workspace(1)
        d.run("systemd-run --user --collect --unit=vogix-test-term1 foot")
        d.poll("hyprctl -j clients", lambda o: len(json.loads(o)) >= 1, 30, "window on workspace 1")
        d.focus_workspace(2)
        d.run("systemd-run --user --collect --unit=vogix-test-term2 foot")
        d.poll("hyprctl -j clients", lambda o: any(c["workspace"]["id"] == 2 for c in json.loads(o)), 30,
               "window on workspace 2")
        # The focused box is filled with the bar accent: with workspace 2
        # focused, that box is workspace 2's.
        cell, accent = d.widget("top", "workspaces"), d.accent()
        box: Rect | None = None
        deadline = time.monotonic() + 5
        while box is None or box[2] < 8:
            box = d.find_color(cell, accent)
            if time.monotonic() > deadline:
                raise Exception(f"no focused box drawn in the WS cell {cell}")
            time.sleep(0.2)
        d.focus_workspace(1)
        d.m.screenshot(f"{d.dialect}-workspaces")
        before = d.journal("vogix-desktop.service").count("Dispatch request")
        bx, by, bw, bh = box
        d.click(bx + bw // 2, by + bh // 2)
        d.poll("hyprctl -j activeworkspace", lambda o: json.loads(o)["id"] == 2, 2,
               "the click on workspace 2's box focused it")
        failed = d.journal("vogix-desktop.service").count("Dispatch request") - before
        assert failed == 0, "quickshell logged a failed dispatch"
        d.focus_workspace(1)


    def gate_focus_brackets(d: Desktop) -> None:
        if json.loads(d.run("hyprctl -j clients")) == []:
            d.run("systemd-run --user --collect --unit=vogix-test-term0 foot")
        win = d.poll("hyprctl -j activewindow", lambda o: o.startswith("{") and "at" in json.loads(o), 30,
                     "a focused window")
        doc = json.loads(win)
        x, y, w, h = doc["at"][0], doc["at"][1], doc["size"][0], doc["size"][1]
        accent, arm, pad = d.accent(), 14, 3
        # An accent L-bracket just outside two opposite corners.
        for corner, (bx, by) in {"top-left": (x - pad, y - pad),
                                 "bottom-right": (x + w + pad - arm, y + h + pad - arm)}.items():
            got: Rect | None = None
            deadline = time.monotonic() + 3
            while got != (bx, by, arm, arm):
                got = d.find_color((bx - 4, by - 4, arm + 8, arm + 8), accent)
                if time.monotonic() > deadline:
                    raise Exception(f"{corner} bracket: want {(bx, by, arm, arm)}, drawn {got}")
                time.sleep(0.2)
        d.m.screenshot(f"{d.dialect}-brackets")


    def gate_panel_placement(d: Desktop) -> None:
        w, h, bars = d.width, d.height, d.bar_sizes()
        top, bottom, left, right = bars["top"], bars["bottom"], bars["left"], bars["right"]
        x, y, cw, ch = d.widget("left", "audio-out-picker")
        # The cell's body opens the device list (its glyph toggles mute).
        d.click(x + cw // 2, y + ch * 5 // 6)
        panel = d.await_layer(lambda s: s["w"] == 380, 5, "audio-out panel")
        d.m.screenshot("panel-from-rail")
        assert panel["x"] == left + 8, f"panel x {panel['x']}: not beside the left rail ({left + 8})"
        low, high = top + 8, h - bottom - 8 - panel["h"]
        want = min(max(round(y + ch / 2 - panel["h"] / 2), low), high)
        assert abs(panel["y"] - want) <= 1, f"panel y {panel['y']}: not centred on the cell ({want})"
        d.run("vogix desktop panel --close")
        d.await_no_layer(lambda s: s["w"] == 380, 5, "panel closed")
        d.run("vogix desktop panel audio-out")
        panel = d.await_layer(lambda s: s["w"] == 380, 5, "verb-opened panel")
        assert panel["x"] + 380 == w - right - 8 and panel["y"] == top + 8, \
            f"verb-opened panel at {panel['x']},{panel['y']}"
        d.run("vogix desktop panel --close")
        d.await_no_layer(lambda s: s["w"] == 380, 5, "panel closed")


    def gate_notification_placement(d: Desktop) -> None:
        w, bars = d.width, d.bar_sizes()
        d.run("notify-send -a vogix-test -t 60000 Placement 'a card on the desktop'")
        card = d.await_layer(lambda s: s["w"] == 440, 10, "notification column")
        d.m.screenshot("notification")
        assert card["y"] >= bars["top"] + 8, f"card top {card['y']} overlaps the top bar"
        assert card["x"] + 440 <= w - bars["right"] - 8, f"card right {card['x'] + 440} overlaps the right rail"
        d.run("vogix desktop notify dismiss --all")


    def gate_tray_menu(d: Desktop) -> None:
        d.run("systemd-run --user --collect --unit=vogix-test-tray vogix-test-tray")
        deadline = time.monotonic() + 30
        while True:
            tx, ty, tw, th = d.widget("bottom", "tray")
            if tw > 0:
                break
            if time.monotonic() > deadline:
                raise Exception("the test item never reached the tray")
            time.sleep(0.5)
        d.click(tx + tw // 2, ty + th // 2, "right")
        time.sleep(1.5)
        d.m.screenshot("tray-menu")
        # The menu opens against the icon, growing away from the bottom
        # edge: its entry lies just above the icon.
        d.click(tx + tw // 2, ty - 10)
        d.poll("journalctl --user -u vogix-test-tray -o cat", lambda o: "TRAY-ACTIVATED vogix-test-item" in o,
               5, "the menu entry above the icon was triggered")


    def gate_screencast(d: Desktop) -> None:
        p = "vogix desktop privacy"
        d.poll(p, lambda o: "screencast:off" in o, 5, "no capture before the test")
        d.run("systemd-run --user --collect --unit=vogix-test-cast wl-mirror --backend screencopy-shm Virtual-1")
        d.poll(p, lambda o: "screencast:on" in o, 10, "PRIVACY lights while the screen is captured")
        d.run("systemctl --user stop vogix-test-cast")
        d.poll(p, lambda o: "screencast:off" in o, 2, "PRIVACY clears when the capture ends")


    def taps() -> str:
        return "echo $(pgrep -u vogix -x cava) $(pgrep -u vogix -x pw-record)"


    def gate_taps_pipewire(d: Desktop) -> None:
        # A player that keeps a playback stream open, reconnecting when
        # PipeWire goes away and comes back.
        d.run("systemd-run --user --collect --unit=vogix-test-play sh -c "
              + shlex.quote("while :; do pw-play --raw --format=s16 --rate=48000 --channels=2 /dev/zero; sleep 0.5; done"))
        running = "spectrum:running scope:running"
        d.poll("vogix desktop meters", lambda o: running in o, 30, "taps run while something plays")
        old = d.run(taps()).split()
        assert len(old) == 2, f"expected a cava and a pw-record process, got {old}"
        d.run("systemctl --user restart pipewire")
        d.poll("vogix desktop meters", lambda o: running in o and not set(d.run(taps()).split()) & set(old), 15,
               "taps back after the PipeWire restart")


    def gate_lock_sampling(d: Desktop) -> None:
        pid = d.prop("vogix-desktop", "MainPID")
        d.lock()
        try:
            d.poll("vogix desktop meters",
                   lambda o: o.endswith("spectrum:off scope:off vu-out:off vu-mic:off stats:none"), 5,
                   "a locked session samples nothing")
            d.poll(taps(), lambda o: o == "", 3, "no tap process left while locked")
            d.m.screenshot("locked")
        finally:
            d.unlock()
        d.poll("vogix desktop meters", lambda o: "spectrum:running" in o and not o.endswith("stats:none"), 10,
               "sampling resumes after the unlock")
        assert d.prop("vogix-desktop", "MainPID") == pid, "the shell restarted across the lock"


    def gate_network_lock(d: Desktop) -> None:
        d.m.succeed("systemctl stop NetworkManager")
        d.run("systemctl --user restart vogix-desktop")
        d.wait_shell()
        pid = d.prop("vogix-desktop", "MainPID")
        d.lock()
        try:
            d.m.succeed("systemctl start NetworkManager")
            time.sleep(5)
            assert d.prop("vogix-desktop", "MainPID") == pid, "the shell restarted under the lock"
        finally:
            d.unlock()
        d.poll("systemctl --user show vogix-desktop -p MainPID --value",
               lambda o: o not in ("", "0", pid), 10, "shell restarted once unlocked")
        count = d.journal("vogix-desktop.service").count(NM_LINE)
        assert count == 2, f"the attach restart was logged {count} times over both starts"
        d.wait_shell()


    lua_desktop = Desktop(lua, "lua")
    lua_desktop.boot()
    gate("lua mode-border", lambda: gate_mode_border(lua_desktop))
    gate("lua network-backend", lambda: gate_network_backend(lua_desktop))
    gate("lua keyboard-active", lambda: gate_keyboard_active(lua_desktop))
    gate("lua keyboard-reload", lambda: gate_keyboard_reload(lua_desktop))
    gate("lua workspace-click", lambda: gate_workspace_click(lua_desktop))
    gate("lua focus-brackets", lambda: gate_focus_brackets(lua_desktop))
    gate("lua panel-placement", lambda: gate_panel_placement(lua_desktop))
    gate("lua notification-place", lambda: gate_notification_placement(lua_desktop))
    gate("lua tray-menu", lambda: gate_tray_menu(lua_desktop))
    gate("lua screencast", lambda: gate_screencast(lua_desktop))
    gate("lua vu-out", lambda: gate_vu_out(lua_desktop))
    gate("lua vu-mic", lambda: gate_vu_mic(lua_desktop))
    gate("lua taps-pipewire", lambda: gate_taps_pipewire(lua_desktop))
    gate("lua lock-sampling", lambda: gate_lock_sampling(lua_desktop))
    gate("lua network-lock", lambda: gate_network_lock(lua_desktop))
    gate("lua mode-border-stall", lambda: gate_mode_border_stall(lua_desktop))
    gate("lua mode-border-reload", lambda: gate_mode_border_reload(lua_desktop))
    lua.shutdown()

    hyprlang_desktop = Desktop(hyprlang, "hyprlang")
    hyprlang_desktop.boot()
    gate("hyprlang mode-border", lambda: gate_mode_border(hyprlang_desktop))
    gate("hyprlang keyboard-active", lambda: gate_keyboard_active(hyprlang_desktop))
    gate("hyprlang keyboard-reload", lambda: gate_keyboard_reload(hyprlang_desktop))
    gate("hyprlang workspace-click", lambda: gate_workspace_click(hyprlang_desktop))
    gate("hyprlang mode-border-stall", lambda: gate_mode_border_stall(hyprlang_desktop))
    gate("hyprlang mode-border-reload", lambda: gate_mode_border_reload(hyprlang_desktop))
    hyprlang.shutdown()

    luks_desktop = Desktop(luks, "lua")
    luks_desktop.boot()
    gate("luks root-gauge-device", lambda: gate_root_gauge_device(luks_desktop))
    luks.shutdown()

    if failures:
        raise Exception("desktop gates failed:\n  " + "\n  ".join(failures))
  '';
}
