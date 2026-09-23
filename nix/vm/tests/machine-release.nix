# The machine surfaces exactly as the NixOS module declares them
# (nix/modules/machine.nix), in one QEMU guest with the kernel's real VT, a
# QEMU xHCI controller, and the real OpenRGB server of vogix's OpenRGB build
# running headless under systemd.
#
# The machine owner is declared: `vogix`, whose real CLI publishes the
# palette. `other`, a second vogix user configured with desert and the first
# by name (the default owner without that declaration), must not reach the
# machine.
# The module renders machine.json from:
# - OpenRGB devices: `dram` (every "ENE DRAM" controller, Direct, base01),
#   `govee` ("Govee", Static, base0D), and keychron-k2-he from its hardware
#   module, which matches no controller here;
# - a command device `probe`: a recorder appending its colour argument to
#   /run/vogix-test/cmd, re-run on hidraw 0627:0001 (QEMU's usb-kbd).
# The server's controllers come from vogix.openrgb.settings: two Debug
# "ENE DRAM" sticks, a Debug keyboard with key and underglow matrices, a DDP
# device "ENE DRAM Wire" sending to 127.0.0.1:4048, and a Govee device at
# 127.0.0.1, which sends its commands to UDP 4003.
#
# Colours are checked against sources vogix does not compute: what
# OpenRGB's own DDP and Govee drivers send (recorded by socat), the kernel's
# /sys/module/vt/parameters/default_{red,grn,blu}, the theme packages'
# console/palette files, the vogix16 theme data, and console.colors as the
# module evaluated it.
#
# Gates:
# - boot with nothing published: the owner's tty1 autologin shell runs its
#   login profile and publishes nothing, vogix-machine is ready and waiting,
#   the VT keeps console.colors (built from the owner's configured theme),
#   vogix-openrgb waits for a palette, `vogix machine status` and
#   `validate` accept the state and the installed machine.json;
# - the owner's `vogix theme set` publishes after the state commit and every
#   surface follows: the VT palette with no VT switch, the command device,
#   every selected OpenRGB controller confirmed at protocol 6 with the
#   observers showing the colours; STATUS=, both status files and `vogix
#   machine status` agree;
# - a theme change and five rapid ones land as the last; identical bytes are
#   not republished; a user without state applies their configured theme on
#   their first refresh; another user's apply does not reach the machine;
# - a hot-added hidraw node of the probe's ids re-runs it; SIGHUP and
#   vogix-machine-resume.service make both owners re-apply;
# - `vogix machine inspect` lists the controllers and captures their raw
#   payloads at protocols 6 and 5 ($out/capture; with the published
#   palette, $out/published, the source of tests/fixtures);
# - a palette the drop zone's owner did not write is rejected by both
#   owners and `vogix machine status` exits 1 until the owner's refresh
#   replaces it;
# - the drop zone removed: both owners exit 75, both units restart them once
#   and the restarts, finding no zone, exit 78 and stay failed; re-creating
#   the zone recovers;
# - openrgb restarted: vogix-openrgb stops cleanly first and starts again
#   with the server, re-confirming; openrgb killed: the owner exits 75 and
#   both come back; the server never refused a connection;
# - a switch to maxProtocol 5 restarts both owners with the new
#   machine.json; vogix-openrgb speaks protocol 5 to the protocol 6 server;
# - after a reboot: vogix-machine is ready before systemd-user-sessions, so
#   before any login, with the published palette on the VT (that the owner
#   writes the VT palette before READY=1 is the unit test
#   machine::local::tests::startup_reports_the_console_reconcile_before_ready),
#   the LEDs and the command device follow it again, and the absent device is
#   warned about once, after OpenRGB's detection completes.
{ pkgs
, vogix16Themes
, home-manager
, self
,
}:

let
  inherit (pkgs) lib;

  testLib = import ./lib.nix {
    inherit pkgs home-manager self vogix16Themes;
  };

  recorder = pkgs.writeShellScript "vogix-test-recorder" ''
    printf '%s\n' "$1" >> /run/vogix-test/cmd
  '';

  # socat writes every datagram it receives, as one write, to the file.
  observer = port: {
    description = "Record the UDP datagrams OpenRGB sends to port ${toString port}";
    wantedBy = [ "multi-user.target" ];
    before = [ "openrgb.service" ];
    serviceConfig = {
      ExecStart = "${pkgs.socat}/bin/socat -u UDP4-RECV:${toString port},bind=127.0.0.1 OPEN:/run/observe-${toString port}/datagrams,creat,append";
      RuntimeDirectory = "observe-${toString port}";
    };
  };
in
pkgs.testers.nixosTest {
  name = "vogix-machine-release";

  nodes.machine = _: {
    imports = [ testLib.machineConfig ];

    virtualisation = {
      memorySize = 2048;
      qemu.options = [ "-device qemu-xhci,id=xhci" ];
    };

    users.users.other = {
      isNormalUser = true;
      home = "/home/other";
    };
    home-manager.users.other = {
      imports = [ ../home.nix ];
      home = {
        username = lib.mkForce "other";
        homeDirectory = lib.mkForce "/home/other";
      };
      programs.vogix.appearance.theme = lib.mkForce "desert";
    };

    vogix = {
      machine = {
        owner = "vogix";
        logLevel = "debug";
      };

      openrgb.settings = {
        DebugDevices.devices = [
          { type = "dram"; name = "ENE DRAM"; }
          { type = "dram"; name = "ENE DRAM"; }
          { type = "keyboard"; keyboard = true; underglow = true; }
        ];
        DDPDevices.devices = [
          { name = "ENE DRAM Wire"; ip = "127.0.0.1"; port = 4048; num_leds = 8; }
        ];
        GoveeDevices.devices = [{ ip = "127.0.0.1"; }];
      };

      hardware = {
        keychron-k2-he.enable = true;
        devices = {
          dram = {
            slot = "base01";
            provider.openrgb = { nameContains = "ENE DRAM"; mode = "Direct"; };
          };
          govee = {
            slot = "base0D";
            provider.openrgb = { nameContains = "Govee"; mode = "Static"; };
          };
          probe = {
            slot = "base01";
            provider.command = {
              argv = [ "${recorder}" "{{color}}" ];
              hotplug.hidraw = {
                vendorId = "0627";
                productId = "0001";
              };
            };
          };
        };
      };
    };

    environment.systemPackages = [ pkgs.jq pkgs.socat ];
    systemd = {
      tmpfiles.rules = [ "d /run/vogix-test 0755 root root -" ];
      services = {
        vogix-machine.serviceConfig.ReadWritePaths = [ "/run/vogix-test" ];
        observe-ddp = observer 4048;
        observe-govee = observer 4003;
      };
    };

    specialisation.protocol5.configuration.vogix.openrgb.client.maxProtocol = 5;
  };

  testScript = { nodes, ... }: ''
    import json

    all_themes = json.loads(r"""${testLib.themesJSON}""")
    build_console = ["#" + c.lower() for c in json.loads(r"""${builtins.toJSON nodes.machine.console.colors}""")]

    ZONE = "/var/lib/vogix/machine"
    PALETTE = f"{ZONE}/palette.json"
    LOCAL = "/run/vogix/machine/status.json"
    OPENRGB = "/run/vogix/openrgb/status.json"
    THEMES = "/home/vogix/.local/share/vogix/themes"


    def sysfs_palette():
        channels = [
            machine.succeed(f"cat /sys/module/vt/parameters/default_{c}").strip().split(",")
            for c in ("red", "grn", "blu")
        ]
        return ["#%02x%02x%02x" % (int(r), int(g), int(b)) for r, g, b in zip(*channels)]


    def theme_console(theme_variant):
        text = machine.succeed(f"cat {THEMES}/{theme_variant}/console/palette")
        return [line.strip().lower() for line in text.splitlines() if line.strip()]


    def published():
        return json.loads(machine.succeed(f"cat {PALETTE}"))


    def status(path):
        return json.loads(machine.succeed(f"cat {path}"))


    def recorded():
        return machine.succeed("cat /run/vogix-test/cmd 2>/dev/null || true").split()


    def journal(unit):
        return machine.succeed(f"journalctl -b -o cat -u {unit}")


    def count(text, needle):
        return sum(1 for line in text.splitlines() if needle in line)


    def invocation(unit):
        return machine.succeed(f"systemctl show -p InvocationID --value {unit}").strip()


    def colours(theme):
        variant = published()["theme"]["variant"]
        return variant, all_themes[theme][variant]


    def bare(colour):
        return colour.lstrip("#").lower()


    def ddp_shows(colour):
        # The last DDP datagram: a 10-byte header, then R,G,B for 8 LEDs.
        return (
            "test \"$(tail -c 24 /run/observe-4048/datagrams | od -An -tx1 | tr -d ' \\n')\""
            f" = {bare(colour) * 8}"
        )


    def govee_shows(colour):
        h = bare(colour)
        rgb = {"r": int(h[0:2], 16), "g": int(h[2:4], 16), "b": int(h[4:6], 16)}
        return (
            "tr '\\000' '\\n' < /run/observe-4003/datagrams | grep colorwc | tail -n 1"
            f" | jq -e '.msg.data.color == {json.dumps(rgb, sort_keys=True)}'"
        )


    def settled(theme, state="confirmed", protocol=6):
        # Every surface shows the theme: the OpenRGB controllers (status and
        # the observers), the VT palette and the command device.
        variant, c = colours(theme)
        machine.wait_until_succeeds(
            f"jq -e '.phase == \"ready\" and .protocol == {protocol}"
            f" and .theme.name == \"{theme}\""
            f" and .devices.dram.state == \"{state}\" and .devices.dram.controllers == 3"
            f" and .devices.govee.state == \"{state}\" and .devices.govee.controllers == 1"
            " and .devices.\"keychron-k2-he\".state == \"absent\"' " + OPENRGB
        )
        machine.wait_until_succeeds(ddp_shows(c["base01"]))
        machine.wait_until_succeeds(govee_shows(c["base0D"]))
        machine.wait_until_succeeds(
            f"jq -e '.phase == \"ready\" and .theme.name == \"{theme}\""
            " and .console.state == \"confirmed\" and .devices.probe.state == \"confirmed\"' " + LOCAL
        )
        assert recorded()[-1] == bare(c["base01"]), recorded()
        console = theme_console(f"{theme}-{variant}")
        assert sysfs_palette() == console, (sysfs_palette(), console)
        return variant, c


    def set_theme(user, *themes):
        machine.succeed(
            f"su - {user} -c '" + " && ".join(f"vogix theme set -t {t}" for t in themes) + "'"
        )


    machine.start()
    machine.wait_for_unit("multi-user.target")

    with subtest("with nothing published both owners wait and the VT keeps console.colors"):
        machine.wait_for_unit("vogix-machine.service")
        machine.wait_for_unit("openrgb.service")
        machine.wait_for_unit("vogix-openrgb.service")
        # The owner's autologin shell on tty1 has run its login profile once
        # its .bashrc banner is on screen; a login shell publishes nothing.
        machine.wait_until_tty_matches("1", "Vogix Test VM")
        machine.fail(f"test -e {PALETTE}")
        assert machine.succeed(f"stat -c '%U %G %a' {ZONE}").strip() == "vogix users 755"
        s = status(LOCAL)
        assert s["phase"] == "waiting-for-palette", s
        assert s["console"]["state"] == "waiting", s
        assert s["devices"]["probe"]["state"] == "waiting", s
        machine.wait_until_succeeds(f"jq -e '.phase == \"waiting-for-palette\" and .protocol == 6' {OPENRGB}")
        assert recorded() == [], recorded()
        # The kernel holds console.colors, the build-time palette from the
        # owner's configured theme; nordic, published next, differs from it.
        assert build_console == sysfs_palette(), (build_console, sysfs_palette())
        assert build_console != theme_console("nordic-night"), build_console
        # console.colors is the palette the owner's theme package ships.
        assert build_console == theme_console("yoga-night"), build_console
        active_vt = machine.succeed("cat /sys/class/tty/tty0/active").strip()
        out = machine.succeed("vogix machine status")
        print(out)
        assert "owner:    vogix (drop zone /var/lib/vogix/machine)" in out, out
        assert "palette:  none published yet" in out, out
        out = machine.succeed("vogix machine validate")
        print(out)
        assert "  owners:    vogix-openrgb.service, vogix-machine.service" in out, out
        assert "    keychron-k2-he: slot base0D, openrgb, name contains \"Keychron K2 HE\", mode \"Static\"" in out, out

    with subtest("the owner's theme set publishes after the commit, and every surface follows"):
        set_theme("vogix", "nordic")
        assert machine.succeed(f"stat -c '%U %a' {PALETTE}").strip() == "vogix 644"
        p = published()
        assert p["theme"]["name"] == "nordic", p
        state = machine.succeed("cat /home/vogix/.local/state/vogix/state.toml")
        assert 'current_theme = "nordic"' in state, state
        variant, nordic = settled("nordic")
        assert [c.lower() for c in p["console"]] == theme_console(f"nordic-{variant}"), p["console"]
        assert p["slots"]["base01"] == nordic["base01"].lower(), p["slots"]
        assert machine.succeed("cat /sys/class/tty/tty0/active").strip() == active_vt
        journal_text = journal("vogix-openrgb")
        assert "OpenRGB offers SDK protocol 6; speaking protocol 6" in journal_text, journal_text
        assert count(journal_text, "dram: ENE DRAM (id") >= 2 and count(journal_text, "dram: ENE DRAM Wire (id") >= 1, journal_text
        assert f"confirmed {nordic['base01'].lower()}" in journal_text, journal_text
        assert f"console: applied the vogix16 nordic {variant} palette" in journal("vogix-machine")
        machine.copy_from_machine(PALETTE, "published")

    with subtest("STATUS=, both status files and vogix machine status agree"):
        text = machine.succeed("systemctl show -p StatusText --value vogix-openrgb.service").strip()
        assert text == "protocol 6; 5 controllers; dram confirmed x3; govee confirmed; keychron-k2-he absent", text
        for path in (LOCAL, OPENRGB):
            assert machine.succeed(f"stat -c %a {path}").strip() == "644", path
        out = machine.succeed("vogix machine status")
        print(out)
        assert f"palette:  vogix16 nordic {variant}" in out, out
        assert "vogix-machine.service: ready" in out, out
        assert "  probe: confirmed" in out, out
        assert "vogix-openrgb.service: ready" in out, out
        assert "  dram: confirmed (3 controllers)" in out, out
        assert "  govee: confirmed (1 controller)" in out, out
        assert "  keychron-k2-he: absent" in out, out
        assert "problems:" not in out, out

    with subtest("a theme change and five rapid ones land as the last"):
        set_theme("vogix", "desert")
        settled("desert")
        set_theme("vogix", "matrix", "nordic", "desert", "matrix", "nordic")
        settled("nordic")

    with subtest("identical bytes are not republished"):
        inode = machine.succeed(f"stat -c %i {PALETTE}").strip()
        runs = len(recorded())
        machine.succeed("su - vogix -c 'vogix theme refresh'")
        assert machine.succeed(f"stat -c %i {PALETTE}").strip() == inode
        assert len(recorded()) == runs, recorded()

    with subtest("a user without state applies their configured theme first"):
        other_state = "/home/other/.local/state/vogix"
        # home-manager seeded current-theme at the configured theme.
        seeded = machine.succeed(f"readlink {other_state}/current-theme").strip()
        assert seeded.rsplit("/", 1)[-1].startswith("desert-"), seeded
        machine.fail(f"test -e {other_state}/state.toml")
        machine.succeed("su - other -c 'vogix theme refresh'")
        applied = machine.succeed(f"readlink {other_state}/current-theme").strip()
        assert applied == seeded, (seeded, applied)
        out = machine.succeed("su - other -c 'vogix theme status'")
        assert "scheme:  vogix16" in out and "theme:   desert" in out, out
        state = machine.succeed(f"cat {other_state}/state.toml")
        assert 'current_theme = "desert"' in state, state
        assert machine.succeed(f"stat -c %i {PALETTE}").strip() == inode

    with subtest("another user's apply does not reach the machine"):
        out = machine.succeed("su - other -c 'vogix theme set -t matrix 2>&1'")
        assert "Machine surfaces follow 'vogix'" in out, out
        assert machine.succeed(f"stat -c %i {PALETTE}").strip() == inode
        assert published()["theme"]["name"] == "nordic"
        settled("nordic")

    with subtest("a hidraw node of the probe's USB ids re-runs it"):
        runs = len(recorded())
        machine.send_monitor_command("device_add usb-kbd,id=vk,bus=xhci.0")
        machine.wait_until_succeeds(f"test $(wc -l < /run/vogix-test/cmd) -gt {runs}")
        assert recorded()[-1] == bare(nordic["base01"]), recorded()
        # QEMU's tablet carries the same ids and was present from boot; the
        # re-run names the keyboard's node.
        uevent = machine.succeed("grep -l 'HID_NAME=QEMU QEMU USB Keyboard' /sys/class/hidraw/*/device/uevent").strip()
        node = uevent.split("/")[4]
        assert f"probe: hidraw {node} (0627:0001) appeared; re-running" in journal("vogix-machine"), node

    def reapplied(action):
        confirms = count(journal("vogix-openrgb"), "confirmed #")
        hups = count(journal("vogix-openrgb"), "SIGHUP: re-reading the palette; forced re-apply of every device")
        forced = count(journal("vogix-machine"), "forced re-apply (SIGHUP)")
        current = count(journal("vogix-machine"), f"console: the vogix16 nordic {variant} palette is already current")
        runs = len(recorded())
        machine.succeed(action)
        machine.wait_until_succeeds(
            f"test $(journalctl -b -o cat -u vogix-openrgb | grep -c 'confirmed #') -ge {confirms + 4}"
        )
        machine.wait_until_succeeds(f"test $(wc -l < /run/vogix-test/cmd) -gt {runs}")
        assert count(journal("vogix-openrgb"), "SIGHUP: re-reading the palette; forced re-apply of every device") == hups + 1
        journal_text = journal("vogix-machine")
        assert count(journal_text, "forced re-apply (SIGHUP)") == forced + 1, journal_text
        assert count(journal_text, f"console: the vogix16 nordic {variant} palette is already current") == current + 1, journal_text
        settled("nordic")

    with subtest("SIGHUP makes both owners re-apply"):
        reapplied("systemctl reload vogix-openrgb.service vogix-machine.service")

    with subtest("vogix-machine-resume.service makes both owners re-apply"):
        reapplied("systemctl start vogix-machine-resume.service")

    with subtest("inspect lists the controllers read-only and captures their payloads"):
        text = machine.succeed("vogix machine inspect")
        print(text)
        assert "SDK protocol 6" in text, text
        listed = json.loads(machine.succeed("vogix machine inspect --json"))
        names = [c["displayName"] for c in listed["controllers"]]
        assert sorted(names) == sorted(["ENE DRAM", "ENE DRAM", "Debug Keyboard", "ENE DRAM Wire", "Govee "]), names
        for c in listed["controllers"]:
            if c["displayName"].startswith("ENE DRAM"):
                assert set(c["description"]["colors"]) == {nordic["base01"].lower()}, c["description"]["colors"]
        machine.succeed("vogix machine inspect --capture /tmp/capture/v6 > /dev/null")
        machine.succeed("vogix machine inspect --max-protocol 5 --capture /tmp/capture/v5 > /dev/null")
        for version in (6, 5):
            manifest = json.loads(machine.succeed(f"cat /tmp/capture/v{version}/manifest.json"))
            assert manifest["protocol"] == version, manifest
            assert len(manifest["controllers"]) == 5, manifest
        machine.copy_from_machine("/tmp/capture")
        settled("nordic")

    with subtest("a palette the drop zone's owner did not write is rejected, and status exits 1"):
        machine.succeed(
            f"cp {PALETTE} /tmp/p.json && install -m 0644 /tmp/p.json {ZONE}/.planted"
            f" && mv {ZONE}/.planted {PALETTE}"
        )
        machine.wait_until_succeeds(f"jq -e '.phase == \"waiting-for-palette\"' {LOCAL}")
        assert "is owned by uid 0" in status(LOCAL)["detail"], status(LOCAL)
        machine.wait_until_succeeds(f"jq -e '.detail | contains(\"is owned by uid 0\")' {OPENRGB}")
        rc, out = machine.execute("vogix machine status")
        print(out)
        assert rc == 1, (rc, out)
        assert "the published palette is rejected" in out, out
        machine.succeed("su - vogix -c 'vogix theme refresh'")
        assert machine.succeed(f"stat -c %U {PALETTE}").strip() == "vogix"
        settled("nordic")
        machine.succeed("vogix machine status")

    with subtest("a removed drop zone ends both owners with 75, and their restarts without it with 78"):
        owners = ("vogix-machine.service", "vogix-openrgb.service")
        machine.succeed("systemctl reset-failed " + " ".join(owners))

        def show(unit, prop):
            return machine.succeed(f"systemctl show -p {prop} --value {unit}").strip()

        before = {
            unit: (
                int(show(unit, "NRestarts")),
                count(journal(unit), "status=75/TEMPFAIL"),
                count(journal(unit), "status=78/CONFIG"),
            )
            for unit in owners
        }
        machine.succeed(f"rm -r {ZONE}")
        for unit in owners:
            machine.wait_until_succeeds(f"systemctl is-failed {unit}")
        for unit in owners:
            restarts, tempfail, config = before[unit]
            journal_text = journal(unit)
            # The zone's removal ended the owner with 75, its unit restarted
            # it once, and that start, with no zone, ended with 78.
            assert count(journal_text, "status=75/TEMPFAIL") == tempfail + 1, (unit, journal_text)
            assert count(journal_text, "status=78/CONFIG") == config + 1, (unit, journal_text)
            assert int(show(unit, "NRestarts")) == restarts + 1, unit
            assert show(unit, "ExecMainStatus") == "78", unit
            assert show(unit, "Result") == "exit-code", unit
            assert "was removed or moved" in journal_text, (unit, journal_text)
            assert f"cannot watch the drop zone {ZONE}" in journal_text, (unit, journal_text)
        # Re-creating the zone and starting the units recovers; the owner's
        # refresh publishes into it.
        machine.succeed("systemd-tmpfiles --create --prefix=/var/lib/vogix")
        assert machine.succeed(f"stat -c '%U %G %a' {ZONE}").strip() == "vogix users 755"
        machine.succeed("systemctl reset-failed " + " ".join(owners))
        machine.succeed("systemctl start " + " ".join(owners))
        machine.succeed("su - vogix -c 'vogix theme refresh'")
        settled("nordic")
        machine.succeed("vogix machine status")

    with subtest("restarting openrgb stops vogix-openrgb cleanly and starts a new one with it"):
        first = invocation("vogix-openrgb.service")
        machine.succeed("systemctl reset-failed vogix-openrgb.service")
        machine.succeed("systemctl restart openrgb.service")
        machine.wait_until_succeeds(
            f"test \"$(systemctl show -p InvocationID --value vogix-openrgb.service)\" != {first}"
            " && systemctl is-active vogix-openrgb.service"
        )
        settled("nordic")
        journal_text = machine.succeed("journalctl -b -u vogix-openrgb.service")
        assert "stopping: closing the OpenRGB connection once it drains" in journal_text, journal_text
        assert "vogix-openrgb.service: Deactivated successfully" in journal_text, journal_text
        assert "refused" not in journal_text, journal_text

    with subtest("a killed openrgb closes the connection: vogix-openrgb exits 75 and both come back"):
        second = invocation("vogix-openrgb.service")
        machine.succeed("systemctl reset-failed vogix-openrgb.service")
        machine.succeed("systemctl kill -s KILL openrgb.service")
        machine.wait_until_succeeds(
            f"test \"$(systemctl show -p InvocationID --value vogix-openrgb.service)\" != {second}"
            " && systemctl is-active vogix-openrgb.service"
        )
        settled("nordic")
        journal_text = machine.succeed("journalctl -b -u vogix-openrgb.service")
        assert "OpenRGB closed the connection" in journal_text, journal_text
        assert "status=75/TEMPFAIL" in journal_text, journal_text
        assert "refused" not in journal_text, journal_text

    with subtest("a switch to maxProtocol 5 restarts both owners with the new machine.json"):
        openrgb_before = invocation("vogix-openrgb.service")
        local_before = invocation("vogix-machine.service")
        machine.succeed("systemctl reset-failed vogix-openrgb.service")
        machine.succeed("/run/current-system/specialisation/protocol5/bin/switch-to-configuration test")
        machine.wait_until_succeeds(
            f"test \"$(systemctl show -p InvocationID --value vogix-openrgb.service)\" != {openrgb_before}"
            " && systemctl is-active vogix-openrgb.service"
        )
        assert invocation("vogix-machine.service") != local_before
        machine.succeed("vogix machine validate | grep -qxF '  openrgb:   127.0.0.1:6742, protocol up to 5, client name \"vogix\"'")
        settled("nordic", state="sent", protocol=5)
        journal_text = journal("vogix-openrgb")
        assert "OpenRGB offers SDK protocol 6; speaking protocol 5" in journal_text, journal_text
        assert f"sent {nordic['base01'].lower()}; protocol 5 cannot confirm it" in journal_text, journal_text
        set_theme("vogix", "desert")
        settled("desert", state="sent", protocol=5)

    with subtest("after a reboot the palette is on the VT before user sessions, and the devices follow it"):
        machine.shutdown()
        machine.start()
        machine.wait_for_unit("vogix-machine.service")
        machine.wait_for_unit("systemd-user-sessions.service")
        ready_at = int(machine.succeed(
            "systemctl show -p ActiveEnterTimestampMonotonic --value vogix-machine.service"
        ))
        sessions_at = int(machine.succeed(
            "systemctl show -p ActiveEnterTimestampMonotonic --value systemd-user-sessions.service"
        ))
        assert 0 < ready_at <= sessions_at, (ready_at, sessions_at)
        variant, desert = colours("desert")
        assert sysfs_palette() == theme_console(f"desert-{variant}"), sysfs_palette()
        assert machine.succeed("cat /sys/class/tty/tty0/active").strip() == active_vt
        machine.wait_for_unit("vogix-openrgb.service")
        settled("desert")
        # At protocol 6 absence is a warning once per DETECTION_COMPLETE,
        # after it; the presence transitions before it are debug lines.
        warning = "[WARN ] keychron-k2-he: no controller matches \"Keychron K2 HE\""
        machine.wait_until_succeeds(
            f"journalctl -b -o cat -u vogix-openrgb.service | grep -qF '{warning}'"
        )
        lines = journal("vogix-openrgb").splitlines()
        done = [i for i, l in enumerate(lines) if "OpenRGB detection complete" in l]
        warned = [i for i, l in enumerate(lines) if l.startswith(warning)]
        assert len(done) == 1 and len(warned) == 1 and warned[0] > done[0], (done, warned)
        present = lines[warned[0]].split("(present: ", 1)[1].rstrip(")").split(", ")
        assert sorted(present) == sorted(["ENE DRAM Wire", "ENE DRAM", "ENE DRAM", "Debug Keyboard", "Govee "]), present
  '';
}
